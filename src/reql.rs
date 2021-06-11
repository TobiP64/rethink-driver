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

use {
	crate::*,
	std::{time::Duration, io::Write},
	serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned},
	serde_repr::*
};

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};

pub use self::{
	result::*,
	types::*,
	primitives::*,
	expr::*,
	none::*,
	cast::*,
	var::*,
	var_args::*,
	typed_fn::*,
	terms::*
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize_repr)]
#[repr(u32)]
pub enum QueryType {
	Start       = 1,
	Continue    = 2,
	Stop        = 3,
	NoReplyWait = 4,
	ServerInfo  = 5
}

#[derive(Clone, Debug)]
pub struct Query<'a, T: ReqlTerm = ()> {
	pub r#type:  QueryType,
	pub term:    Option<T>,
	pub options: Option<QueryOptions<'a>>
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct QueryOptions<'a> {
	/// One of three possible values affecting the consistency guarantee for the query
	/// (default: `Single`)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub read_mode:                    Option<ReadMode>,
	/// What format to return times in (default: `Native`). Set this to `Raw` if you want
	/// times returned as JSON objects for exporting.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub time_format:                  Option<Format>,
	/// Whether or not to return a profile of the query’s execution (default: `false`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub profile:                      Option<bool>,
	/// In soft durability mode RethinkDB will acknowledge the write immediately after
	/// receiving it, but before the write has been committed to disk.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub durability:                   Option<Durability>,
	/// What format to return grouped_data and grouped_streams in (default: `Native`).
	/// Set this to `Raw` if you want the raw pseudotype.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub group_format:                 Option<Format>,
	/// Set to `true` to not receive the result object or cursor and return immediately.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub noreply:                      Option<bool>,
	/// The database to run this query against as a string.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub db:                           Option<&'a str>,
	/// The maximum numbers of array elements that can be returned by a query (default: `100_000`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub array_limit:                  Option<usize>,
	/// What format to return binary data in (default: `native`). Set this to `Raw` if
	/// you want the raw pseudotype.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub binary_format:                Option<Format>,
	/// Minimum number of rows to wait for before batching a result set (default: `8`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_batch_rows:               Option<usize>,
	/// Maximum number of rows to wait for before batching a result set (default: `unlimited`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_batch_rows:               Option<usize>,
	/// Maximum number of bytes to wait for before batching a result set (default: `1MB`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_batch_bytes:              Option<usize>,
	/// Maximum number of seconds to wait before batching a result set (default: `500ms`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_batch_seconds:            Option<Duration>,
	/// Factor to scale the other parameters down by on the first batch (default: `4`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub first_batch_scaledown_factor: Option<usize>
}

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
	Native,
	Raw
}

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadMode {
	Single,
	Majority,
	Outdated
}

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Durability {
	Hard,
	Soft
}

impl<T: ReqlTerm> Query<'_, T> {
	pub fn serialize(&self, mut dst: &mut impl Write) -> std::io::Result<()> {
		write!(dst, "[{}", self.r#type as u32)?;
		
		if let Some(term) = &self.term {
			dst.write_all(b",")?;
			term.serialize(dst)?;
		}
		
		if let Some(global_optargs) = &self.options {
			dst.write_all(b",")?;
			serde_json::to_writer(&mut dst, &global_optargs)?;
			dst.write_all(b"]")
		} else {
			dst.write_all(b",{}]")
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize_repr)]
#[repr(u32)]
pub enum ResponseType {
	SuccessAtom     = 1,
	SuccessSequence = 2,
	SuccessPartial  = 3,
	WaitComplete    = 4,
	ServerInfo      = 5,
	ClientError     = 16,
	CompileError    = 17,
	RuntimeError    = 18
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize_repr)]
#[repr(u32)]
pub enum ReqlRuntimeError {
	Internal        = 1000000,
	ResourceLimit   = 2000000,
	QueryLogic      = 3000000,
	NonExistence    = 3100000,
	OpFailed        = 4100000,
	OpIndeterminate = 4200000,
	User            = 5000000,
	PermissionError = 6000000
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize_repr)]
#[repr(u32)]
pub enum ResponseNote {
	SequenceFeed     = 1,
	AtomFeed         = 2,
	OrderByLimitFeed = 3,
	UnionedFeed      = 4,
	IncludeStates    = 5
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct ResponseWithTypeOnly {
	#[serde(rename = "t")]
	pub r#type: ResponseType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response<T> {
	#[serde(rename = "t")]
	pub r#type:     ResponseType,
	#[serde(rename = "r", default = "Vec::new")]
	pub result:     Vec<DocResult<T>>,
	#[serde(rename = "p")]
	pub profile:    Option<std::string::String>,
	#[serde(rename = "n")]
	pub notes:      Option<Vec<ResponseNote>>
}

impl<T> Response<T> {
	pub fn new(ty: ResponseType) -> Self {
		Self {
			r#type:  ty,
			result:  Vec::new(),
			profile: None,
			notes:   None
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResponseWithError {
	#[serde(rename = "t")]
	pub r#type:     ResponseType,
	#[serde(rename = "e")]
	pub error:      Option<ReqlRuntimeError>,
	#[serde(rename = "r")]
	pub result:     Option<[String; 1]>,
	#[serde(rename = "b")]
	pub backtrace:  Option<Vec<usize>>,
}

impl<T: DeserializeOwned> Response<T> {
	pub fn from_buf(buf: &[u8]) -> Result<Self> {
		use {crate::Error::*, ReqlError::*};
		
		/*#[derive(Clone, Debug, Deserialize)]
		struct Response2<T> {
			#[serde(rename = "t")]
			r#type: ResponseType,
			//#[serde(flatten)]
			inner:  ResponseEnum<T>
		}*/
		
		#[derive(Clone, Debug, Deserialize)]
		#[serde(untagged)]
		enum ResponseEnum<T> {
			Ok(Response<T>),
			Err(ResponseWithError)
		}
		
		match serde_json::from_slice::<ResponseEnum<T>>(buf).map_err(|e| InvalidReply(e.to_string()))? {
			ResponseEnum::Err(ResponseWithError { r#type: ResponseType::ClientError, result, .. }) =>
				Err(Reql(Driver(ReqlDriverError::BadRequest), result.map_or_else(String::new, |[v]| v))),
			ResponseEnum::Err(ResponseWithError { r#type: ResponseType::CompileError, result, .. }) =>
				Err(Reql(Compile, result.map_or_else(String::new, |[v]| v))),
			ResponseEnum::Err(ResponseWithError { r#type: ResponseType::RuntimeError, error, result, .. }) =>
				Err(Reql(Runtime(error.unwrap_or(ReqlRuntimeError::Internal)), result.map_or_else(String::new, |[v]| v))),
			ResponseEnum::Ok(v) => Ok(v),
			_ => Err(InvalidReply("".to_string())),
		}
	}
}

pub type Document<T> = std::result::Result<T, String>;

#[derive(Debug)]
pub enum ReqlError {
	Compile,
	Runtime(ReqlRuntimeError),
	Driver(ReqlDriverError)
}

#[derive(Debug)]
pub enum ReqlDriverError {
	BadRequest,
	Auth
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
pub struct Binary;

pub trait Run: ReqlTerm + Sized {
	/// Run a query on the given connection. Returns a cursor that yields elements of the type `T`.
	fn run<T: DeserializeOwned>(self, conn: &RDbConnection, options: Option<QueryOptions>) -> Result<Cursor<T>> {
		conn.run(self, options)
	}
	
	/// Run a query on the given connection asynchronously. Returns a cursor that yields elements of the type `T`.
	#[cfg(feature = "async")]
	fn run_async<'a, T: DeserializeOwned + Send + Sync + Unpin + 'static>(self, conn: &'a RDbAsyncConnection, options: Option<QueryOptions<'a>>)
		-> Pin<Box<dyn Future<Output = Result<AsyncCursor<T>>> + Send + 'a>> where Self: 'a + Send + Sync + Unpin {
		Box::pin(conn.run(self, options))
	}
}

impl<T: ReqlTerm + Sized> Run for T {}

pub trait ReqlTerm {
	fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()>;
}

mod result {
	use super::*;
	
	#[derive(Clone, Debug)]
	pub enum DocResult<T> {
		Ok(T),
		Err(String)
	}
	
	impl<T> DocResult<T> {
		pub fn into_result(self) -> Result<T> {
			match self {
				Self::Ok(doc)  => Ok(doc),
				Self::Err(err) => Err(crate::Error::Deserialize(err))
			}
		}
	}
	
	impl<'de, T: Deserialize<'de>> Deserialize<'de> for DocResult<T> {
		fn deserialize<D>(deserializer: D) -> std::result::Result<Self, <D as Deserializer<'de>>::Error> where
			D: Deserializer<'de> {
			Ok(match T::deserialize(deserializer) {
				Ok(v)  => Self::Ok(v),
				Err(e) => Self::Err(e.to_string()),
			})
		}
	}
	
	/// Document returned by the `server_info` operation.
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct ServerInfo {
		pub id:    String,
		pub name:  String,
		pub proxy: bool
	}
	
	/// Document returned by the `db_create` operation.
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct DbCreateResult {
		/// Always `1`.
		pub dbs_created:    usize,
		#[serde(default)]
		pub config_changes: Vec<Change<DbConfig, DbConfig>>
	}
	
	/// Document returned by the `db_drop` operation.
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct DbDropResult {
		/// Always `1`.
		pub dbs_dropped:    usize,
		#[serde(default)]
		pub config_changes: Vec<Change<DbConfig, DbConfig>>
	}
	
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct DbConfig {
		pub id:   String,
		pub name: String
	}
	
	/// Document returned by the `table_create` operation.
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct TableCreateResult {
		/// Always `1`.
		pub tables_created: usize,
		#[serde(default)]
		pub config_changes: Vec<Change<ConfigResult, ()>>
	}
	
	/// Document returned by the `table_drop` operation.
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct TableDropResult {
		/// Always `1`.
		pub tables_dropped: usize,
		#[serde(default)]
		pub config_changes: Vec<Change<(), ConfigResult>>
	}
	
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct WriteHookResult {
		pub function: Binary,
		pub query:    String
	}
	
	/// Document returned by writing operations, such as `insert`, `update`, `replace`
	/// and `delete`.
	#[derive(Debug, Clone, Default, Deserialize)]
	pub struct WriteResult<O = (), N = ()> {
		/// The number of documents that were deleted.
		pub deleted:        usize,
		/// The number of errors encountered while performing the operation.
		pub errors:         usize,
		/// If errors were encountered, contains the text of the first error.
		pub first_error:    Option<String>,
		/// A list of generated primary keys for inserted documents whose primary keys were
		/// not specified (capped to 100,000).
		#[serde(default)]
		pub generated_keys: Vec<RDbId>,
		/// The number of documents that were inserted.
		pub inserted:       usize,
		/// The number of documents that were replaced/updated.
		pub replaced:       usize,
		/// The number of documents that were skipped.
		pub skipped:        usize,
		/// The number of documents that would have been modified, except that the new value
		/// was the same as the old value.
		pub unchanged:      usize,
		/// If `return_changes` is set to `true`, this will be an array of objects, one for
		/// each object affected by the operation.
		///
		/// See `Change<N, O>`
		#[serde(default)]
		pub changes:        Vec<Change<N, O>>
	}
	
	#[derive(Copy, Clone, Debug, Default, Deserialize)]
	pub struct Change<N, O>{
		pub new_val: Option<N>,
		pub old_val: Option<O>
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct GrantResult {
		pub granted:             usize,
		#[serde(default)]
		pub permissions_changes: Vec<Change<Permissions, Permissions>>
	}
	
	#[derive(Copy, Clone, Debug, Default, Deserialize)]
	pub struct Permissions {
		pub read:    Option<bool>,
		pub write:   Option<bool>,
		pub connect: Option<bool>,
		pub config:  Option<bool>
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct ConfigResult {
		pub db:          String,
		pub id:          String,
		pub name:        String,
		pub primary_key: String,
		pub shards:      Vec<ConfigShard>,
		pub indexes:     Vec<String>,
		pub write_acks:  String,
		pub write_hook:  Option<Binary>,
		pub durability:  String
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct ConfigShard {
		pub primary_replica:    String,
		pub replicas:           Vec<String>,
		pub nonvoting_replicas: Vec<String>
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct StatusResult {
		pub db:          String,
		pub id:          String,
		pub name:        String,
		pub raft_leader: String,
		pub shards:      Vec<StatusShard>,
		pub status:      Status
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct StatusShard {
		pub primary_replica:    String,
		pub replicas:           Vec<StatusReplica>
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct StatusReplica {
		pub server: String,
		pub state:  String
	}
	
	#[derive(Copy, Clone, Debug, Default, Deserialize)]
	pub struct Status {
		pub all_replicas_ready:       bool,
		pub ready_for_outdated_reads: bool,
		pub ready_for_reads:          bool,
		pub ready_for_writes:         bool
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct RebalanceResult {
		pub rebalanced:     usize,
		pub status_changes: Vec<Change<StatusResult, StatusResult>>
	}
	
	#[derive(Clone, Debug, Default, Deserialize)]
	pub struct ReconfigureResult {
		pub reconfigured:   usize,
		pub config_changes: Vec<Change<ConfigResult, ConfigResult>>,
		pub status_changes: Vec<Change<StatusResult, StatusResult>>
	}
}

mod types {
	pub use super::*;
	
	pub trait ReqlTop:             ReqlTerm {}
	pub trait ReqlDatum:           ReqlTop {}
	pub trait ReqlNull:            ReqlDatum {}
	pub trait ReqlBool:            ReqlDatum {}
	pub trait ReqlNumber:          ReqlDatum {}
	pub trait ReqlString:          ReqlDatum {}
	pub trait ReqlObject:          ReqlDatum {}
	pub trait ReqlSingleSelection: ReqlObject {}
	pub trait ReqlArray:           ReqlDatum + ReqlSequence {}
	pub trait ReqlSequence:        ReqlTop {}
	pub trait ReqlStream:          ReqlSequence {}
	pub trait ReqlStreamSelection: ReqlStream {}
	pub trait ReqlTable:           ReqlStreamSelection {}
	pub trait ReqlDatabase:        ReqlTop {}
	pub trait ReqlFunction:        ReqlTop { type Args; type Output; }
	pub trait ReqlOrdering:        ReqlTop {}
	pub trait ReqlPathSpec:        ReqlTop {}
	pub trait ReqlErrorTy:         ReqlTerm {}
	
	pub struct ReqlDynTop;
	pub struct ReqlDynDatum;
	pub struct ReqlDynNull;
	pub struct ReqlDynBool;
	pub struct ReqlDynNumber;
	pub struct ReqlDynString;
	pub struct ReqlDynObject;
	pub struct ReqlDynSingleSelection;
	pub struct ReqlDynArray;
	pub struct ReqlDynSequence;
	pub struct ReqlDynStream;
	pub struct ReqlDynStreamSelection;
	pub struct ReqlDynTable;
	pub struct ReqlDynDatabase;
	pub struct ReqlDynFunction;
	pub struct ReqlDynOrdering;
	pub struct ReqlDynPathSpec;
	pub struct ReqlDynError;
}

mod primitives {
	use super::*;
	
	impl ReqlTop for () {}
	impl ReqlDatum for () {}
	impl ReqlNull for () {}
	impl ReqlTerm for () {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			dst.write_all(b"null")
		}
	}
	
	impl ReqlTop for bool {}
	impl ReqlDatum for bool {}
	impl ReqlBool for bool {}
	impl ReqlTerm for bool {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			dst.write_all(if *self { b"true" } else { b"false" })
		}
	}
	
	macro_rules! reql_number_impl {
		( $( $ty:ty ),* ) => {
			$(
				impl ReqlTop for $ty {}
				impl ReqlDatum for $ty {}
				impl ReqlNumber for $ty {}
				impl ReqlTerm for $ty {
					fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
						write!(dst, "{}", self)
					}
				}
			)*
		};
	}
	
	reql_number_impl!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);
	
	impl<T: ReqlTop> ReqlTerm for [T] {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			if self.is_empty() {
				return dst.write_all(b"[2, []]");
			}
			
			dst.write_all(b"[2, [")?;
			self[0].serialize(dst)?;
			
			for e in &self[1..] {
				dst.write_all(b",")?;
				e.serialize(dst)?;
			}
			
			dst.write_all(b"]]")
		}
	}
	
	macro_rules! reql_array_impl {
		(@internal $( < $( $lifetime:lifetime ),* > $ty:ty )? ) => {};
		(@internal ser $( < $( $lifetime:lifetime ),* > $ty:ty )? ) => {
			$(
				impl< $( $lifetime, )* T: ReqlTop> ReqlTerm     for $ty {
					fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
						<[T] as ReqlTerm>::serialize(&*self, dst)
					}
				}
			)*
		};
		( $( $( $ident:ident )? < $( $lifetime:lifetime ),* > $ty:ty ),* ) => {
			$(
				impl< $( $lifetime, )* T: ReqlTop> ReqlTop      for $ty {}
				impl< $( $lifetime, )* T: ReqlTop> ReqlDatum    for $ty {}
				impl< $( $lifetime, )* T: ReqlTop> ReqlSequence for $ty {}
				impl< $( $lifetime, )* T: ReqlTop> ReqlArray    for $ty {}
				reql_array_impl!(@internal $( $ident )? < $( $lifetime ),* > $ty );
			)*
		};
	}
	
	reql_array_impl!(<> [T], ser <'a> &'a [T], ser <> Vec<T>, ser <'a> &'a Vec<T>);
	
	impl ReqlTerm for str {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			write!(dst, "\"{}\"", self)
		}
	}
	
	macro_rules! reql_str_impl {
		(@internal $( < $( $lifetime:lifetime ),* > $ty:ty )? ) => {};
		(@internal ser $( < $( $lifetime:lifetime ),* > $ty:ty )? ) => {
			$(
				impl< $( $lifetime, )*> ReqlTerm     for $ty {
					fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
						<str as ReqlTerm>::serialize(&*self, dst)
					}
				}
			)*
		};
		( $( $( $ident:ident )? < $( $lifetime:lifetime ),* > $ty:ty ),* ) => {
			$(
				impl< $( $lifetime, )* > ReqlTop      for $ty {}
				impl< $( $lifetime, )* > ReqlDatum    for $ty {}
				impl< $( $lifetime, )* > ReqlString   for $ty {}
				impl< $( $lifetime, )* > ReqlPathSpec for $ty {}
				reql_str_impl!(@internal $( $ident )? < $( $lifetime ),* > $ty );
			)*
		};
	}
	
	reql_str_impl!(<> str, ser <'a> &'a str, ser <> String, ser <'a> &'a String);
	
	impl ReqlTop for RDbId {}
	impl ReqlDatum for RDbId {}
	impl ReqlString for RDbId {}
	impl ReqlTerm for RDbId {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			write!(dst, "\"{}\"", self)
		}
	}
}

mod expr {
	use super::*;
	
	#[derive(Clone, Default, Debug)]
	pub struct ReqlExpr(pub String);
	
	impl<T: serde::Serialize> From<&T> for ReqlExpr {
		fn from(v: &T) -> Self {
			Self(serde_json::to_string(&v).unwrap())
		}
	}
	
	impl ReqlExpr {
		pub fn obj() -> ReqlExprObjBuilder {
			ReqlExprObjBuilder::default()
		}
		
		pub fn seq() -> ReqlExprSeqBuilder {
			ReqlExprSeqBuilder::default()
		}
	}
	
	#[derive(Clone, Default, Debug)]
	pub struct ReqlExpr2<F: Fn(&mut dyn Write) -> std::io::Result<()>>(F);
	
	impl<F: Fn(&mut dyn Write) -> std::io::Result<()>> ReqlTerm for ReqlExpr2<F> {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			self.0(dst)
		}
	}
	
	impl ReqlTop for ReqlExpr {}
	impl ReqlDatum for ReqlExpr {}
	impl ReqlObject for ReqlExpr {}
	
	impl ReqlTerm for ReqlExpr {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			dst.write_all(self.0.as_bytes())
		}
	}
	
	pub struct ReqlExprObjBuilder(Vec<u8>);
	
	impl Default for ReqlExprObjBuilder {
		fn default() -> Self {
			Self(vec![b'{'])
		}
	}
	
	impl ReqlExprObjBuilder {
		pub fn field(&mut self, key: &str, val: impl ReqlTerm) -> &mut Self {
			write!(&mut self.0, "\"{}\":", key).unwrap();
			val.serialize(&mut self.0).unwrap();
			self.0.push(b',');
			self
		}
		
		pub fn finish(mut self) -> ReqlExpr {
			if self.0.ends_with(&[b',']) {
				self.0.truncate(self.0.len() - 1);
			}
			
			self.0.push(b'}');
			ReqlExpr(String::from_utf8(self.0).unwrap())
		}
	}
	
	pub struct ReqlExprSeqBuilder(Vec<u8>);
	
	impl Default for ReqlExprSeqBuilder {
		fn default() -> Self {
			Self(vec![b'['])
		}
	}
	
	impl ReqlExprSeqBuilder {
		pub fn element(&mut self, val: impl ReqlTerm) -> &mut Self {
			val.serialize(&mut self.0).unwrap();
			self.0.push(b',');
			self
		}
		
		pub fn finish(mut self) -> ReqlExpr {
			if self.0.ends_with(&[b',']) {
				self.0.truncate(self.0.len() - 1);
			}
			
			self.0.push(b']');
			ReqlExpr(String::from_utf8(self.0).unwrap())
		}
	}
	
	#[macro_export]
	macro_rules! obj {
		() => {{ ReqlExpr("{}".to_string()) }};
		() => [{ ReqlExpr("[]".to_string()) }];
		( $key0:ident: $val0:expr $(, $key:ident: $val:expr )* ) => {{
			ReqlExpr({
				let mut buf = Vec::new();
				std::io::Write::write_all(&mut buf, concat!("{", "\"", stringify!($key0), "\":").as_bytes()).unwrap();
				$crate::reql::reql_to_string(&mut buf, $val0);
				
				$(
					std::io::Write::write_all(&mut buf, concat!(",\"", stringify!($key), "\":").as_bytes()).unwrap();
					$crate::reql::reql_to_string(&mut buf, $val);
				)*
				
				buf.push(b'}');
				String::from_utf8(buf).unwrap()
			})
		}};
		( $val0:ident $(, $val:ident )* ) => [{
			ReqlExpr(concat!("[", stringify!($val0), $( ",", stringify!($val), )* "]"))
		}]
	}
	
	#[macro_export]
	macro_rules! func {
		(move | $( $ident:ident: $ty:ty ),* | $expr:expr) => {
			(move | $( $ident: $ty, )* | $expr, std::marker::PhantomData::<dyn Fn( $( $ty, )* ) -> _ + Send + Sync + Unpin>)
		};
		( | $( $ident:ident: $ty:ty ),* | $expr:expr) => {
			(| $( $ident: $ty, )* | $expr, std::marker::PhantomData::<dyn Fn( $( $ty, )* ) -> _ + Send + Sync + Unpin>)
		};
	}
}

mod none {
	use super::*;
	
	#[derive(Copy, Clone, Default, Debug)]
	pub struct ReqlNoneTerm;
	
	impl ReqlTop             for ReqlNoneTerm {}
	impl ReqlDatum           for ReqlNoneTerm {}
	impl ReqlNull            for ReqlNoneTerm {}
	impl ReqlBool            for ReqlNoneTerm {}
	impl ReqlNumber          for ReqlNoneTerm {}
	impl ReqlString          for ReqlNoneTerm {}
	impl ReqlObject          for ReqlNoneTerm {}
	impl ReqlSingleSelection for ReqlNoneTerm {}
	impl ReqlArray           for ReqlNoneTerm {}
	impl ReqlSequence        for ReqlNoneTerm {}
	impl ReqlStream          for ReqlNoneTerm {}
	impl ReqlStreamSelection for ReqlNoneTerm {}
	impl ReqlTable           for ReqlNoneTerm {}
	impl ReqlDatabase        for ReqlNoneTerm {}
	impl ReqlFunction        for ReqlNoneTerm { type Args = (); type Output = (); }
	impl ReqlOrdering        for ReqlNoneTerm {}
	impl ReqlPathSpec        for ReqlNoneTerm {}
	impl ReqlErrorTy         for ReqlNoneTerm {}
	
	impl ReqlTerm for ReqlNoneTerm {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			write!(dst, "null")
		}
	}
}

mod cast {
	use super::*;
	
	pub struct ReqlOpCast<T: ReqlTerm>(T);
	
	impl<T: ReqlTerm> ReqlTop             for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlDatum           for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlNull            for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlBool            for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlNumber          for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlString          for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlObject          for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlSingleSelection for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlArray           for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlSequence        for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlStream          for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlStreamSelection for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlTable           for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlDatabase        for ReqlOpCast<T> {}
	//impl<T: ReqlTerm> ReqlFunction        for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlOrdering        for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlPathSpec        for ReqlOpCast<T> {}
	impl<T: ReqlTerm> ReqlErrorTy         for ReqlOpCast<T> {}
	
	impl<T: ReqlTerm> ReqlTerm for ReqlOpCast<T> {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			self.0.serialize(dst)
		}
	}
	
	pub trait ReqlCastInfix: ReqlTerm + Sized {
		fn cast(self) -> ReqlOpCast<Self> {
			ReqlOpCast(self)
		}
	}
	
	impl<T: ReqlTerm> ReqlCastInfix for T {}
	
	pub fn reql_to_string(buf: &mut Vec<u8>, term: impl ReqlTerm) {
		term.serialize(buf).unwrap();
	}
}

mod var {
	use super::*;
	
	#[derive(Debug, Default)]
	pub struct ReqlVar<T, const I: usize>(pub(crate) std::marker::PhantomData<T>);
	
	impl<T, const I: usize> Clone for ReqlVar<T, I> {
		fn clone(&self) -> Self {
			Self(std::marker::PhantomData)
		}
	}
	
	impl<T, const I: usize> Copy for ReqlVar<T, I> {}
	
	impl<T, const I: usize> ReqlTerm for ReqlVar<T, I> {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			write!(dst, "[10,[{}]]", I)?;
			Ok(())
		}
	}
	
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynTop, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynDatum, I> {}
	impl<const I: usize> ReqlDatum           for ReqlVar<ReqlDynDatum, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynNull, I> {}
	impl<const I: usize> ReqlDatum           for ReqlVar<ReqlDynNull, I> {}
	impl<const I: usize> ReqlNull            for ReqlVar<ReqlDynNull, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynBool, I> {}
	impl<const I: usize> ReqlDatum           for ReqlVar<ReqlDynBool, I> {}
	impl<const I: usize> ReqlBool            for ReqlVar<ReqlDynBool, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynNumber, I> {}
	impl<const I: usize> ReqlNumber          for ReqlVar<ReqlDynNumber, I> {}
	impl<const I: usize> ReqlDatum           for ReqlVar<ReqlDynNumber, I> {}
	impl<const I: usize> ReqlString          for ReqlVar<ReqlDynNumber, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynObject, I> {}
	impl<const I: usize> ReqlDatum           for ReqlVar<ReqlDynObject, I> {}
	impl<const I: usize> ReqlObject          for ReqlVar<ReqlDynObject, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynSingleSelection, I> {}
	impl<const I: usize> ReqlDatum           for ReqlVar<ReqlDynSingleSelection, I> {}
	impl<const I: usize> ReqlObject          for ReqlVar<ReqlDynSingleSelection, I> {}
	impl<const I: usize> ReqlSingleSelection for ReqlVar<ReqlDynSingleSelection, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynArray, I> {}
	impl<const I: usize> ReqlDatum           for ReqlVar<ReqlDynArray, I> {}
	impl<const I: usize> ReqlSequence        for ReqlVar<ReqlDynArray, I> {}
	impl<const I: usize> ReqlArray           for ReqlVar<ReqlDynArray, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynSequence, I> {}
	impl<const I: usize> ReqlSequence        for ReqlVar<ReqlDynSequence, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynStream, I> {}
	impl<const I: usize> ReqlSequence        for ReqlVar<ReqlDynStream, I> {}
	impl<const I: usize> ReqlStream          for ReqlVar<ReqlDynStream, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynStreamSelection, I> {}
	impl<const I: usize> ReqlSequence        for ReqlVar<ReqlDynStreamSelection, I> {}
	impl<const I: usize> ReqlStream          for ReqlVar<ReqlDynStreamSelection, I> {}
	impl<const I: usize> ReqlStreamSelection for ReqlVar<ReqlDynStreamSelection, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynTable, I> {}
	impl<const I: usize> ReqlSequence        for ReqlVar<ReqlDynTable, I> {}
	impl<const I: usize> ReqlStream          for ReqlVar<ReqlDynTable, I> {}
	impl<const I: usize> ReqlStreamSelection for ReqlVar<ReqlDynTable, I> {}
	impl<const I: usize> ReqlTable           for ReqlVar<ReqlDynTable, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynDatabase, I> {}
	impl<const I: usize> ReqlDatabase        for ReqlVar<ReqlDynDatabase, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynFunction, I> {}
	//impl<const I: usize> ReqlFunction        for ReqlVar<ReqlDynFunction, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynOrdering, I> {}
	impl<const I: usize> ReqlOrdering        for ReqlVar<ReqlDynOrdering, I> {}
	impl<const I: usize> ReqlTop             for ReqlVar<ReqlDynPathSpec, I> {}
	impl<const I: usize> ReqlPathSpec        for ReqlVar<ReqlDynPathSpec, I> {}
	impl<const I: usize> ReqlErrorTy         for ReqlVar<ReqlDynError, I> {}
}

mod var_args {
	use super::*;
	
	pub trait VarArgsSerializable {
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()>;
	}
	
	pub trait VarArgsTrait<T>: VarArgsSerializable {}
	
	#[derive(Copy, Clone, Debug, Default)]
	pub struct One<T: ReqlTerm>(T);
	
	impl<T: ReqlTop     > VarArgsTrait<ReqlDynTop>      for One<T> {}
	impl<T: ReqlDatum   > VarArgsTrait<ReqlDynDatum>    for One<T> {}
	impl<T: ReqlNull    > VarArgsTrait<ReqlDynNull>     for One<T> {}
	impl<T: ReqlBool    > VarArgsTrait<ReqlDynBool>     for One<T> {}
	impl<T: ReqlNumber  > VarArgsTrait<ReqlDynNumber>   for One<T> {}
	impl<T: ReqlString  > VarArgsTrait<ReqlDynString>   for One<T> {}
	impl<T: ReqlObject  > VarArgsTrait<ReqlDynObject>   for One<T> {}
	impl<T: ReqlArray   > VarArgsTrait<ReqlDynArray>    for One<T> {}
	impl<T: ReqlSequence> VarArgsTrait<ReqlDynSequence> for One<T> {}
	impl<T: ReqlPathSpec> VarArgsTrait<ReqlDynPathSpec> for One<T> {}
	
	impl<T: ReqlTerm> VarArgsSerializable for One<T> {
		#[allow(non_snake_case, unused_variables)]
		fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
			T::serialize(&self.0, dst)
		}
	}
	
	macro_rules! varargs {
		( $head:ident, ) => {};
		( $head:ident $(, $tail:ident )*, ) => {
			varargs!( $( $tail, )* );
		
			impl< $head: ReqlTop      $(, $tail: ReqlTop      )* > VarArgsTrait<ReqlDynTop>      for ( $head $(, $tail )* ) {}
			impl< $head: ReqlDatum    $(, $tail: ReqlDatum    )* > VarArgsTrait<ReqlDynDatum>    for ( $head $(, $tail )* ) {}
			impl< $head: ReqlNull     $(, $tail: ReqlNull     )* > VarArgsTrait<ReqlDynNull>     for ( $head $(, $tail )* ) {}
			impl< $head: ReqlBool     $(, $tail: ReqlBool     )* > VarArgsTrait<ReqlDynBool>     for ( $head $(, $tail )* ) {}
			impl< $head: ReqlNumber   $(, $tail: ReqlNumber   )* > VarArgsTrait<ReqlDynNumber>   for ( $head $(, $tail )* ) {}
			impl< $head: ReqlString   $(, $tail: ReqlString   )* > VarArgsTrait<ReqlDynString>   for ( $head $(, $tail )* ) {}
			impl< $head: ReqlObject   $(, $tail: ReqlObject   )* > VarArgsTrait<ReqlDynObject>   for ( $head $(, $tail )* ) {}
			impl< $head: ReqlArray    $(, $tail: ReqlArray    )* > VarArgsTrait<ReqlDynArray>    for ( $head $(, $tail )* ) {}
			impl< $head: ReqlSequence $(, $tail: ReqlSequence )* > VarArgsTrait<ReqlDynSequence> for ( $head $(, $tail )* ) {}
			impl< $head: ReqlPathSpec $(, $tail: ReqlPathSpec )* > VarArgsTrait<ReqlDynPathSpec> for ( $head $(, $tail )* ) {}
			
			impl< $head: ReqlTerm $(, $tail: ReqlTerm )* > VarArgsSerializable for ( $head $(, $tail )* ) {
				#[allow(non_snake_case, unused_variables)]
				fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
					let ( $head $(, $tail )* ) = self;
					$head.serialize(dst)?;
					
					$(
						dst.write_all(b",")?;
						$tail.serialize(dst)?;
					)*
					
					Ok(())
				}
			}
		};
	}
	
	varargs!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15, );
}

mod typed_fn {
	use super::*;
	
	pub trait ReqlTypedFunction<A, O>: ReqlTerm {}
	
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynTop> for F where F::Output: ReqlTop {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynDatum> for F where F::Output: ReqlDatum {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynNull> for F where F::Output: ReqlNull {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynBool> for F where F::Output: ReqlBool {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynNumber> for F where F::Output: ReqlNumber {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynString> for F where F::Output: ReqlString {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynObject> for F where F::Output: ReqlObject {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynSingleSelection> for F where F::Output: ReqlSingleSelection {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynArray> for F where F::Output: ReqlArray {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynSequence> for F where F::Output: ReqlSequence {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynStream> for F where F::Output: ReqlStream {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynStreamSelection> for F where F::Output: ReqlStreamSelection {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynTable> for F where F::Output: ReqlTable {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynDatabase> for F where F::Output: ReqlDatabase {}
	//impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynFunction> for F where F::Output: ReqlFunction {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynOrdering> for F where F::Output: ReqlOrdering {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynPathSpec> for F where F::Output: ReqlPathSpec {}
	impl<F: ReqlFunction> ReqlTypedFunction<F::Args, ReqlDynError> for F where F::Output: ReqlErrorTy {}
}

mod terms {
	#![allow(clippy::wrong_self_convention)]

	use {super::*, std::{marker::PhantomData, io::Result}};

	#[macro_export]
	macro_rules! reql_impl_traits {
		(impl $trait:ident for $struct_name:ident) => {
			impl_traits!(impl $trait for $struct_name<>);
		};
		(impl $trait:ident for $struct_name:ident< $( $arg_type:ident: $arg_trait:path ),+ >) => {
			impl_traits!(impl $trait for $struct_name< $( $arg_type: $arg_trait, )* >);
		};
		(impl ReqlTop for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlDatum for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlNull for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlNull for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlBool for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlBool for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlNumber for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlNumber for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlString for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlString for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlObject for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlObject for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlSingleSelection for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlObject for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlSingleSelection for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlArray for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatum for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlSequence for $struct_name < $( $arg_type, )* >{}
			impl< $( $arg_type: $arg_trait, )* > ReqlArray for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlSequence for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlSequence for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlStream for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlSequence for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlStream for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlStreamSelection for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlSequence for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlStream for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlStreamSelection for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlTable for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlSequence for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlStream for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlStreamSelection for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlTable for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlDatabase for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlDatabase for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlFunction for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlFunction for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlOrdering for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlOrdering for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlPathSpec for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlTop for $struct_name< $( $arg_type, )* > {}
			impl< $( $arg_type: $arg_trait, )* > ReqlPathSpec for $struct_name< $( $arg_type, )* > {}
		};
		(impl ReqlError for $struct_name:ident< $( $arg_type:ident: $arg_trait:path, )* >) => {
			impl< $( $arg_type: $arg_trait, )* > ReqlErrorTy for $struct_name< $( $arg_type, )* > {}
		};
	}
	
	macro_rules! function_impl {
	    ( $head_ty:ident, $head_idx:ident, $( $tail_ty:ident, $tail_idx:ident, )* ) => {
	    	function_impl!( $( $tail_ty, $tail_idx, )* );
	    	
	    	impl<
	    		T: Fn( ReqlVar<$head_ty, $head_idx>, $( ReqlVar<$tail_ty, $tail_idx>, )* ) -> O + Send + Sync + Unpin,
	    		O: ReqlTerm,
	    		$head_ty $(, $tail_ty )*,
	    		const $head_idx: usize $(, const $tail_idx: usize )*
			> ReqlTerm for (T, PhantomData<dyn Fn( ReqlVar<$head_ty, $head_idx>, $( ReqlVar<$tail_ty, $tail_idx>, )* ) -> O + Send + Sync + Unpin>) {
				fn serialize(&self, dst: &mut impl Write) -> Result<()> {
					write!(dst, concat!("[69,[[2,[{}", $( function_impl!(@ignore $tail_idx, ",{}"), )* "]],"), $head_idx $(, $tail_idx )* )?;
					self.0(ReqlVar(PhantomData) $(, ReqlVar(function_impl!(@ignore $tail_ty, PhantomData)) )* ).serialize(dst)?;
					write!(dst, "]]")?;
					Ok(())
				}
			}
			
			impl<
	    		T: Fn( ReqlVar<$head_ty, $head_idx>, $( ReqlVar<$tail_ty, $tail_idx>, )* ) -> O + Send + Sync + Unpin,
	    		O: ReqlTerm,
	    		$head_ty $(, $tail_ty )*,
	    		const $head_idx: usize $(, const $tail_idx: usize )*
			> ReqlTop for (T, PhantomData<dyn Fn( ReqlVar<$head_ty, $head_idx>, $( ReqlVar<$tail_ty, $tail_idx>, )* ) -> O + Send + Sync + Unpin>) {}
	    	
	    	#[allow(unused_parens)]
	    	impl<
	    		T: Fn( ReqlVar<$head_ty, $head_idx>, $( ReqlVar<$tail_ty, $tail_idx>, )* ) -> O + Send + Sync + Unpin,
	    		O: ReqlTerm,
	    		$head_ty $(, $tail_ty )*,
	    		const $head_idx: usize $(, const $tail_idx: usize )*
			> ReqlFunction for (T, PhantomData<dyn Fn( ReqlVar<$head_ty, $head_idx>, $( ReqlVar<$tail_ty, $tail_idx>, )* ) -> O + Send + Sync + Unpin>) {
				type Args = ($head_ty $(, $tail_ty )* );
				type Output = O;
			}
	    };
	    () => {};
	    (@ignore $tt:tt, $tt2:tt) => { $tt2 };
	}
	
	function_impl!(A0, A0I, A1, A1I, A2, A2I, A3, A3I, A4, A4I, A5, A5I, A6, A6I, A7, A7I, A8, A8I, A9, A9I, A10, A10I, A11, A11I, A12, A12I, A13, A13I, A14, A14I, A15, A15I, );
	
	macro_rules! term {
		// no args,  no optargs
		($struct_name:ident = $id:expr, fn $fn_name:ident() -> $out:ident) => {
			pub struct $struct_name;
			
			reql_impl_traits!(impl $out for $struct_name<>);
			
			impl ReqlTerm for $struct_name {
				fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
					write!(dst, "[{},[]]", $id)
				}
			}
			
			pub fn $fn_name() -> $struct_name { $struct_name }
		};
		// args, no optargs
		($struct_name:ident = $id:expr, $trait_name:ident, fn $fn_name:ident( $arg0_name:ident: $arg0_type:ident; $arg0_trait:path $(, $arg_name:ident: $arg_type:ident; $arg_trait:path )* ) -> $out:ident) => {
			pub struct $struct_name< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* >( $arg0_type, $( $arg_type, )* );
			
			reql_impl_traits!(impl $out for $struct_name< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* >);
			
			impl< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* > ReqlTerm for $struct_name< $arg0_type, $( $arg_type, )* > {
				fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
					let Self( $arg0_name, $( $arg_name, )* ) = self;
					write!(dst, "[{},[", $id)?;
					$arg0_name.serialize(dst)?;
					$(
						dst.write_all(b",")?;
						$arg_name.serialize(dst)?;
					)*
					dst.write_all(b"]]")
				}
			}
			
			pub trait $trait_name: $arg0_trait + Sized {
				fn $fn_name< $( $arg_type: $arg_trait, )* >(self, $( $arg_name: $arg_type, )* )
					-> $struct_name< Self, $( $arg_type, )* > {
					$struct_name(self, $( $arg_name, )* )
				}
			}
			
			impl<T: $arg0_trait> $trait_name for T {}
			
			pub fn $fn_name< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* >( $arg0_name: $arg0_type, $( $arg_name: $arg_type, )* )
				-> $struct_name< $arg0_type, $( $arg_type, )* > {
				$struct_name( $arg0_name, $( $arg_name, )* )
			}
		};
		// no args, optargs
		($struct_name:ident = $id:expr, $trait_name:ident, fn $fn_name:ident() $opts_name:ident { $( $optarg_name:ident: $optarg_type:ident; $optarg_trait:path, )+ } -> $out:ident) => {
			#[derive(Copy, Clone, Debug)]
			pub struct $struct_name< $( $optarg_type: $optarg_trait, )* >( $( $optarg_type )* );
			
			reql_impl_traits!(impl $out for $struct_name< $( $optarg_type: $optarg_trait, )* >);
			
			#[derive(Copy, Clone, Debug, Default)]
			pub struct $opts_name< $( $optarg_type: $optarg_trait, )* > {
				$( $optarg_name: $optarg_type, )*
			}
			
			impl< $( $optarg_type: $optarg_trait, )* > $struct_name< $( $optarg_type, )* > {
				pub fn options< $( $optarg_type: $optarg_trait, )* >(self, options: $opts_name< $( $optarg_type, )* >) -> $struct_name< $( $optarg_type, )* > {
					let $opts_name { $( $optarg_name: $optarg_type, )* } = options;
					Self( $( $optarg_name, )* )
				}
			}
			
			impl< $( $optarg_type: $optarg_trait, )* > ReqlTerm for $struct_name< $( $optarg_type, )* > {
				fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
					let Self($( $optarg_name, )* ) = self;
					
					if false $( || $optarg_name.is_some() )* {
						write!(dst, "[{},[],{", $id)?;
						let mut buf = Vec::new();
						
						$(
							if let Some($optarg_name) = $optarg_name {
								buf.write_all(b"\"")?;
								buf.write_all(stringify!($optarg_name).as_bytes())?;
								buf.write_all(b"\":")?;
								$optarg_name.serialize(&mut buf)?;
								buf.write_all(b",")?;
							}
						)*
						
						if !buf.is_empty() {
							buf.pop();
							dst.write_all(&buf)?;
						}
						
						dst.write_all(b"}]")
					} else {
						write!(dst, "[{},[]]", $id)
					}
				}
			}
			
			pub fn $fn_name() -> $struct_name< $( term!(@ignore $optarg_name; NoneTerm), )* > {
				$struct_name( $( term!(@ignore $optarg_name; None), )* )
			}
		};
		// args, optargs
		($struct_name:ident = $id:expr, $trait_name:ident, fn $fn_name:ident( $arg0_name:ident: $arg0_type:ident; $arg0_trait:path $(, $arg_name:ident: $arg_type:ident; $arg_trait:path )* ) $opts_name:ident { $( $optarg_name:ident: $optarg_type:ident; $optarg_trait:path, )+ } -> $out:ident) => {
			#[derive(Copy, Clone, Debug)]
			pub struct $struct_name< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* $( $optarg_type: $optarg_trait, )* >( $arg0_type, $( $arg_type, )* $( Option<$optarg_type>, )* );
			
			reql_impl_traits!(impl $out for $struct_name< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* $( $optarg_type: $optarg_trait, )* >);
			
			#[derive(Copy, Clone, Debug, Default)]
			pub struct $opts_name< $( $optarg_type: $optarg_trait, )* > {
				$( $optarg_name: $optarg_type, )*
			}
			
			impl< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* $( $optarg_type: $optarg_trait, )* > ReqlTerm for $struct_name< $arg0_type, $( $arg_type, )* $( $optarg_type, )* > {
				fn serialize(&self, dst: &mut impl Write) -> std::io::Result<()> {
					let Self( $arg0_name, $( $arg_name, )* $( $optarg_name, )* ) = self;
					write!(dst, "[{},[", $id)?;
					$arg0_name.serialize(dst)?;
					$(
						dst.write_all(b",")?;
						$arg_name.serialize(dst)?;
					)*
					
					if false $( || $optarg_name.is_some() )* {
						dst.write_all(b"],{")?;
						let mut buf = Vec::new();
						
						$(
							if let Some($optarg_name) = $optarg_name {
								buf.write_all(b"\"")?;
								buf.write_all(stringify!($optarg_name).as_bytes())?;
								buf.write_all(b"\":")?;
								$optarg_name.serialize(&mut buf)?;
								buf.write_all(b",")?;
							}
						)*
						
						if !Vec::is_empty(&buf) {
							buf.pop();
							dst.write_all(&buf)?;
						}
						
						dst.write_all(b"}]")
					} else {
						dst.write_all(b"]]")
					}
				}
			}
			
			pub trait $trait_name: $arg0_trait + Sized {
				fn $fn_name< $( $arg_type: $arg_trait, )* >(self, $( $arg_name: $arg_type, )* )
					-> $struct_name< Self, $( $arg_type, )* $( term!(@ignore $optarg_name; ReqlNoneTerm), )* > {
					$struct_name(self, $( $arg_name, )* $( term!(@ignore $optarg_name; None), )* )
				}
			}
			
			impl<T: $arg0_trait> $trait_name for T {}
			
			pub fn $fn_name< $arg0_type: $arg0_trait, $( $arg_type: $arg_trait, )* >( $arg0_name: $arg0_type, $( $arg_name: $arg_type, )* )
				-> $struct_name< $arg0_type, $( $arg_type, )* $( term!(@ignore $optarg_name; ReqlNoneTerm), )* > {
				$struct_name( $arg0_name, $( $arg_name, )* $( term!(@ignore $optarg_name; None), )* )
			}
			
		};
		(@ignore $tt:tt; $tt2:tt) => { $tt2 };
	}
	
	term!(ReqlOpVar              =  10, ReqlVarInfix,              fn var(r: R; ReqlNumber) -> ReqlDatum);
	term!(ReqlOpJavaScript       =  11, ReqlJavaScriptInfix,       fn javascript(code: C; ReqlString) JavaScriptOptions { timeout: T; ReqlNumber, } -> ReqlDatum);
	term!(ReqlOpError            =  12,                            fn error() -> ReqlError);
	term!(ReqlOpErrorFromString  =  12, ReqlErrorFromStringInfix,  fn error_from_string(s: S; ReqlString) -> ReqlError);
	term!(ReqlOpImplicitVar      =  13,                            fn implicit_var() -> ReqlDatum);
	term!(ReqlOpDb               =  14, ReqlDbInfix,               fn db(s: S; ReqlString) -> ReqlDatabase);
	term!(ReqlOpTable            =  15, ReqlTableInfix,            fn table(db: DB; ReqlDatabase, s: S; ReqlString) TableOptions { read_mode: RM; ReqlString, id_format: IF; ReqlString, } -> ReqlTable);
	term!(ReqlOpGetStr           =  16, ReqlGetStrInfix,           fn get_str(t: T; ReqlTable, s: S; ReqlString) -> ReqlSingleSelection);
	term!(ReqlOpGetNum           =  16, ReqlGetNumInfix,           fn get_num(t: T; ReqlTable, n: N; ReqlNumber) -> ReqlSingleSelection);
	term!(ReqlOpEq               =  17, ReqlEqInfix,               fn eq(a: A; ReqlDatum, b: B; ReqlDatum) -> ReqlBool);
	term!(ReqlOpNe               =  18, ReqlNeInfix,               fn ne(a: A; ReqlDatum, b: B; ReqlDatum) -> ReqlBool);
	term!(ReqlOpLt               =  19, ReqlLtInfix,               fn lt(a: A; ReqlDatum, b: B; ReqlDatum) -> ReqlBool);
	term!(ReqlOpLe               =  20, ReqlLeInfix,               fn le(a: A; ReqlDatum, b: B; ReqlDatum) -> ReqlBool);
	term!(ReqlOpGt               =  21, ReqlGtInfix,               fn gt(a: A; ReqlDatum, b: B; ReqlDatum) -> ReqlBool);
	term!(ReqlOpGe               =  22, ReqlGeInfix,               fn ge(a: A; ReqlDatum, b: B; ReqlDatum) -> ReqlBool);
	term!(ReqlOpNot              =  23, ReqlNotInfix,              fn not(b: B; ReqlBool) -> ReqlBool);
	term!(ReqlOpAdd              =  24, ReqlAddInfix,              fn add(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpSub              =  25, ReqlSubInfix,              fn sub(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpMul              =  26, ReqlMulInfix,              fn mul(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpDiv              =  27, ReqlDivInfix,              fn div(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpMod              =  28, ReqlModInfix,              fn r#mod(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpAppend           =  29, ReqlAppendInfix,           fn append(a: A; ReqlArray, v: V; ReqlDatum) -> ReqlArray);
	term!(ReqlOpSlice            =  30, ReqlSliceInfix,            fn slice(s: S; ReqlSequence, from: F; ReqlNumber, to: T; ReqlNumber) -> ReqlSequence);
	term!(ReqlOpGetField         =  31, ReqlGetFieldInfix,         fn get_field(o: O; ReqlObject, s: S; ReqlString) -> ReqlDatum);
	term!(ReqlOpHasFields        =  32, ReqlHasFieldsInfix,        fn has_fields(o: O; ReqlObject, p: P; VarArgsTrait<ReqlDynPathSpec>) -> ReqlBool);
	term!(ReqlOpPluck            =  33, ReqlPluckInfix,            fn pluck(s: S; ReqlSequence, p: P; VarArgsTrait<ReqlDynPathSpec>) -> ReqlSequence);
	term!(ReqlOpPluckObj         =  33, ReqlPluckObjInfix,         fn pluck_obj(o: O; ReqlObject, p: P; VarArgsTrait<ReqlDynPathSpec>) -> ReqlObject);
	term!(ReqlOpWithout          =  34, ReqlWithoutInfix,          fn without(s: S; ReqlSequence, p: P; VarArgsTrait<ReqlDynPathSpec>) -> ReqlSequence);
	term!(ReqlOpWithoutObj       =  34, ReqlWithoutObjInfix,       fn without_obj(o: O; ReqlObject, p: P; VarArgsTrait<ReqlDynPathSpec>) -> ReqlObject);
	term!(ReqlOpMerge            =  35, ReqlMergeInfix,            fn merge(o: O; VarArgsTrait<ReqlDynObject>) -> ReqlObject);
	term!(ReqlOpMergeWith        =  35, ReqlMergeWithInfix,        fn merge_with(o: O; ReqlObject, f: G; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlObject);
	term!(ReqlOpMergeSeq         =  35, ReqlMergeSeqInfix,         fn merge_seq(s: S; ReqlSequence) -> ReqlSequence);
	term!(ReqlOpMergeSeqWith     =  35, ReqlMergeSeqWithInfix,     fn merge_seq_with(s: S; ReqlSequence, f: G; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlSequence);
	term!(ReqlOpReduce           =  37, ReqlReduceInfix,           fn reduce(s: S; ReqlSequence, f: F; ReqlTypedFunction<(ReqlDynDatum, ReqlDynDatum), ReqlDynDatum>) -> ReqlDatum);
	term!(ReqlOpMap              =  38, ReqlMapInfix,              fn map(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlSequence);
	term!(ReqlOpFilter           =  39, ReqlFilterInfix,           fn filter(s: S; ReqlSequence, o: O; ReqlObject) -> ReqlSequence);
	term!(ReqlOpFilterWith       =  39, ReqlFilterWithInfix,       fn filter_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynBool>) -> ReqlSequence);
	term!(ReqlOpConcatMap        =  40, ReqlConcatMapInfix,        fn concat_map(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynSequence>) -> ReqlSequence);
	term!(ReqlOpOrderBy          =  41, ReqlOrderByInfix,          fn order_by(s: S; ReqlSequence, o: O; ReqlOrdering) OrderByOptions { index: I; ReqlOrdering, } -> ReqlSequence);
	term!(ReqlOpDistinct         =  42, ReqlDistinctInfix,         fn distinct(s: S; ReqlSequence) -> ReqlSequence);
	term!(ReqlOpCount            =  43, ReqlCountInfix,            fn count(s: S; ReqlSequence) -> ReqlNumber);
	term!(ReqlOpCountDatum       =  43, ReqlCountDatumInfix,       fn count_datum(s: S; ReqlSequence, v: V; ReqlDatum) -> ReqlNumber);
	term!(ReqlOpCountWith        =  43, ReqlCountWithInfix,        fn count_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynBool>) -> ReqlNumber);
	term!(ReqlOpUnion            =  44, ReqlUnionInfix,            fn union(s: S; VarArgsTrait<ReqlDynSequence>) -> ReqlSequence);
	term!(ReqlOpNth              =  45, ReqlNthInfix,              fn nth(s: S; ReqlSequence, n: N; ReqlNumber) -> ReqlDatum);
	term!(ReqlOpInnerJoin        =  48, ReqlInnerJoinInfix,        fn inner_join(s1: S1; ReqlSequence, s2: S2; ReqlSequence, f: F; ReqlTypedFunction<(ReqlDynDatum, ReqlDynDatum), ReqlDynBool>) -> ReqlSequence);
	term!(ReqlOpOuterJoin        =  49, ReqlOuterJoinInfix,        fn outer_join(s1: S1; ReqlSequence, s2: S2; ReqlSequence, f: F; ReqlTypedFunction<(ReqlDynDatum, ReqlDynDatum), ReqlDynBool>) -> ReqlSequence);
	term!(ReqlOpEqJoin           =  50, ReqlEqJoinInfix,           fn eq_join(s1: S1; ReqlSequence, s: S; ReqlString, s2: S2; ReqlSequence) EqJoinOptions { index: I; ReqlString, } -> ReqlSequence);
	term!(ReqlOpCoerceTo         =  51, ReqlCoerceToInfix,         fn coerce_to(v: V; ReqlTop, t: T; ReqlString) -> ReqlTop);
	term!(ReqlOpTypeOf           =  52, ReqlTypeOfInfix,           fn type_of(v: V; ReqlTop) -> ReqlString);
	term!(ReqlOpUpdateWith       =  53, ReqlUpdateWithInfix,       fn update_with(s: S; ReqlStreamSelection, f: F; ReqlTypedFunction<ReqlDynObject, ReqlDynObject>) UpdateWithOptions { non_atomic: A; ReqlBool, durability: D; ReqlString, return_changes: C; ReqlBool, } -> ReqlObject);
	term!(ReqlOpUpdateOneWith    =  53, ReqlUpdateOneWithInfix,    fn update_one_with(s: S; ReqlSingleSelection, f: F; ReqlTypedFunction<ReqlDynObject, ReqlDynObject>) UpdateOneWithOptions { non_atomic: A; ReqlBool, durability: D; ReqlString, return_changes: C; ReqlBool, } -> ReqlObject);
	term!(ReqlOpUpdate           =  53, ReqlUpdateInfix,           fn update(s: S; ReqlStreamSelection, o: O; ReqlObject) UpdateOptions { non_atomic: A; ReqlBool, durability: D; ReqlString, return_changes: C; ReqlBool, } -> ReqlObject);
	term!(ReqlOpUpdateOne        =  53, ReqlUpdateOneInfix,        fn update_one(s: S; ReqlSingleSelection, o: O; ReqlObject) UpdateOneOptions { non_atomic: A; ReqlBool, durability: D; ReqlString, return_changes: C; ReqlBool, } -> ReqlObject);
	term!(ReqlOpDelete           =  54, ReqlDeleteInfix,           fn delete(s: S; ReqlStreamSelection) DeleteOptions { durability: D; ReqlString, return_cahnges: C; ReqlBool, } -> ReqlObject);
	term!(ReqlOpDeleteOne        =  54, ReqlDeleteOneInfix,        fn delete_one(s: S; ReqlSingleSelection) -> ReqlObject);
	term!(ReqlOpReplace          =  55, ReqlReplaceInfix,          fn reaplce(s: S; ReqlStreamSelection, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) ReplaceOptions { non_atomic: A; ReqlBool, durability: D; ReqlString, return_changes: C; ReqlBool, } -> ReqlObject);
	term!(ReqlOpReplaceOne       =  55, ReqlReplaceOneInfix,       fn replace_one(s: S; ReqlSingleSelection, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) ReplaceOneOptions { non_atomic: A; ReqlBool, durability: D; ReqlString, return_changes: C; ReqlBool, } -> ReqlObject);
	term!(ReqlOpInsert           =  56, ReqlInsertInfix,           fn insert(t: T; ReqlTable, o: O; ReqlObject) InsertOptions { conflict: C; ReqlString, durability: D; ReqlString, return_changes: R; ReqlBool, } -> ReqlObject);
	term!(ReqlOpInsertSeq        =  56, ReqlInsertSeqInfix,        fn insert_seq(t: T; ReqlTable, s: S; ReqlSequence) InsertSeqOptions { conflict: C; ReqlString, durability: D; ReqlString, return_changes: R; ReqlBool, } -> ReqlObject);
	term!(ReqlOpDbCreate         =  57, ReqlDbCreateInfix,         fn db_create(name: S; ReqlString) -> ReqlObject);
	term!(ReqlOpDbDrop           =  58, ReqlDbDropInfix,           fn db_drop(name: S; ReqlString) -> ReqlObject);
	term!(ReqlOpDbList           =  59,                            fn db_list() -> ReqlArray);
	term!(ReqlOpTableCreate      =  60, ReqlTableCreateInfix,      fn table_create(name: S; ReqlString) TableCreateOptions { primary_key: PK; ReqlString, shards: SH; ReqlNumber, replicas: RE; ReqlDatum, primary_replica_tag: PR; ReqlString, } -> ReqlObject);
	term!(ReqlOpTableCreateDb    =  60, ReqlTableCreateDbInfix,    fn table_create_db(db: DB; ReqlDatabase, name: S; ReqlString) TableCreateDbOptions { primary_key: PK; ReqlString, shards: SH; ReqlNumber, replicas: RE; ReqlDatum, primary_replica_tag: PR; ReqlString, } -> ReqlObject);
	term!(ReqlOpTableDrop        =  61, ReqlTableDropInfix,        fn table_drop(name: S; ReqlString) -> ReqlObject);
	term!(ReqlOpTableDropDb      =  61, ReqlTableDropDbInfix,      fn table_drop_db(db: DB; ReqlDatabase, name: S; ReqlString) -> ReqlObject);
	term!(ReqlOpTableList        =  62,                            fn table_list() -> ReqlArray);
	term!(ReqlOpTableListDb      =  62, ReqlTableListDbInfix,      fn table_list_db(db: DB; ReqlDatabase) -> ReqlArray);
	//term!(ReqlOpFunCall          =  64, ReqlFunCallInfix,          fn funcall(f: F; ReqlTypedFunction, data: D; VarArgsTrait<ReqlDynDatum>) -> ReqlDatum);
	term!(ReqlOpBranch           =  65, ReqlBranchInfix,           fn branch(c: C; ReqlBool, a: A; ReqlTop, b: B; ReqlTop) -> ReqlTop);
	term!(ReqlOpOr               =  66, ReqlOrInfix,               fn or(v: V; VarArgsTrait<ReqlDynBool>) -> ReqlBool);
	term!(ReqlOpAnd              =  67, ReqlAndInfix,              fn and(v: V; VarArgsTrait<ReqlDynBool>) -> ReqlBool);
	term!(ReqlOpForEach          =  68, ReqlForEachInfix,          fn for_each(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ()>) -> ReqlObject);
	term!(ReqlOpFunc             =  69, ReqlFuncInfix,             fn func(args: A; ReqlArray, t: T; ReqlTop) -> ReqlTop);
	term!(ReqlOpSkip             =  70, ReqlSkipInfix,             fn skip(s: S; ReqlSequence, v: V; ReqlNumber) -> ReqlSequence);
	term!(ReqlOpLimit            =  71, ReqlLimitInfix,            fn limit(s: S; ReqlSequence, v: V; ReqlNumber) -> ReqlSequence);
	term!(ReqlOpZip              =  72, ReqlZipInfix,              fn zip(s: S; ReqlSequence) -> ReqlSequence);
	term!(ReqlOpAsc              =  73, ReqlAscInfix,              fn asc(s: S; ReqlString) -> ReqlOrdering);
	term!(ReqlOpDesc             =  74, ReqlDescInfix,             fn desc(s: S; ReqlString) -> ReqlOrdering);
	term!(ReqlOpIndexCreate      =  75, ReqlIndexCreateInfix,      fn index_create(t: T; ReqlTable, s: S; ReqlString, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) IndexCreateOptions { multi: M; ReqlBool, } -> ReqlObject);
	term!(ReqlOpIndexDrop        =  76, ReqlIndexDropInfix,        fn index_drop(t: T; ReqlTable, s: S; ReqlString) -> ReqlObject);
	term!(ReqlOpIndexList        =  77, ReqlIndexListInfix,        fn index_list(t: T; ReqlTable) -> ReqlArray);
	term!(ReqlOpGetAll           =  78, ReqlGetAllInfix,           fn get_all(t: T; ReqlTable, d: D; VarArgsTrait<ReqlDynDatum>) -> ReqlArray);
	term!(ReqlOpInfo             =  79, ReqlInfoInfix,             fn info(v: T; ReqlTop) -> ReqlObject);
	term!(ReqlOpPrepend          =  80, ReqlPrependInfix,          fn prepend(a: A; ReqlArray, v: V; ReqlDatum) -> ReqlArray);
	term!(ReqlOpSample           =  81, ReqlSampleInfix,           fn sample(s: S; ReqlSequence, n: N; ReqlNumber) -> ReqlSequence);
	term!(ReqlOpInsertAt         =  82, ReqlInsertAtInfix,         fn insert_at(a: A; ReqlArray, i: I; ReqlNumber, v: V; ReqlDatum) -> ReqlArray);
	term!(ReqlOpDeleteAt         =  83, ReqlDeleteAtInfix,         fn delete_at(a: A; ReqlArray, i: I; ReqlNumber) -> ReqlArray);
	term!(ReqlOpDeleteRange      =  83, ReqlDeleteRangeInfix,      fn delete_range(a: A; ReqlArray, o: O; ReqlNumber, e: E; ReqlNumber) -> ReqlArray);
	term!(ReqlOpChangeAt         =  84, ReqlChangeAtInfix,         fn change_at(a: A; ReqlArray, i: I; ReqlNumber, v: V; ReqlDatum) -> ReqlArray);
	term!(ReqlOpSpliceAt         =  85, ReqlSpliceAtInfix,         fn splice_at(a: A; ReqlArray, i: I; ReqlNumber, v: V; ReqlArray) -> ReqlArray);
	term!(ReqlOpIsEmpty          =  86, ReqlIsEmptyInfix,          fn is_empty(s: S; ReqlSequence) -> ReqlBool);
	term!(ReqlOpOffsetOf         =  87, ReqlOffsetOfInfix,         fn offset_of(s: S; ReqlSequence, v: V; ReqlDatum) -> ReqlSequence);
	term!(ReqlOpOffsetOfWith     =  87, ReqlOffsetOfWithInfix,     fn offset_of_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynBool>) -> ReqlSequence);
	term!(ReqlOpSetInsert        =  88, ReqlSetInsertInfix,        fn set_insert(a: A; ReqlArray, v: V; ReqlDatum) -> ReqlArray);
	term!(ReqlOpSetIntersection  =  89, ReqlSetIntersectionInfix,  fn set_intersection(a: A; ReqlArray, b: B; ReqlArray) -> ReqlArray);
	term!(ReqlOpSetUnion         =  90, ReqlSetUnionInfix,         fn set_union(a: A; ReqlArray, b: B; ReqlArray) -> ReqlArray);
	term!(ReqlOpSetDifference    =  91, ReqlSetDifferenceInfix,    fn set_difference(a: A; ReqlArray, b: B; ReqlArray) -> ReqlArray);
	term!(ReqlOpDefault          =  92, ReqlDefaultInfix,          fn default_(a: A; ReqlTop, b: B; ReqlTop) -> ReqlTop);
	term!(ReqlOpContains         =  93, ReqlContainsInfix,         fn contains(s: S; ReqlSequence, v: V; VarArgsTrait<ReqlDynDatum>) -> ReqlBool);
	term!(ReqlOpContainsWith     =  93, ReqlContainsWithInfix,     fn contains_with(s: S; ReqlSequence, v: V; VarArgsTrait<ReqlDynFunction>) -> ReqlBool);
	term!(ReqlOpKeys             =  94, ReqlKeysInfix,             fn keys(o: O; ReqlObject) -> ReqlArray);
	term!(ReqlOpDifference       =  95, ReqlDifferenceInfix,       fn difference(a: A; ReqlArray, b: B; ReqlArray) -> ReqlArray);
	term!(ReqlOpWithFields       =  96, ReqlWithFieldsInfix,       fn with_fields(s: S; ReqlSequence, p: P; VarArgsTrait<ReqlDynPathSpec>) -> ReqlSequence);
	term!(ReqlOpMatch            =  97, ReqlMathInfix,             fn matches(s: S; ReqlString, pattern: P; ReqlString) -> ReqlDatum);
	term!(ReqlOpJson             =  98, ReqlJsonInfix,             fn json(s: S; ReqlString) -> ReqlDatum);
	term!(ReqlOpIso8601          =  99, ReqlIso8601Infix,          fn iso8601(s: S; ReqlString) -> ReqlDatum);
	term!(ReqlOpToIso8601        = 100, ReqlToIso8601Infix,        fn to_iso8601(v: T; ReqlDatum) -> ReqlString);
	term!(ReqlOpEpochTime        = 101, ReqlEpochTimeInfix,        fn epoch_time(n: N; ReqlNumber) -> ReqlDatum);
	term!(ReqlOpToEpochTime      = 102, ReqlToEpochTimeInfix,      fn to_epoch_time(v: T; ReqlDatum) -> ReqlNumber);
	term!(ReqlOpNow              = 103,                            fn now() -> ReqlDatum);
	term!(ReqlOpInTimezone       = 104, ReqlInTimezoneInfix,       fn in_timezone(v: T; ReqlDatum, tz: TZ; ReqlString) -> ReqlDatum);
	term!(ReqlOpDuring           = 105, ReqlDuringInfix,           fn during(a: A; ReqlDatum, b: B; ReqlDatum, c: C; ReqlDatum) -> ReqlBool);
	term!(ReqlOpDate             = 106, ReqlDateInfix,             fn date(d: D; ReqlDatum) -> ReqlDatum);
	term!(ReqlOpMonday           = 107,                            fn monday() -> ReqlNumber);
	term!(ReqlOpTuesday          = 108,                            fn tuesday() -> ReqlNumber);
	term!(ReqlOpWednesday        = 109,                            fn wednesday() -> ReqlNumber);
	term!(ReqlOpThursday         = 110,                            fn thursday() -> ReqlNumber);
	term!(ReqlOpFriday           = 111,                            fn friday() -> ReqlNumber);
	term!(ReqlOpSaturday         = 112,                            fn saturday() -> ReqlNumber);
	term!(ReqlOpSunday           = 113,                            fn sunday() -> ReqlNumber);
	term!(ReqlOpJanuary          = 114,                            fn january() -> ReqlNumber);
	term!(ReqlOpFebruary         = 115,                            fn february() -> ReqlNumber);
	term!(ReqlOpMarch            = 116,                            fn march() -> ReqlNumber);
	term!(ReqlOpApril            = 117,                            fn april() -> ReqlNumber);
	term!(ReqlOpMay              = 118,                            fn may() -> ReqlNumber);
	term!(ReqlOpJune             = 119,                            fn june() -> ReqlNumber);
	term!(ReqlOpJuly             = 120,                            fn july() -> ReqlNumber);
	term!(ReqlOpAugust           = 121,                            fn august() -> ReqlNumber);
	term!(ReqlOpSeptember        = 122,                            fn september() -> ReqlNumber);
	term!(ReqlOpOctober          = 123,                            fn october() -> ReqlNumber);
	term!(ReqlOpNovember         = 124,                            fn november() -> ReqlNumber);
	term!(ReqlOpDecember         = 125,                            fn december() -> ReqlNumber);
	term!(ReqlOpTimeOfDay        = 126, ReqlTimeOfDayInfix,        fn time_of_day(t: T; ReqlDatum) -> ReqlNumber);
	term!(ReqlOpTimezone         = 127, ReqlTimezoneInfix,         fn timezone(t: T; ReqlDatum) -> ReqlString);
	term!(ReqlOpYear             = 128,                            fn year() -> ReqlNumber);
	term!(ReqlOpMonth            = 129,                            fn month() -> ReqlNumber);
	term!(ReqlOpDay              = 130,                            fn day() -> ReqlNumber);
	term!(ReqlOpDayOfWeek        = 131,                            fn dayofweek() -> ReqlNumber);
	term!(ReqlOpDayOfYear        = 132,                            fn dayofyear() -> ReqlNumber);
	term!(ReqlOpHours            = 133,                            fn hours() -> ReqlNumber);
	term!(ReqlOpMinutes          = 134,                            fn minutes() -> ReqlNumber);
	term!(ReqlOpSeconds          = 135,                            fn seconds() -> ReqlNumber);
	term!(ReqlOpToDate           = 136, ReqlToDateInfix,           fn to_date(year: Y; ReqlNumber, month: M; ReqlNumber, day: D; ReqlNumber, timezone: TZ; ReqlString) -> ReqlDatum);
	term!(ReqlOpToDateTime       = 136, ReqlToDateTimeInfix,       fn to_date_time(year: Y; ReqlNumber, month: M; ReqlNumber, day: D; ReqlNumber, hour: H; ReqlNumber, minute: MI; ReqlNumber, second: S; ReqlNumber, timezone: TZ; ReqlString) -> ReqlDatum);
	term!(ReqlOpLiteral          = 137,                            fn literal() -> ReqlTop);
	term!(ReqlOpLiteralJson      = 137, ReqlLiteralJsonInfix,      fn literal_json(s: S; ReqlString) -> ReqlTop);
	term!(ReqlOpSync             = 138, ReqlSyncInfix,             fn sync(t: T; ReqlTable) -> ReqlObject);
	term!(ReqlOpIndexStatus      = 139, ReqlIndexStatusInfix,      fn index_status(t: T; ReqlTable, indexes: I; VarArgsTrait<String>) -> ReqlArray);
	term!(ReqlOpIndexWait        = 140, ReqlIndexWaitInfix,        fn index_wait(t: T; ReqlTable, indexes: I; VarArgsTrait<String>) -> ReqlArray);
	term!(ReqlOpToUpperCase      = 141, ReqlToUpperCaseInfix,      fn to_uppercase(s: S; ReqlString) -> ReqlString);
	term!(ReqlOpToLowerCase      = 142, ReqlToLowerCaseInfix,      fn to_lowercase(s: S; ReqlString) -> ReqlString);
	term!(ReqlOpCreateObject     = 143, ReqlCreateObjectInfix,     fn create_object(s: S; ReqlString, d: D; VarArgsTrait<ReqlDynDatum>) -> ReqlObject);
	term!(ReqlOpGroup            = 144, ReqlGroupInfix,            fn group(s: S; ReqlSequence, v: V; ReqlString) -> ReqlSequence);
	term!(ReqlOpGroupWith        = 144, ReqlGroupWithInfix,        fn group_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlSequence);
	term!(ReqlOpSum              = 145, ReqlSumInfix,              fn sum(s: S; ReqlSequence, v: V; ReqlString) -> ReqlSequence);
	term!(ReqlOpSumWith          = 145, ReqlSumWithInfix,          fn sum_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlSequence);
	term!(ReqlOpAvg              = 146, ReqlAvgInfix,              fn avg(s: S; ReqlSequence, v: V; ReqlString) -> ReqlSequence);
	term!(ReqlOpAvgWith          = 146, ReqlAvgWithInfix,          fn avg_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlSequence);
	term!(ReqlOpMin              = 147, ReqlMinInfix,              fn min(s: S; ReqlSequence, v: V; ReqlString) -> ReqlSequence);
	term!(ReqlOpMinWith          = 147, ReqlMinWithInfix,          fn min_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlSequence);
	term!(ReqlOpMax              = 148, ReqlMaxInfix,              fn max(s: S; ReqlSequence, v: V; ReqlString) -> ReqlSequence);
	term!(ReqlOpMaxWith          = 148, ReqlMaxWithInfix,          fn max_with(s: S; ReqlSequence, f: F; ReqlTypedFunction<ReqlDynDatum, ReqlDynDatum>) -> ReqlSequence);
	term!(ReqlOpSplitWhitespace  = 149, ReqlSplitWhitespaceInfix,  fn split_whitespace(s: S; ReqlString) -> ReqlArray);
	term!(ReqlOpSplit            = 149, ReqlSplitInfix,            fn split(s: S; ReqlString, p: P; ReqlString) -> ReqlArray);
	term!(ReqlOpSplitN           = 149, ReqlSplitNInfix,           fn splitn(s: S; ReqlString, p: P; ReqlString, n: N; ReqlNumber) -> ReqlArray);
	term!(ReqlOpSplitNWhitespace = 149, ReqlSplitNWhitespaceInfix, fn splitn_whitespace(s: S; ReqlString, null: NULL; ReqlNull, n: N; ReqlNumber) -> ReqlArray);
	term!(ReqlOpUngroup          = 150, ReqlUngroupInfix,          fn ungroup(v: T; ReqlTop) -> ReqlArray);
	term!(ReqlOpRandom           = 151, ReqlRandomInfix,           fn random(start: S; ReqlNumber, end: E; ReqlNumber) RandomOptions { float: F; ReqlBool, } -> ReqlDatum);
	term!(ReqlOpChanges          = 152, ReqlChangesInfix,          fn changes(t: T; ReqlTable) -> ReqlStream);
	term!(ReqlOpHttp             = 153, ReqlHttpInfix,             fn http(url: T; ReqlString) -> ReqlString);
	term!(ReqlOpArgs             = 154, ReqlArgsInfix,             fn args(a: A; ReqlArray) -> ReqlTop);
	
	term!(ReqlOplUuid            = 169,                            fn uuid() -> ReqlDatum);
	
	term!(ReqlOpConfigDb         = 174, ReqlConfigDbInfix,         fn db_config(db: DB; ReqlDatabase) -> ReqlObject);
	term!(ReqlOpConfigTable      = 174, ReqlConfigTableInfix,      fn table_config(t: T; ReqlTable) -> ReqlObject);
	term!(ReqlOpStatusDb         = 175, ReqlStatusDbInfix,         fn db_status(db: DB; ReqlDatabase) -> ReqlObject);
	term!(ReqlOpStatusTable      = 175, ReqlStatusTableInfix,      fn table_status(t: T; ReqlTable) -> ReqlObject);
	
	term!(ReqlOpWaitDb           = 177, ReqlWaitDbInfix,           fn db_wait(db: DB; ReqlDatabase) -> ReqlObject);
	term!(ReqlOpWaitTable        = 177, ReqlWaitTableInfix,        fn table_wait(t: T; ReqlTable) -> ReqlObject);
	term!(ReqlOpRebalanceDb      = 179, ReqlRebalanceDbInfix,      fn db_rebalance(db: DB; ReqlDatabase) -> ReqlObject);
	term!(ReqlOpRebalanceTable   = 179, ReqlRebalanceTableInfix,   fn table_rebalance(t: T; ReqlTable) -> ReqlObject);
	term!(ReqlOpMinVal           = 180,                            fn minval() -> ReqlNumber);
	term!(ReqlOpMaxVal           = 181,                            fn maxval() -> ReqlNumber);
	term!(ReqlOpBetween          = 182, ReqlBetweenInfix,          fn between(s: S; ReqlStreamSelection, a: A; ReqlDatum, b: B; ReqlDatum) BetweenOptions { index: I; ReqlString, right_bound: R; ReqlString, left_bound: L; ReqlString, } -> ReqlStreamSelection);
	term!(ReqlOpFloor            = 183, ReqlFloorInfix,            fn floor(v: T; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpCeil             = 184, ReqlCeilInfix,             fn ceil(v: T; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpRound            = 185, ReqlRoundInfix,            fn round(v: T; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpValues           = 186, ReqlValuesInfix,           fn values(o: O; ReqlObject) -> ReqlArray);
	//term!(ReqlOpFold             = 187, ReqlFoldInfix,             fn fold(s: S; ReqlSequence, base: B; ReqlDatum, f: F; ReqlTypedFunction<(ReqlDynDatum, ReqlDynDatum), ReqlDynDatum>) FoldOptions { emit: E; ReqlTypedFunction<(ReqlDynDatum, ReqlDynDatum, ReqlDynDatum), ReqlDynArray>, final_emit: FE; ReqlTypedFunction<(ReqlDynDatum), ReqlDynArray>, } -> ReqlSequence);
	term!(ReqlOpGrant            = 188,                            fn grant() -> ReqlObject);
	term!(ReqlOpGrantDb          = 188, ReqlGrantDbInfix,          fn db_grant(db: DB; ReqlDatabase) -> ReqlObject);
	term!(ReqlOpGrantTable       = 188, ReqlGrantTableInfix,       fn table_grant(t: T; ReqlTable) -> ReqlObject);
	term!(ReqlOpSetWriteHook     = 189, ReqlSetWriteHookInfix,     fn set_write_hook(t: T; ReqlTable, f: F; ReqlTypedFunction<(ReqlDynObject, ReqlDynObject, ReqlDynObject), ReqlDynObject>) -> ReqlNull);
	//term!(ReqlOpGetWriteHook     = 190, ReqlGetWriteHookInfix,     fn get_write_hook(t: T; ReqlTable) -> ReqlTypedFunction<(ReqlDynObject, ReqlDynObject, ReqlDynObject), ReqlDynObject>);
	term!(ReqlOpBitAnd           = 191, ReqlBitAndInfix,           fn bit_and(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpBitOr            = 192, ReqlBitOrInfix,            fn bit_or(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpBitXor           = 193, ReqlBitXorInfix,           fn bit_xor(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpBitNot           = 194, ReqlBitNotInfix,           fn bit_not(v: V; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpBitSal           = 195, ReqlBitSalInfix,           fn bit_sal(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
	term!(ReqlOpBitSar           = 196, ReqlBitSarInfix,           fn bit_sar(a: A; ReqlNumber, b: B; ReqlNumber) -> ReqlNumber);
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
	fn simple_term() {
		let mut buf = Vec::<u8>::new();
		db("blog")
			.table("users")
			.filter(crate::obj! { name: "Michel" })
			.serialize(&mut buf)
			.unwrap();
		
		assert_eq!(std::str::from_utf8(&buf), Ok("[39,[[15,[[14,[\"blog\"]],\"users\"]],{\"name\":\"Michel\"}]]"));
	}
	
	#[test]
	fn function_term() {
		let mut buf = Vec::<u8>::new();
		func!(|a: ReqlVar<ReqlDynNumber, 1>, b: ReqlVar<ReqlDynNumber, 2>| { a.add(b) })
			.serialize(&mut buf)
			.unwrap();
		
		assert_eq!(std::str::from_utf8(&buf), Ok("[69,[[2,[1,2]],[24,[[10,[1]],[10,[2]]]]]]"));
	}
}
