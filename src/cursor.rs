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

#[derive(Clone, Debug)]
pub struct Cursor<T: DeserializeOwned> {
	pub conn:   RDbConnection,
	pub token:  QueryToken,
	pub buffer: VecDeque<DocResult<T>>
}

impl<T: DeserializeOwned> Cursor<T> {
	pub(crate) fn from_response(conn: &RDbConnection, token: QueryToken, response: Response<T>) -> Self {
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
#[derive(Clone, Debug)]
pub struct AsyncCursor<T: DeserializeOwned> {
	pub conn: RDbAsyncConnection,
	pub token:  QueryToken,
	pub buffer: VecDeque<DocResult<T>>
}

#[cfg(feature = "async")]
impl<T: DeserializeOwned> AsyncCursor<T> {
	pub(crate) fn from_response(conn: &RDbAsyncConnection, token: QueryToken, response: Response<T>) -> Self {
		Self {
			conn:  conn.clone(),
			token: match response.r#type {
				ResponseType::SuccessPartial => token,
				_ => !0
			},
			buffer: VecDeque::from(response.result)
		}
	}
	
	pub async fn next(&mut self) -> Option<Result<T>> {
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
			}).await {
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