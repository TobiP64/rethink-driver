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
	crate::{auth, ConnectionOptions, Result, QueryToken, DEFAULT_TIMEOUT},
};

pub const VERSION_1_0: u32 = 0x34c2bdc3;

pub enum Stream {
	Tcp(TcpStream),
	#[cfg(feature = "tls")]
	Tls(rustls::StreamOwned<rustls::ClientSession, TcpStream>)
}

impl Stream {
	#[cfg(not(feature = "tls"))]
	pub fn connect(host_name: &str, port: u16, options: &ConnectionOptions) -> Result<Self> {
		let mut stream = TcpStream::connect_timeout(&resolve(host_name, port)?, options.timeout.unwrap_or(DEFAULT_TIMEOUT))?;
		auth::handshake(&mut stream, options)?;
		Ok(Self::Tcp(stream))
	}
	
	#[cfg(feature = "tls")]
	pub fn connect(host_name: &str, port: u16, options: &ConnectionOptions) -> Result<Self> {
		let mut stream = TcpStream::connect_timeout(&resolve(host_name, port)?, options.timeout.unwrap_or(DEFAULT_TIMEOUT))?;
		Ok(match &options.tls {
			None      => {
				auth::handshake(&mut stream, options)?;
				Self::Tcp(stream)
			},
			Some(cfg) => {
				let mut stream = rustls::StreamOwned::new(rustls::ClientSession::new(
					cfg, webpki::DNSNameRef::try_from_ascii_str(host_name)
						.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?), stream);
				auth::handshake(&mut stream, options)?;
				Self::Tls(stream)
			}
		})
	}
	
	pub fn send(&mut self, token: QueryToken, query: &[u8]) -> Result<()> {
		fn send_inner(stream: &mut impl io::Write, token: QueryToken, query: &[u8]) -> Result<()> {
			stream.write_all(&buf_from_token_len(token, query.len()))?;
			stream.write_all(query)?;
			log::trace!("sent #{}: `{}`", token, std::str::from_utf8(query).unwrap_or("<invalid UTF-8>"));
			Ok(())
		}
		
		match self {
			Self::Tcp(stream) => send_inner(stream, token, query),
			Self::Tls(stream) => send_inner(stream, token, query)
		}
	}
	
	pub fn recv(&mut self) -> Result<(QueryToken, Vec<u8>)> {
		fn recv_inner(stream: &mut impl io::Read) -> Result<(QueryToken, Vec<u8>)> {
			let mut buf = [0u8; 12];
			stream.read_exact(&mut buf)?;
			let (token, len) = buf_to_token_len(buf);
			let mut buf = vec![0u8; len];
			stream.read_exact(&mut buf)?;
			log::trace!("received #{}: `{}`", token, std::str::from_utf8(&buf).unwrap_or("<invalid UTF-8>"));
			Ok((token, buf))
		}
		
		match self {
			Self::Tcp(stream) => recv_inner(stream),
			Self::Tls(stream) => recv_inner(stream)
		}
	}
}

impl std::fmt::Debug for Stream {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple(match self {
			Self::Tcp(_) => "Stream::Tcp",
			#[cfg(feature = "tls")]
			Self::Tls(_) => "Stream::Tls"
		}).finish()
	}
}

#[cfg(feature = "async")]
pub enum AsyncStream {
	Tcp(async_io::Async<TcpStream>),
	#[cfg(feature = "tls")]
	Tls(async_tls::client::TlsStream<async_io::Async<TcpStream>>)
}

#[cfg(feature = "async")]
impl AsyncStream {
	#[cfg(not(feature = "tls"))]
	pub async fn connect(host_name: &str, port: u16, options: &ConnectionOptions) -> Result<Self> {
		let mut stream = Self::AsyncTcp(async_std::net::TcpStream::connect((host_name, port)).await?);
		auth::handshake_async(&mut stream, options).await?;
		Ok(Self::Tcp(stream))
	}
	
	#[cfg(feature = "tls")]
	pub async fn connect(host_name: &str, port: u16, options: &ConnectionOptions) -> Result<Self> {
		let mut stream = async_io::Async::<TcpStream>::connect(resolve(host_name, port)?).await?;
		Ok(match &options.tls {
			None      => {
				auth::handshake_async(&mut stream, options).await?;
				Self::Tcp(stream)
			},
			Some(cfg) => {
				let mut stream = async_tls::TlsConnector::from((&**cfg).clone())
					.connect(host_name, stream).await?;
				auth::handshake_async(&mut stream, options).await?;
				Self::Tls(stream)
			}
		})
	}
	
	pub async fn send(&mut self, token: QueryToken, query: &[u8]) -> Result<()> {
		async fn send_inner(stream: &mut (impl futures_lite::AsyncWrite + std::marker::Unpin), token: QueryToken, query: &[u8]) -> Result<()> {
			use futures_lite::io::AsyncWriteExt;
			stream.write_all(&buf_from_token_len(token, query.len())).await?;
			stream.write_all(query).await?;
			log::trace!("sent #{}: `{}`", token, std::str::from_utf8(&query).unwrap_or("<invalid UTF-8>"));
			Ok(())
		}
		
		match self {
			Self::Tcp(stream) => send_inner(stream, token, query).await,
			Self::Tls(stream) => send_inner(stream, token, query).await
		}
	}
	
	pub async fn recv(&mut self) -> Result<(QueryToken, Vec<u8>)> {
		async fn recv_inner(stream: &mut (impl futures_lite::AsyncRead + std::marker::Unpin)) -> Result<(QueryToken, Vec<u8>)> {
			use futures_lite::io::AsyncReadExt;
			let mut buf = [0u8; 12];
			stream.read_exact(&mut buf).await?;
			let (token, len) = buf_to_token_len(buf);
			let mut buf = vec![0u8; len];
			stream.read_exact(&mut buf).await?;
			log::trace!("received #{}: `{}`", token, std::str::from_utf8(&buf).unwrap_or("<invalid UTF-8>"));
			Ok((token, buf))
		}
		
		match self {
			Self::Tcp(stream) => recv_inner(stream).await,
			Self::Tls(stream) => recv_inner(stream).await
		}
	}
}

#[cfg(feature = "async")]
impl std::fmt::Debug for AsyncStream {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple(match self {
			Self::Tcp(_) => "AsyncStream::Tcp",
			#[cfg(feature = "tls")]
			Self::Tls(_) => "AsyncStream::Tls"
		}).finish()
	}
}

fn resolve(host_name: &str, port: u16) -> io::Result<SocketAddr> {
	(host_name, port).to_socket_addrs()?.next().ok_or_else(
		|| io::Error::new(io::ErrorKind::Other, "failed to resolve host name"))
}

fn buf_to_token_len(buf: [u8; 12]) -> (QueryToken, usize) {
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