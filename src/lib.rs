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

#![forbid(unsafe_code)]
#![warn(clippy::all)]

use {
	std::{time::Duration, sync::{Arc, Mutex, RwLock, atomic::{AtomicU64, Ordering}}},
	serde::de::DeserializeOwned,
	self::auth::AuthError
};

pub use self::{wire::*, reql::*, cursor::*};

pub mod wire;
pub mod reql;
pub mod cursor;
pub mod auth;

pub type QueryToken = u64;
pub type ReDBResult<T> = std::result::Result<T, Error>;
type Result<T> = std::result::Result<T, Error>;

pub const DEFAULT_HOST:    &str     = "localhost";
pub const DEFAULT_PORT:    u16      = 28015;
pub const DEFAULT_DB:      &str     = "test";
pub const DEFAULT_USER:    &str     = "admin";
pub const DEFAULT_PWD:     &str     = "";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(20);

#[derive(Clone, Debug, Default)]
pub struct ConnectionOptions {
	/// the host to connect to (default `localhost`)
	pub hostname: Option<String>,
	/// the port to connect on (default `28015`)
	pub port:     Option<u16>,
	/// the default database (default `test`)
	pub db:       Option<String>,
	/// the user account to connect as (default `"admin"`)
	pub user:     Option<String>,
	/// the password for the user account to connect as (default `""`)
	pub password: Option<String>,
	/// timeout period for the connection to be opened (default `20`)
	pub timeout:  Option<Duration>,
	/// a hash of options to support TLS/SSL connections (default `None`)
	#[cfg(feature = "tls")]
	pub tls:      Option<DebugImpl<Arc<rustls::ClientConfig>>>
}

impl ConnectionOptions {
	pub fn connect(self) -> Result<RDbConnection> {
		RDbConnection::connect(self)
	}
	
	#[cfg(feature = "async")]
	pub async fn connect_async(self) -> Result<RDbAsyncConnection> {
		RDbAsyncConnection::connect(self).await
	}
}

#[derive(Debug)]
pub struct ConnectionInner {
	pub options: ConnectionOptions,
	db:          RwLock<String>,
	query_token: AtomicU64,
	stream:      Mutex<Stream>
}

#[derive(Clone)]
pub struct RDbConnection(Arc<ConnectionInner>);

impl RDbConnection {
	pub fn connect(options: ConnectionOptions) -> Result<Self> {
		Ok(Self(Arc::new(ConnectionInner {
			db:          RwLock::new(options.db.clone().unwrap_or_else(|| DEFAULT_DB.to_string())),
			query_token: AtomicU64::new(0),
			stream:      Mutex::new(Stream::connect(
				options.hostname.as_deref().unwrap_or(DEFAULT_HOST),
				options.port.unwrap_or(DEFAULT_PORT),
				&options
			)?),
			options
		})))
	}
	
	fn new_token(&self) -> QueryToken {
		self.0.query_token.fetch_add(1, Ordering::SeqCst)
	}
	
	fn send_recv<Q: ReqlTerm, T: DeserializeOwned>(&self, token: QueryToken, query: Query<Q>) -> Result<Response<T>> {
		let start = std::time::Instant::now();
		let mut buf = Vec::new();
		query.serialize(&mut buf)?;
		let mut stream = self.0.stream.lock().unwrap();
		stream.send(token, &buf)?;
		
		if query.options.as_ref().and_then(|o| o.noreply) == Some(true) {
			log::trace!("completed query #{} ({}ms, noreply)", token, start.elapsed().as_millis());
			return Ok(Response::new(ResponseType::SuccessSequence))
		}
		
		let (recv_token, buf) = stream.recv()?;
		if recv_token != token {
			return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "tokens did not match")));
		}
		
		let r = Response::from_buf(&buf);
		log::trace!("completed query #{} ({}ms)", token, start.elapsed().as_millis());
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
		
		*self.0.stream.lock().unwrap() = Stream::connect(
			self.0.options.hostname.as_deref().unwrap_or(DEFAULT_HOST),
			self.0.options.port.unwrap_or(DEFAULT_PORT),
			&self.0.options
		)?;
		
		Ok(())
	}
	
	pub fn use_db(&self, db: String) {
		*self.0.db.write().unwrap() = db;
	}
	
	pub fn run<Q: reql::ReqlTerm, T: DeserializeOwned>(&self, term: Q, options: Option<QueryOptions>) -> Result<Cursor<T>> {
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

impl std::fmt::Debug for RDbConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncConnectionInner {
	pub options: ConnectionOptions,
	db:          RwLock<String>,
	query_token: AtomicU64,
	stream:      async_mutex::Mutex<AsyncStream>
}

#[cfg(feature = "async")]
#[derive(Clone)]
pub struct RDbAsyncConnection(Arc<AsyncConnectionInner>);

#[cfg(feature = "async")]
impl RDbAsyncConnection {
	pub async fn connect(options: ConnectionOptions) -> Result<Self> {
		Ok(Self(Arc::new(AsyncConnectionInner {
			db:          RwLock::new(options.db.clone().unwrap_or_else(|| DEFAULT_DB.to_string())),
			query_token: AtomicU64::new(0),
			stream:      async_mutex::Mutex::new(AsyncStream::connect(
				options.hostname.as_deref().unwrap_or(DEFAULT_HOST),
				options.port.unwrap_or(DEFAULT_PORT),
				&options
			).await?),
			options
		})))
	}
	
	fn new_token(&self) -> QueryToken {
		self.0.query_token.fetch_add(1, Ordering::SeqCst)
	}
	
	async fn send_recv<Q: ReqlTerm, T: DeserializeOwned>(&self, token: QueryToken, query: Query<'_, Q>) -> Result<Response<T>> {
		let start = std::time::Instant::now();
		let mut buf = Vec::new();
		query.serialize(&mut buf)?;
		let mut stream = self.0.stream.lock().await;
		stream.send(token, &buf).await?;
		
		if query.options.as_ref().and_then(|o| o.noreply) == Some(true) {
			log::debug!("completed query #{} ({}ms, noreply)", token, start.elapsed().as_millis());
			return Ok(Response::new(ResponseType::SuccessSequence))
		}
		
		let (recv_token, buf) = stream.recv().await?;
		if recv_token != token {
			return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "tokens did not match")));
		}
		
		let r = Response::from_buf(&buf);
		log::debug!("completed query #{} ({}ms)", token, start.elapsed().as_millis());
		r
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
		
		let mut guard = self.0.stream.lock().await;
		*guard = AsyncStream::connect(
			self.0.options.hostname.as_deref().unwrap_or(DEFAULT_HOST),
			self.0.options.port.unwrap_or(DEFAULT_PORT),
			&self.0.options
		).await?;
		
		Ok(())
	}
	
	pub fn use_db(&self, db: String) {
		*self.0.db.write().unwrap() = db;
	}
	
	pub async fn run<Q: ReqlTerm, T: DeserializeOwned>(&self, term: Q, options: Option<QueryOptions<'_>>) -> Result<AsyncCursor<T>> {
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
impl std::fmt::Debug for RDbAsyncConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

#[derive(Debug)]
pub enum Error {
	Sync,
	Io(std::io::Error),
	#[cfg(feature = "tls")]
	Tls(rustls::TLSError),
	#[cfg(feature = "tls")]
	Dns(webpki::InvalidDNSNameError),
	Auth(AuthError),
	Reql(ReqlError, String),
	InvalidReply(String),
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
		Self::Sync
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
pub struct RDbId(u128);

impl std::str::FromStr for RDbId {
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

impl std::fmt::Debug for RDbId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
			   (self.0 & 0xFFFF_FFFF_0000_0000_0000_0000_0000_0000) >> 96,
			   (self.0 & 0x0000_0000_FFFF_0000_0000_0000_0000_0000) >> 80,
			   (self.0 & 0x0000_0000_0000_FFFF_0000_0000_0000_0000) >> 64,
			   (self.0 & 0x0000_0000_0000_0000_FFFF_0000_0000_0000) >> 48,
			   (self.0 & 0x0000_0000_0000_0000_0000_FFFF_FFFF_FFFF))
	}
}

impl std::fmt::Display for RDbId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}

impl<'de> serde::Deserialize<'de> for RDbId {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
		use serde::de::Error;
		
		if deserializer.is_human_readable() {
			<&str>::deserialize(deserializer)?.parse::<Self>().map_err(|e| D::Error::custom(e.to_string()))
		} else {
			<u128>::deserialize(deserializer).map(Self)
		}
	}
}

impl serde::Serialize for RDbId {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}

/// A wrapper that implements `Debug` for a type that doesn't.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default)]
pub struct DebugImpl<T>(pub T);

impl<T> std::fmt::Debug for DebugImpl<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct(std::any::type_name::<T>())
			.finish()
	}
}

impl<T> std::ops::Deref for DebugImpl<T> {
	type Target = T;
	
	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> std::ops::DerefMut for DebugImpl<T> {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[cfg(test)]
mod tests {
	use {super::*, log::{Metadata, Record}, std::{io::Write, sync::atomic::AtomicBool}, serde::Deserialize};
	
	#[derive(Deserialize, Debug)]
	struct Test {
		id: String,
		test: usize
	}
	
	#[test]
	fn test_rdbid() {
		use std::str::FromStr;
		let id = RDbId::from_str("1065a246-64d2-4dea-a191-506f653732ac").unwrap();
		assert_eq!(id.0, 0x1065a24664d24deaa191506f653732ac);
		assert_eq!(&id.to_string(), "1065a246-64d2-4dea-a191-506f653732ac");
	}
	
	fn init_conn() -> RDbConnection {
		static LOGGER: Logger = Logger;
		static INIT: AtomicBool = AtomicBool::new(false);
		
		if !INIT.swap(true, Ordering::SeqCst) {
			log::set_logger(&LOGGER).unwrap();
			log::set_max_level(log::LevelFilter::Trace);
		}
		
		ConnectionOptions::default()
			.connect()
			.unwrap()
	}
	
	#[test]
	#[ignore]
	fn connect_success() {
		init_conn();
	}
	
	#[test]
	#[ignore]
	fn connect_invalid_user() {
		let err = ConnectionOptions {
			user: Some("invalid".to_string()),
			..ConnectionOptions::default()
		}.connect();
		
		debug_assert!(matches!(err, Err(Error::Auth(AuthError::ServerError { code: Some(17), .. }))));
	}
	
	#[test]
	#[ignore]
	fn query_bad_request() {
		struct InvalidTerm;
		
		impl ReqlTerm for InvalidTerm {
			fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
				dst.write_all(b"invald query")
			}
		}
		
		let conn = init_conn();
		let err = conn.send_recv::<_, ()>(0, Query {
			r#type:  QueryType::Start,
			term:    Some(InvalidTerm),
			options: None
		});
		
		debug_assert!(matches!(err, Err(Error::Reql(ReqlError::Driver(ReqlDriverError::BadRequest), _))));
	}
	
	#[test]
	#[ignore]
	fn query_no_db() {
		let conn = init_conn();
		let err = db("non-existent_db")
			.table("test")
			.run::<()>(&conn, None);
		
		debug_assert!(matches!(err, Err(Error::Reql(ReqlError::Runtime(ReqlRuntimeError::OpFailed), _))));
	}
	
	#[test]
	#[ignore]
	fn query_serde_err() {
		#[derive(Deserialize, Debug)]
		struct Test { id: String, invalid: usize }
		
		let conn = init_conn();
		let err = db("test")
			.table("test")
			.run::<Test>(&conn, None)
			.unwrap()
			.next();
		
		debug_assert!(matches!(err, Some(Err(Error::Deserialize(_)))));
	}
	
	#[test]
	#[ignore]
	fn query_success() {
		let conn = init_conn();
		let ok = db("test")
			.table("test")
			.run::<Test>(&conn, None);
		
		debug_assert!(ok.is_ok());
	}
	
	#[test]
	#[ignore]
	fn query_with_options_success() {
		let conn = init_conn();
		let ok = db("test")
			.table("test")
			.run::<Test>(&conn, Some(QueryOptions {
				read_mode: Some(ReadMode::Majority),
				.. QueryOptions::default()
			}));
		
		debug_assert!(ok.is_ok());
	}
	
	#[test]
	#[ignore]
	fn query_noreply() {
		let conn = init_conn();
		let ok = db("test")
			.table("test")
			.run::<()>(&conn, Some(QueryOptions {
				noreply: Some(true),
				.. QueryOptions::default()
			}));
		
		debug_assert!(ok.is_ok());
	}
	
	struct Logger;
	
	impl log::Log for Logger {
		fn enabled(&self, _metadata: &Metadata) -> bool {
			true
		}
	
		fn log(&self, record: &Record) {
			println!("{:?} {}:{} {}", record.level(), record.target(), record.line().unwrap_or(0), record.args());
		}
		
		fn flush(&self) {}
	}
}