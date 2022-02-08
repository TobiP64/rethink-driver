// MIT License
//
// Copyright (c) 2020-2021 Tobias Pfeiffer
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//unwrap()
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use {
	super::*,
	std::{
		cell::UnsafeCell,
		collections::LinkedList,
		pin::Pin,
		task::{Poll, Waker}
	}
};

#[derive(Debug)]
enum QueryState {
	NoReplyRequestActive(Vec<u8>),
	NoReplyRequestWake(Vec<u8>, Waker),
	RequestActive(Vec<u8>),
	RequestWake(Vec<u8>, Waker),
	PendingActive,
	PendingWake(Waker),
	ResultNoReply,
	ResultOk(Vec<u8>),
	ResultErr(Error)
}

#[derive(Debug)]
struct SchedulerQueue {
	wake:  Option<Waker>,
	queue: LinkedList<(QueryToken, Arc<__UnsafeCell__<QueryState>>)>
}

#[derive(Debug)]
pub struct Scheduler {
	stream: __UnsafeCell__<wire::AsyncStream>,
	queue:  smol::lock::Mutex<SchedulerQueue>
}

impl Scheduler {
	pub fn new(stream: wire::AsyncStream) -> Self {
		Self {
			queue:   smol::lock::Mutex::new(SchedulerQueue {
				wake:  None,
				queue: LinkedList::new()
			}),
			stream: __UnsafeCell__::new(stream),
		}
	}
	
	pub async fn set_stream(&self, stream: wire::AsyncStream) {
		// spin lock until the queue is empty, so it is safe to replace the stream
		loop {
			let queue = self.queue.lock().await;
			
			if queue.queue.is_empty() {
				unsafe { *self.stream.get() = stream; }
				return;
			}
		}
	}
	
	pub async fn dispatch(&self, token: QueryToken, mut buf: Vec<u8>, reply: bool) -> Result<Option<Vec<u8>>> {
		use QueryState::*;
		
		let mut queue = self.queue.lock().await;
		
		let result = if queue.queue.is_empty() {
			queue.queue.push_back((token, Arc::new(__UnsafeCell__::new(match reply {
				true  => RequestActive(buf),
				false => NoReplyRequestActive(buf)
			}))));
			
			self.run(queue).await
		} else {
			let state = smol::future::poll_fn(|cx| Poll::Ready(match reply {
				true  => RequestWake(std::mem::take(&mut buf), cx.waker().clone()),
				false => NoReplyRequestWake(std::mem::take(&mut buf), cx.waker().clone())
			})).await;
			
			let state = Arc::new(__UnsafeCell__::new(state));
			queue.queue.push_back((token, state.clone()));
			
			if let Some(waker) = queue.wake.take() {
				waker.wake();
			}
			
			std::mem::drop(queue);
			let mut ready = false;
			smol::future::poll_fn(|_| if ready {
				Poll::Ready(())
			} else {
				ready = true;
				Poll::Pending
			}).await;
			
			loop {
				match unsafe { __UnsafeCell_get_deref__(&*state) } {
					NoReplyRequestActive(_) | RequestActive(_) | PendingActive => break self.run(self.queue.lock().await).await,
					NoReplyRequestWake(..) | PendingWake(..) | RequestWake(..) => {
						ready = false;
						smol::future::poll_fn(|_| if ready {
							Poll::Ready(())
						} else {
							ready = true;
							Poll::Pending
						}).await
					}
					_ => break Arc::try_unwrap(state)
						.map_err(|_| Error::Sync("failed to lock unwrap state"))?
						.into_inner()
				}
			}
		};
		
		match result {
			ResultNoReply => Ok(None),
			ResultOk(buf) => Ok(Some(buf)),
			ResultErr(e)  => Err(e),
			_             => Err(Error::Sync("invalid query state after wakeup"))
		}
	}
	
	async fn run<'a>(&'a self, mut queue: smol::lock::MutexGuard<'a, SchedulerQueue>) -> QueryState {
		use QueryState::*;
		
		enum StreamResult {
			Send,
			Recv(Result<(QueryToken, Vec<u8>)>)
		}
		
		debug_assert!(!queue.queue.is_empty(), "cannot acquire stream when queue is empty");
		let stream = unsafe { __UnsafeCell_get_deref_mut__(&self.stream) };
		let mut result = StreamResult::Send;
		
		let (queue, state_) = 'main: loop {
			
			match result {
				StreamResult::Recv(Ok((recv_token, buf))) => {
					let mut cursor = queue.queue.cursor_front_mut();
					
					loop {
						match cursor.current() {
							None => break,
							Some((token, _)) if *token == recv_token => break,
							Some(_) => cursor.move_next()
						}
					}
					
					match cursor.remove_current() {
						Some((_, state)) => match unsafe { &mut*state.get() } {
							PendingWake(waker) => {
								let waker = unsafe { std::ptr::read(waker) };
								unsafe { std::ptr::write(state.get(), ResultOk(buf)) }
								waker.wake();
							}
							PendingActive => break (queue, ResultOk(buf)),
							_ => log::warn!("query #{}: received response for already completed query", recv_token)
						}
						None => log::warn!("query #{}: received response for unregistered query", recv_token)
					}
				}
				StreamResult::Recv(Err(e)) => break (queue, ResultErr(e)),
				StreamResult::Send => {
					let mut cursor = queue.queue.cursor_front_mut();
					
					while let Some((token, state)) = cursor.current() {
						let token = *token;
						
						match unsafe { __UnsafeCell_get_deref_mut__(&*state) } {
							RequestActive(buf) => match stream.send(token, buf).await {
								Ok(()) => {
									std::mem::drop(unsafe { std::ptr::read(buf) });
									unsafe { std::ptr::write(state.get(), PendingActive) }
								}
								Err(e) => {
									cursor.remove_current();
									break 'main (queue, ResultErr(e))
								}
							}
							NoReplyRequestActive(buf) => {
								let new_state = match stream.send(token, buf).await {
									Ok(()) => ResultNoReply,
									Err(e) => ResultErr(e)
								};
								
								cursor.remove_current();
								break 'main (queue, new_state);
							}
							RequestWake(buf, waker) => match stream.send(token, buf).await {
								Ok(()) => {
									let waker = unsafe { std::ptr::read(waker) };
									unsafe { std::ptr::write(state.get(), PendingWake(waker)) }
								}
								Err(e) => {
									let waker = unsafe { std::ptr::read(waker) };
									unsafe { std::ptr::write(state.get(), ResultErr(e)) }
									cursor.remove_current();
									waker.wake();
								}
							}
							NoReplyRequestWake(buf, waker) => {
								let new_state = match stream.send(token, buf).await {
									Ok(()) => ResultNoReply,
									Err(e) => ResultErr(e)
								};
								
								let waker = unsafe { std::ptr::read(waker) };
								unsafe { std::ptr::write(state.get(), new_state) }
								cursor.remove_current();
								waker.wake();
							}
							_ => ()
						}
						
						cursor.move_next();
					}
				}
			}
			
			let mut future_state = Some(queue);
			
			result = smol::future::poll_fn(|cx| {
				if let Some(mut queue) = future_state.take() {
					// first call, set waker and return
					queue.wake = Some(cx.waker().clone());
					Pin::new(&mut*stream).poll_recv(cx).map(StreamResult::Recv)
				} else {
					// call after wake up, check if it was caused by the stream or a newly created
					// query
					match Pin::new(&mut*stream).poll_recv(cx) {
						Poll::Ready(v) => Poll::Ready(StreamResult::Recv(v)),
						Poll::Pending  => Poll::Ready(StreamResult::Send)
					}
				}
			}).await;
			
			queue = self.queue.lock().await;
		};
		
		for (_, state) in queue.queue.iter() {
			let state = unsafe { &mut*state.get() };
			
			match state {
				NoReplyRequestWake(buf, waker) => {
					let buf = unsafe { std::ptr::read(buf) };
					let waker = unsafe { std::ptr::read(waker) };
					unsafe { std::ptr::write(state, NoReplyRequestActive(buf)) };
					waker.wake();
				}
				RequestWake(buf, waker) => {
					let buf = unsafe { std::ptr::read(buf) };
					let waker = unsafe { std::ptr::read(waker) };
					unsafe { std::ptr::write(state, RequestActive(buf)) };
					waker.wake();
				}
				PendingWake(waker) => {
					let waker = unsafe { std::ptr::read(waker) };
					unsafe { std::ptr::write(state, PendingActive) };
					waker.wake();
				}
				_ => continue
			}
			
			return state_;
		}
		
		state_
	}
}

#[derive(Debug)]
struct __UnsafeCell__<T>(UnsafeCell<T>);

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<T> Send for __UnsafeCell__<T> {}
unsafe impl<T> Sync for __UnsafeCell__<T> {}

impl<T> __UnsafeCell__<T> {
	fn new(v: T) -> Self {
		Self(UnsafeCell::new(v))
	}
	
	fn into_inner(self) -> T {
		self.0.into_inner()
	}
}

impl<T> std::ops::Deref for __UnsafeCell__<T> {
	type Target = UnsafeCell<T>;
	
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> std::ops::DerefMut for __UnsafeCell__<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[allow(non_snake_case)]
unsafe fn __UnsafeCell_get_deref__<'a, T>(v: &__UnsafeCell__<T>) -> &'a T {
	&*v.get()
}

#[allow(non_snake_case)]
unsafe fn __UnsafeCell_get_deref_mut__<'a, T>(v: &__UnsafeCell__<T>) -> &'a mut T {
	&mut*v.get()
}