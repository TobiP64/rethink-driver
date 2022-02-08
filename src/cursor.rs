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
//
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

use {crate::*, std::collections::VecDeque, serde::de::DeserializeOwned};

#[cfg(feature = "async")]
use std::{pin::Pin, task::{Poll, Context}};

pub struct Cursor<T: DeserializeOwned> {
	pub conn: Connection,
	pub token:  QueryToken,
	pub buffer: VecDeque<DocResult<T>>
}

impl<T: DeserializeOwned> Cursor<T> {
	pub(crate) fn from_response(conn: &Connection, token: QueryToken, response: Response<T>) -> Self {
		Self {
			conn: conn.clone(),
			token: match response.r#type {
				ResponseType::SuccessPartial => token,
				_ => !0
			},
			buffer: VecDeque::from(response.result)
		}
	}
}

impl<T: DeserializeOwned> Iterator for Cursor<T> {
	type Item = Result<T>;
	
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if let Some(doc) = self.buffer.pop_front() {
				return Some(doc.into_result())
			} else if self.token == !0 {
				return None;
			}
			
			let response = match self.conn.send_recv::<_, T>(self.token, Query::<()> {
				r#type:  QueryType::Continue,
				term:    None,
				options: None
			}) {
				Err(e) => return Some(Err(e)),
				Ok(v) => v
			};
			
			self.buffer.extend(response.result);
			
			if response.r#type != ResponseType::SuccessPartial {
				self.token = !0;
			}
		}
	}
}

impl<T: DeserializeOwned> Drop for Cursor<T> {
	fn drop(&mut self) {
		if self.token != !0 {
			std::mem::drop(self.conn.send_recv::<_, ()>(self.token, Query::<()> {
				r#type:  QueryType::Stop,
				term:    None,
				options: None
			}));
		}
	}
}

#[cfg(feature = "async")]
pub struct AsyncCursor<T: DeserializeOwned + Send + Sync> {
	pub conn: AsyncConnection,
	pub token:  QueryToken,
	pub buffer: VecDeque<DocResult<T>>,
	pub state:  Option<smol::future::Boxed<Option<Result<T>>>>
}

#[cfg(feature = "async")]
impl<T: 'static + DeserializeOwned + Send + Sync> AsyncCursor<T> {
	pub(crate) fn from_response(conn: &AsyncConnection, token: QueryToken, response: Response<T>) -> Self {
		Self {
			conn:  conn.clone(),
			token: match response.r#type {
				ResponseType::SuccessPartial => token,
				_ => !0
			},
			buffer: VecDeque::from(response.result),
			state:  None
		}
	}
}

#[cfg(feature = "async")]
unsafe impl<T: 'static + DeserializeOwned + Send + Sync> Send for AsyncCursor<T> {}
#[cfg(feature = "async")]
unsafe impl<T: 'static + DeserializeOwned + Send + Sync> Sync for AsyncCursor<T> {}

#[cfg(feature = "async")]
impl<T: 'static + DeserializeOwned + Send + Sync> smol::stream::Stream for AsyncCursor<T> {
	type Item = Result<T>;
	
	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		use smol::future::FutureExt;
		
		let self_ = unsafe { Pin::into_inner_unchecked(self) };
		let self_ = unsafe { &mut*(self_ as *mut Self) };
		
		let r = self_.state.get_or_insert_with(|| Box::pin(async {
			loop {
				if let Some(doc) = self_.buffer.pop_front() {
					return Some(doc.into_result())
				} else if self_.token == !0 {
					return None;
				}
				
				let response = match self_.conn.send_recv::<_, T>(self_.token, Query::<()> {
					r#type:  QueryType::Continue,
					term:    None,
					options: None
				}).await {
					Err(e) => return Some(Err(e)),
					Ok(v) => v
				};
				
				self_.buffer.extend(response.result);
				
				if response.r#type != ResponseType::SuccessPartial {
					self_.token = !0;
				}
			}
		})).poll(cx);
		
		if r.is_ready() {
			self_.state.take();
		}
		
		r
	}
}