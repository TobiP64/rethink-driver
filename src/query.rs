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
	crate::*,
	std::time::Duration,
	serde::{*, de::DeserializeOwned},
	serde_repr::*
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
pub struct Query<'a, T: Serialize = ()> {
	pub r#type:  QueryType,
	pub term:    Option<T>,
	pub options: Option<QueryOptions<'a>>
}

impl<'a, T: Serialize> Serialize for Query<'a, T> {
	fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		#[derive(Serialize)]
		struct Tuple<'a, T: Serialize>(
			QueryType,
			#[serde(skip_serializing_if = "Option::is_none")] &'a Option<T>,
			#[serde(skip_serializing_if = "Option::is_none")] &'a Option<QueryOptions<'a>>
		);
		
		Tuple(self.r#type, &self.term, &self.options).serialize(serializer)
	}
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

#[derive(Debug, Clone)]
pub enum ReqlError {
	Compile,
	Runtime(ReqlRuntimeError),
	Driver(ReqlDriverError)
}

#[derive(Debug, Copy, Clone)]
pub enum ReqlDriverError {
	BadRequest,
	Auth
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
pub struct Binary;

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
	pub generated_keys: Vec<Id>,
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