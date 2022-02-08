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

//#![forbid(unsafe_code)]
#![feature(linked_list_cursors)]
#![warn(clippy::all)]

use {
	std::{time::Duration, borrow::Cow, sync::{Arc, Mutex, RwLock, atomic::{AtomicU64, Ordering}}},
	serde::{Serialize, de::DeserializeOwned}
};

pub use self::{query::*, cursor::*, auth::AuthError};

#[cfg(feature = "tls")]
pub use rustls::ClientConfig as TlsConfig;

pub mod wire;
pub mod reql;
pub mod query;
pub mod cursor;
pub mod auth;
#[cfg(feature = "async")]
pub mod scheduler;

pub type QueryToken = u64;
pub type Result<T> = std::result::Result<T, Error>;

pub const DEFAULT_HOST:    &str     = "localhost";
pub const DEFAULT_PORT:    u16      = 28015;
pub const DEFAULT_DB:      &str     = "test";
pub const DEFAULT_USER:    &str     = "admin";
pub const DEFAULT_PWD:     &str     = "";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(20);

#[derive(Clone)]
pub struct ConnectionOptions {
	/// the host to connect to (default `localhost`)
	pub hostname: Cow<'static, str>,
	/// the port to connect on (default `28015`)
	pub port:     u16,
	/// the default database (default `test`)
	pub db:       Cow<'static, str>,
	/// the user account to connect as (default `"admin"`)
	pub user:     Cow<'static, str>,
	/// the password for the user account to connect as (default `""`)
	pub password: Cow<'static, str>,
	/// timeout period for the connection to be opened (default `20`)
	pub timeout:  Duration,
	/// a hash of options to support TLS/SSL connections (default `None`)
	#[cfg(feature = "tls")]
	pub tls:      Option<Arc<TlsConfig>>
}

impl ConnectionOptions {
	pub fn new() -> Self {
		Self::default()
	}
	
	pub fn connect(self) -> Result<Connection> {
		Connection::connect(self)
	}
	
	#[cfg(feature = "async")]
	pub async fn connect_async(self) -> Result<AsyncConnection> {
		AsyncConnection::connect(self).await
	}
}

impl Default for ConnectionOptions {
	fn default() -> Self {
		Self {
			hostname: Cow::Borrowed(DEFAULT_HOST),
			port:     DEFAULT_PORT,
			db:       Cow::Borrowed(DEFAULT_DB),
			user:     Cow::Borrowed(DEFAULT_USER),
			password: Cow::Borrowed(DEFAULT_PWD),
			timeout:  DEFAULT_TIMEOUT,
			tls:      None
		}
	}
}

pub struct ConnectionInner {
	pub options: ConnectionOptions,
	db:          RwLock<String>,
	query_token: AtomicU64,
	stream:      Mutex<wire::Stream>
}

#[derive(Clone)]
pub struct Connection(Arc<ConnectionInner>);

impl Connection {
	pub fn connect(options: ConnectionOptions) -> Result<Self> {
		Ok(Self(Arc::new(ConnectionInner {
			db:          RwLock::new(options.db.clone().into_owned()),
			query_token: AtomicU64::new(0),
			stream:      Mutex::new(wire::Stream::connect(&options)?),
			options
		})))
	}
	
	fn new_token(&self) -> QueryToken {
		self.0.query_token.fetch_add(1, Ordering::SeqCst)
	}
	
	fn send_recv<Q: Serialize, T: DeserializeOwned>(&self, token: QueryToken, query: Query<Q>) -> Result<Response<T>> {
		let start   = std::time::Instant::now();
		let noreply = query.options.as_ref().and_then(|o| o.noreply) == Some(true);
		let buf     = serde_json::to_vec(&query).map_err(|e| Error::Serialize(e.to_string()))?;
		let mut stream = self.0.stream.lock().unwrap();
		stream.send(token, &buf)?;
		
		if noreply {
			log::trace!("completed query #{} ({}ms, noreply) OK", token, start.elapsed().as_millis());
			return Ok(Response::new(ResponseType::SuccessSequence))
		}
		
		let (recv_token, buf) = stream.recv()?;
		if recv_token != token {
			log::trace!("completed query #{} ({}ms, noreply) ERROR", token, start.elapsed().as_millis());
			return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "tokens did not match")));
		}
		
		let r = Response::from_buf(&buf);
		
		log::trace!(
			"completed query #{} ({}ms) {}",
			token,
			start.elapsed().as_millis(),
			if r.is_ok() { "OK" } else { "ERROR" }
		);
		r
	}
	
	pub fn close(self, noreply_wait: bool) -> Result<()> {
		if noreply_wait {
			self.noreply_wait()?;
		}
		
		Ok(())
	}
	
	pub fn reconnect(&self, noreply_wait: bool) -> Result<()> {
		if noreply_wait {
			self.noreply_wait()?;
		}
		
		*self.0.stream.lock().unwrap() = wire::Stream::connect(&self.0.options)?;
		
		Ok(())
	}
	
	pub fn use_db(&self, db: String) {
		*self.0.db.write().unwrap() = db;
	}
	
	pub fn query<Q: Serialize, T: DeserializeOwned>(&self, term: Q, options: Option<QueryOptions>) -> Result<Cursor<T>> {
		let token = self.new_token();
		self.send_recv::<_, T>(token, Query {
			r#type: QueryType::Start,
			term:   Some(term),
			options
		}).map(|r| Cursor::from_response(self, token, r))
	}
	
	pub fn noreply_wait(&self) -> Result<()> {
		let _: Response<()> = self.send_recv(self.new_token(), Query::<()> {
			r#type:  QueryType::NoReplyWait,
			term:    None,
			options: None
		})?;
		Ok(())
	}
	
	pub fn server(&self) -> Result<ServerInfo> {
		self.send_recv(self.new_token(), Query::<()> {
			r#type:  QueryType::ServerInfo,
			term:    None,
			options: None
		})?.result.pop()
			.ok_or_else(|| Error::InvalidReply("no results returned".to_string()))
			.and_then(DocResult::into_result)
	}
}

impl std::fmt::Debug for Connection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Connection").finish_non_exhaustive()
	}
}

#[cfg(feature = "async")]
pub struct AsyncConnectionInner {
	pub options: ConnectionOptions,
	db:          RwLock<String>,
	query_token: AtomicU64,
	scheduler:   scheduler::Scheduler
}

#[cfg(feature = "async")]
#[derive(Clone)]
pub struct AsyncConnection(Arc<AsyncConnectionInner>);

#[cfg(feature = "async")]
impl AsyncConnection {
	pub async fn connect(options: ConnectionOptions) -> Result<Self> {
		Ok(Self(Arc::new(AsyncConnectionInner {
			db:          RwLock::new(options.db.clone().into_owned()),
			query_token: AtomicU64::new(0),
			scheduler:   scheduler::Scheduler::new(wire::AsyncStream::connect(&options).await?),
			options
		})))
	}
	
	fn new_token(&self) -> QueryToken {
		self.0.query_token.fetch_add(1, Ordering::SeqCst)
	}
	
	async fn send_recv<Q: Serialize, T: DeserializeOwned>(&self, token: QueryToken, query: Query<'_, Q>) -> Result<Response<T>> {
		let start   = std::time::Instant::now();
		let noreply = query.options.as_ref().and_then(|o| o.noreply) == Some(true);
		let buf     = serde_json::to_vec(&query).map_err(|e| Error::Serialize(e.to_string()))?;
		
		let result = match self.0.scheduler.dispatch(token, buf, !noreply).await {
			Ok(None)      => Ok(Response::new(ResponseType::SuccessAtom)),
			Ok(Some(buf)) => Response::from_buf(&buf),
			Err(e)        => Err(e)
		};
		
		log::trace!(
			"completed query #{} ({}ms{}) {}",
			token,
			start.elapsed().as_millis(),
			if noreply { ", noreply" } else { "" },
			if result.is_ok() { "OK" } else { "ERROR" }
		);
		
		result
	}
	
	pub async fn close(self, noreply_wait: bool) -> Result<()> {
		if noreply_wait {
			self.noreply_wait().await?;
		}
		
		Ok(())
	}
	
	pub async fn reconnect(&self, noreply_wait: bool) -> Result<()> {
		if noreply_wait {
			self.noreply_wait().await?;
		}
		
		self.0.scheduler.set_stream(wire::AsyncStream::connect(&self.0.options).await?).await;
		
		Ok(())
	}
	
	pub fn use_db(&self, db: String) {
		*self.0.db.write().unwrap() = db;
	}
	
	pub async fn query<Q: Serialize, T: 'static + DeserializeOwned + Send + Sync>(&self, term: Q, options: Option<QueryOptions<'_>>) -> Result<AsyncCursor<T>> {
		let token = self.new_token();
		self.send_recv::<_, T>(token, Query {
			r#type: QueryType::Start,
			term:   Some(term),
			options
		}).await.map(|r| AsyncCursor::from_response(self, token, r))
	}
	
	pub async fn noreply_wait(&self) -> Result<()> {
		let _: Response<()> = self.send_recv(self.new_token(), Query::<()> {
			r#type:  QueryType::NoReplyWait,
			term:    None,
			options: None
		}).await?;
		Ok(())
	}
	
	pub async fn server(&self) -> Result<ServerInfo> {
		self.send_recv(self.new_token(), Query::<()> {
			r#type:  QueryType::ServerInfo,
			term:    None,
			options: None
		}).await?.result.pop()
			.ok_or_else(|| Error::InvalidReply("no results returned".to_string()))
			.and_then(DocResult::into_result)
	}
}

#[cfg(feature = "async")]
impl std::fmt::Debug for AsyncConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AsyncConnection").finish_non_exhaustive()
	}
}

#[derive(Debug)]
pub enum Error {
	Sync(&'static str),
	Io(std::io::Error),
	#[cfg(feature = "tls")]
	Tls(rustls::TLSError),
	#[cfg(feature = "tls")]
	Dns(webpki::InvalidDNSNameError),
	Auth(AuthError),
	Reql(ReqlError, String),
	InvalidReply(String),
	Serialize(String),
	Deserialize(String)
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Self {
		Self::Io(e)
	}
}

impl<T> From<std::sync::PoisonError<T>> for Error {
	fn from(_: std::sync::PoisonError<T>) -> Self {
		Self::Sync("poison error")
	}
}

impl From<rustls::TLSError> for Error {
	fn from(e: rustls::TLSError) -> Self {
		Self::Tls(e)
	}
}

impl From<webpki::InvalidDNSNameError> for Error {
	fn from(e: webpki::InvalidDNSNameError) -> Self {
		Self::Dns(e)
	}
}

impl From<(ReqlError, String)> for Error {
	fn from((err, msg): (ReqlError, String)) -> Self {
		Self::Reql(err, msg)
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Id(pub u128);

impl std::str::FromStr for Id {
	type Err = <u128 as std::str::FromStr>::Err;
	
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		if s.len() != 36 {
			return Err("".parse::<u128>().unwrap_err());
		}
		
		Ok(Self(u128::from_str_radix(&s[0..8], 16)? << 96
			| u128::from_str_radix(&s[9..13], 16)? << 80
			| u128::from_str_radix(&s[14..18], 16)? << 64
			| u128::from_str_radix(&s[19..23], 16)? << 48
			| u128::from_str_radix(&s[24..36], 16)?))
	}
}

impl std::fmt::Debug for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
			   (self.0 & 0xFFFF_FFFF_0000_0000_0000_0000_0000_0000) >> 96,
			   (self.0 & 0x0000_0000_FFFF_0000_0000_0000_0000_0000) >> 80,
			   (self.0 & 0x0000_0000_0000_FFFF_0000_0000_0000_0000) >> 64,
			   (self.0 & 0x0000_0000_0000_0000_FFFF_0000_0000_0000) >> 48,
			   (self.0 & 0x0000_0000_0000_0000_0000_FFFF_FFFF_FFFF))
	}
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}

impl<'de> serde::Deserialize<'de> for Id {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
		use serde::de::Error;
		
		if deserializer.is_human_readable() {
			<&str>::deserialize(deserializer)?.parse::<Self>().map_err(|e| D::Error::custom(e.to_string()))
		} else {
			<u128>::deserialize(deserializer).map(Self)
		}
	}
}

impl serde::Serialize for Id {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}