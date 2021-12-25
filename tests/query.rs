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

#![allow(dead_code)]

use {
	rethink_driver::*,
	log::{Metadata, Record},
	serde::Deserialize
};

#[cfg(feature = "async")]
use smol::stream::StreamExt;

static LOGGER: Logger = Logger;

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

fn init_conn() -> Connection {
	if log::set_logger(&LOGGER).is_ok() {
		log::set_max_level(log::LevelFilter::Trace);
	}
	
	ConnectionOptions::default()
		.connect()
		.unwrap()
}

#[cfg(feature = "async")]
async fn init_conn_async() -> AsyncConnection {
	if log::set_logger(&LOGGER).is_ok() {
		log::set_max_level(log::LevelFilter::Trace);
	}
	
	ConnectionOptions::default()
		.connect_async().await
		.unwrap()
}

#[test]
#[ignore]
fn connect_success() {
	init_conn();
}

#[cfg(feature = "async")]
#[test]
#[ignore]
fn connect_success_async() {
	smol::block_on(async {
		init_conn_async().await;
	});
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

#[cfg(feature = "async")]
#[test]
#[ignore]
fn connect_invalid_user_async() {
	smol::block_on(async {
		let err = ConnectionOptions {
			user: Some("invalid".to_string()),
			..ConnectionOptions::default()
		}.connect_async().await;
		
		debug_assert!(matches!(err, Err(Error::Auth(AuthError::ServerError { code: Some(17), .. }))));
	});
}

/*#[test]
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
}*/

#[test]
#[ignore]
fn query_no_db() {
	let conn = init_conn();
	let err = db("non-existent_db")
		.table("test")
		.run::<String>(&conn, None)
		.unwrap()
		.next()
		.unwrap()
		.unwrap();
	
	assert_eq!(err, "Database `non-existent_db` does not exist.");
}

#[cfg(feature = "async")]
#[test]
#[ignore]
fn query_no_db_async() {
	smol::block_on(async {
		let conn = init_conn_async().await;
		let err = db("non-existent_db")
			.table("test")
			.run_async::<String>(&conn, None).await
			.unwrap()
			.next().await
			.unwrap()
			.unwrap();
		
		assert_eq!(err, "Database `non-existent_db` does not exist.");
	});
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

#[cfg(feature = "async")]
#[test]
#[ignore]
fn query_serde_err_async() {
	smol::block_on(async {
		#[derive(Deserialize, Debug)]
		struct Test { id: String, invalid: usize }
		
		let conn = init_conn_async().await;
		let err = db("test")
			.table("test")
			.run_async::<Test>(&conn, None).await
			.unwrap()
			.next().await;
		
		debug_assert!(matches!(err, Some(Err(Error::Deserialize(_)))));
	});
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

#[cfg(feature = "async")]
#[test]
#[ignore]
fn query_success_async() {
	smol::block_on(async {
		let conn = init_conn_async().await;
		let ok = db("test")
			.table("test")
			.run_async::<Test>(&conn, None).await;
		
		debug_assert!(ok.is_ok());
	});
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

#[cfg(feature = "async")]
#[test]
#[ignore]
fn query_with_options_success_async() {
	smol::block_on(async {
		let conn = init_conn_async().await;
		let ok = db("test")
			.table("test")
			.run_async::<Test>(&conn, Some(QueryOptions {
				read_mode: Some(ReadMode::Majority),
				.. QueryOptions::default()
			})).await;
		
		debug_assert!(ok.is_ok());
	});
	
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

#[cfg(feature = "async")]
#[test]
#[ignore]
fn query_noreply_async() {
	smol::block_on(async {
		let conn = init_conn_async().await;
		let ok = db("test")
			.table("test")
			.run_async::<()>(&conn, Some(QueryOptions {
				noreply: Some(true),
				.. QueryOptions::default()
			})).await;
		
		debug_assert!(ok.is_ok());
	});
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