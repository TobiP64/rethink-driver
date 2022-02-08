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

#![allow(clippy::large_enum_variant)]

use {
	std::{io, net::{TcpStream, SocketAddr, ToSocketAddrs}},
	crate::{auth, ConnectionOptions, Result, QueryToken},
};

#[cfg(feature = "async")]
use std::{pin::Pin, task::{Context, Poll}};

pub const VERSION_1_0: u32 = 0x34c2bdc3;

trait PrimitiveStream: io::Read + io::Write {}

impl<T: io::Read + io::Write> PrimitiveStream for T {}

pub struct Stream(Box<dyn PrimitiveStream>);

impl Stream {
	#[cfg(not(feature = "tls"))]
	pub fn connect(options: &ConnectionOptions) -> Result<Self> {
		let mut stream = TcpStream::connect_timeout(
			&resolve(options.hostname.as_ref(), options.port)?, options.timeout)?;
		stream.set_read_timeout(Some(options.timeout))?;
		stream.set_write_timeout(Some(options.timeout))?;
		auth::handshake(&mut stream, options)?;
		Ok(Self(stream))
	}
	
	#[cfg(feature = "tls")]
	pub fn connect(options: &ConnectionOptions) -> Result<Self> {
		let stream = TcpStream::connect_timeout(
			&resolve(options.hostname.as_ref(), options.port)?, options.timeout)?;
		stream.set_read_timeout(Some(options.timeout))?;
		stream.set_write_timeout(Some(options.timeout))?;
		
		let mut stream: Box<dyn PrimitiveStream> = match &options.tls {
			None => Box::new(stream),
			Some(cfg) => {
				let hostname = webpki::DNSNameRef::try_from_ascii_str(options.hostname.as_ref())
					.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
				Box::new(rustls::StreamOwned::new(rustls::ClientSession::new(cfg, hostname), stream))
			}
		};
		
		auth::handshake(&mut stream, options)?;
		Ok(Self(stream))
	}
	
	pub fn send(&mut self, token: QueryToken, query: &[u8]) -> Result<()> {
		self.0.write_all(&buf_from_token_len(token, query.len()))?;
		self.0.write_all(query)?;
		Ok(())
	}
	
	pub fn recv(&mut self) -> Result<(QueryToken, Vec<u8>)> {
		let mut buf = [0u8; 12];
		self.0.read_exact(&mut buf)?;
		
		let (token, len) = buf_to_token_len(&buf);
		let mut buf = vec![0u8; len];
		self.0.read_exact(&mut buf)?;
		
		Ok((token, buf))
	}
}

impl std::fmt::Debug for Stream {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Stream").finish_non_exhaustive()
	}
}

#[cfg(feature = "async")]
trait AsyncPrimitiveStream: smol::io::AsyncRead + smol::io::AsyncWrite + Send {}

#[cfg(feature = "async")]
impl<T: smol::io::AsyncRead + smol::io::AsyncWrite + Send> AsyncPrimitiveStream for T {}

#[cfg(feature = "async")]
#[derive(Clone, Debug)]
enum ReadState {
	Ready,
	TokenLen(Vec<u8>, usize),
	Payload(Vec<u8>, QueryToken, usize)
}

#[cfg(feature = "async")]
#[derive(Clone, Debug)]
enum WriteState {
	Ready,
	TokenLen([u8; 12], usize),
	WritePayload(usize)
}

#[cfg(feature = "async")]
pub struct AsyncStream {
	inner: Pin<Box<dyn AsyncPrimitiveStream>>,
	read:  ReadState,
	write: WriteState
}

#[cfg(feature = "async")]
impl AsyncStream {
	#[cfg(not(feature = "tls"))]
	pub async fn connect(options: &ConnectionOptions) -> Result<Self> {
		let mut stream = smol::net::TcpStream::connect(
			resolve(options.hostname.as_ref(), options.port)?).await?;
		auth::handshake_async(&mut stream, options).await?;
		
		Ok(Self {
			inner: stream,
			read:  ReadState::Ready,
			write: WriteState::Ready
		})
	}
	
	#[cfg(feature = "tls")]
	pub async fn connect(options: &ConnectionOptions) -> Result<Self> {
		let stream = smol::net::TcpStream::connect(
			resolve(options.hostname.as_ref(), options.port)?).await?;
		
		let mut stream: Pin<Box<dyn AsyncPrimitiveStream>> = match &options.tls {
			None      => Box::pin(stream),
			Some(cfg) => {
				let hostname = webpki::DNSNameRef::try_from_ascii_str(options.hostname.as_ref())
					.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
				Box::pin(async_rustls::TlsConnector::from(cfg.clone())
					.connect(hostname, stream).await?)
			}
		};
		
		auth::handshake_async(&mut stream, options).await?;
		
		Ok(Self {
			inner: stream,
			read:  ReadState::Ready,
			write: WriteState::Ready
		})
	}
	
	pub async fn send(&mut self, token: QueryToken, query: &[u8]) -> Result<()> {
		smol::future::poll_fn(move |cx| Pin::new(&mut*self).poll_send(cx, token, query)).await
	}
	
	pub async fn recv(&mut self) -> Result<(QueryToken, Vec<u8>)> {
		smol::future::poll_fn(move |cx| Pin::new(&mut*self).poll_recv(cx)).await
	}
	
	pub fn poll_send(self: Pin<&mut Self>, cx: &mut Context<'_>, token: QueryToken, query: &[u8]) -> Poll<Result<()>> {
		let Self { inner, write, .. } = unsafe { Pin::into_inner_unchecked(self) };
		
		loop {
			match write {
				WriteState::Ready => {
					let buf = buf_from_token_len(token, query.len());
					*write = WriteState::TokenLen(buf, 12);
				}
				WriteState::TokenLen(buf, rem) if *rem > 0 => {
					match inner.as_mut().poll_write(cx, &buf[buf.len() - *rem..]) {
						Poll::Pending => return Poll::Pending,
						Poll::Ready(Ok(written)) => *rem -= written,
						Poll::Ready(Err(e)) => return Poll::Ready(Err(crate::Error::Io(e)))
					}
				}
				WriteState::TokenLen(_, _) => *write = WriteState::WritePayload(query.len()),
				WriteState::WritePayload(rem) if *rem > 0 => {
					match inner.as_mut().poll_write(cx, &query[query.len() - *rem..]) {
						Poll::Pending => return Poll::Pending,
						Poll::Ready(Ok(written)) => *rem -= written,
						Poll::Ready(Err(e)) => return Poll::Ready(Err(crate::Error::Io(e)))
					}
				}
				WriteState::WritePayload(_) => {
					*write = WriteState::Ready;
					return Poll::Ready(Ok(()));
				}
			}
		}
	}
	
	#[allow(clippy::uninit_vec)]
	pub fn poll_recv(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(QueryToken, Vec<u8>)>> {
		let Self { inner, read, .. } = unsafe { Pin::into_inner_unchecked(self) };
		
		loop {
			match read {
				ReadState::Ready => {
					let mut buf = Vec::with_capacity(12);
					unsafe { buf.set_len(12) }; // SAFE: capacity is 12
					*read = ReadState::TokenLen(buf, 12);
				}
				ReadState::TokenLen(buf, rem) | ReadState::Payload(buf, _, rem) if *rem > 0 => {
					let __buf_len__ = buf.len();
					match inner.as_mut().poll_read(cx, &mut buf[__buf_len__ - *rem..]) {
						Poll::Pending         => return Poll::Pending,
						Poll::Ready(Ok(read)) => *rem -= read,
						Poll::Ready(Err(e))   => return Poll::Ready(Err(crate::Error::Io(e)))
					}
				}
				ReadState::TokenLen(buf, _) => {
					let (token, len) = buf_to_token_len(buf);
					
					if len > 12 {
						buf.reserve(len - 12);
					}
					
					unsafe { buf.set_len(len) }; // SAFE capacity is at least `len`
					let buf = std::mem::take(buf);
					*read = ReadState::Payload(buf, token, len);
				}
				ReadState::Payload(buf, token, _) => {
					let buf = std::mem::take(buf);
					let token = *token;
					*read = ReadState::Ready;
					return Poll::Ready(Ok((token, buf)));
				}
			}
		}
	}
}

#[cfg(feature = "async")]
impl std::fmt::Debug for AsyncStream {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AsyncStream").finish_non_exhaustive()
	}
}

fn resolve(host_name: &str, port: u16) -> io::Result<SocketAddr> {
	(host_name, port).to_socket_addrs()?.next().ok_or_else(
		|| io::Error::new(io::ErrorKind::Other, "failed to resolve host name"))
}

fn buf_to_token_len(buf: &[u8]) -> (QueryToken, usize) {
	(
		u64::from_le_bytes([buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7]]),
		u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as _
	)
}

fn buf_from_token_len(token: QueryToken, len: usize) -> [u8; 12] {
	let mut buf = [0u8; 12];
	buf[..8].copy_from_slice(&token.to_le_bytes());
	buf[8..].copy_from_slice(&(len as u32).to_le_bytes());
	buf
}