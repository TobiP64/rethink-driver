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
	crate::{*, auth::AuthError::*, Error::Auth},
	std::io::{self, Read, Write},
	serde::{Serialize, Deserialize},
	hmac::{Hmac, Mac, NewMac},
	sha2::Sha256,
	base64::STANDARD
};

pub const AUTH_METHOD_SCRAM_SHA_256: &str = "SCRAM-SHA-256";
pub const MIN_ITERATIONS:            u32 = 4096;
pub const DEFAULT_BUFFER_LEN:        usize = 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
	InvalidReply,
	InvalidServerNonce,
	InvalidServerSignature,
	InvalidIterationCount,
	ServerError { code: Option<usize>, msg: String }
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct ClientFirstMessage<'a> {
	pub protocol_version:      usize,
	pub authentication_method: &'a str,
	pub authentication:        &'a str
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct ClientFinalMessage<'a> {
	pub authentication: &'a str
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
pub struct ServerVersionMessage<'a> {
	pub success:              bool,
	pub min_protocol_version: usize,
	pub max_protocol_version: usize,
	pub server_version:       &'a str
}

impl<'a> ServerVersionMessage<'a> {
	fn from_buf<'b>(buf: &'b [u8]) -> Result<ServerVersionMessage<'b>> {
		let message = serde_json::from_slice::<ServerVersionMessage<'b>>(buf)
			.map_err(|_| match std::str::from_utf8(buf).map(String::from) {
				Ok(msg) => Auth(ServerError { code: None, msg }),
				Err(_)  => Auth(InvalidReply)
			})?;
		
		if !message.success {
			Err(Auth(InvalidReply))
		} else {
			Ok(message)
		}
	}
	
	fn recv(reader: &mut impl Read, buf: &'a mut [u8]) -> Result<(Self, &'a mut [u8])> {
		let (old_buf, new_buf) = recv_msg(reader, buf)?;
		Ok((Self::from_buf(old_buf)?, new_buf))
	}
	
	#[cfg(feature = "async")]
	async fn recv_async<'b>(reader: &mut (impl smol::io::AsyncReadExt + Unpin), buf: &'b mut [u8]) -> Result<(ServerVersionMessage<'b>, &'b mut [u8])> {
		let (old_buf, new_buf) = recv_msg_async(reader, buf).await?;
		Ok((Self::from_buf(old_buf)?, new_buf))
	}
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
pub struct ServerMessage<'a> {
	pub success:        bool,
	pub authentication: Option<&'a str>,
	pub error:          Option<&'a str>,
	pub error_code:     Option<usize>
}

impl<'a> ServerMessage<'a> {
	fn from_buf<'b>(buf: &'b [u8]) -> Result<ServerMessage<'b>> {
		let message = serde_json::from_slice::<ServerMessage<'b>>(buf)
			.map_err(|_| Auth(InvalidReply))?;
		
		if !message.success {
			Err(Auth(ServerError {
				code: message.error_code,
				msg:  message.error.unwrap_or_default().to_string()
			}))
		} else {
			Ok(message)
		}
	}
	
	fn recv(reader: &mut impl Read, buf: &'a mut [u8]) -> Result<(Self, &'a mut [u8])> {
		let (old_buf, new_buf) = recv_msg(reader, buf)?;
		Ok((Self::from_buf(old_buf)?, new_buf))
	}
	
	#[cfg(feature = "async")]
	async fn recv_async<'b>(reader: &mut (impl smol::io::AsyncReadExt + Unpin), buf: &'b mut [u8]) -> Result<(ServerMessage<'b>, &'b mut [u8])> {
		let (old_buf, new_buf) = recv_msg_async(reader, buf).await?;
		Ok((Self::from_buf(old_buf)?, new_buf))
	}
}

pub fn handshake(stream: &mut (impl Read + Write), options: &ConnectionOptions) -> Result<()> {
	let start    = std::time::Instant::now();
	let mut buf  = [0u8; DEFAULT_BUFFER_LEN];
	let user     = stringprep::saslprep(options.user.as_deref().unwrap_or(DEFAULT_USER)).unwrap();
	let password = stringprep::saslprep(options.password.as_deref().unwrap_or(DEFAULT_PWD)).unwrap();
	
	// protocol and server versions
	
	stream.write_all(&VERSION_1_0.to_le_bytes())?;
	stream.flush()?;
	
	let (server_version, _) = ServerVersionMessage::recv(stream, &mut buf)?;
	log::info!("{:?}", server_version);
	
	// client first message
	
	let client_nonce = gen_client_nonce();
	send_msg(stream, &mut buf, &[
		b"{\"protocol_version\":0, \"authentication_method\": \"SCRAM-SHA-256\",\"authentication\":\"n,,n=",
		user.as_bytes(),
		b",r=",
		&client_nonce,
		b"\"}\0"
	])?;
	
	// server first message
	
	let (server_first_message, new_buf) = ServerMessage::recv(stream, &mut buf)?;
	
	let auth = server_first_message.authentication.unwrap();
	let combined_nonce = sasl_get(auth, "r=")
		.ok_or(Auth(InvalidReply))?
		.as_bytes();
	let (salt, new_buf) = base64_decode(sasl_get(auth, "s=")
		.ok_or(Auth(InvalidReply))?, new_buf)
		.map_err(|_| Auth(InvalidReply))?;
	let iterations = sasl_get(auth, "i=")
		.ok_or(Auth(InvalidReply))?
		.parse::<u32>()
		.map_err(|_| Auth(InvalidReply))?;
	
	if !password.is_empty() && iterations < MIN_ITERATIONS {
		return Err(Auth(InvalidIterationCount));
	} else if !combined_nonce.starts_with(&client_nonce) {
		return Err(Auth(InvalidServerNonce));
	}
	
	// client final message
	
	let auth_msg = concat_into(new_buf, &[
		b"n=",
		user.as_bytes(),
		b",r=",
		&client_nonce,
		b",",
		auth.as_bytes(),
		b",c=biws,r=",
		combined_nonce
	]);
	
	let (client_proof, server_signature) = gen_client_proof(
		password.as_bytes(), salt, iterations, auth_msg);
	let (base64_client_proof, new_buf) = base64_encode(&client_proof, new_buf);
	
	send_msg(stream, new_buf, &[
		b"{\"authentication\":\"c=biws,r=",
		combined_nonce,
		b",p=",
		base64_client_proof,
		b"\"}\0"
	])?;
	
	// server final message
	
	let (server_last_message, new_buf) = ServerMessage::recv(stream, &mut buf)?;
	let recv_server_signature = sasl_get(server_last_message.authentication.unwrap(), "v=")
		.ok_or(Auth(InvalidReply))?;
	let (recv_server_signature, _) = base64_decode(recv_server_signature, new_buf)
		.map_err(|_| Auth(InvalidReply))?;
	
	if recv_server_signature != server_signature {
		return Err(Auth(InvalidServerSignature));
	}
	
	// cleanup
	
	for v in buf.iter_mut() { *v = 0; }
	log::info!("successfully authenticated as `{}` ({}ms)", user, start.elapsed().as_millis());
	Ok(())
}

#[cfg(feature = "async")]
pub async fn handshake_async(stream: &mut (impl smol::io::AsyncReadExt + smol::io::AsyncWriteExt + Unpin), options: &ConnectionOptions) -> Result<()> {
	let start    = std::time::Instant::now();
	let mut buf  = [0u8; DEFAULT_BUFFER_LEN];
	let user     = stringprep::saslprep(options.user.as_deref().unwrap_or(DEFAULT_USER)).unwrap();
	let password = stringprep::saslprep(options.password.as_deref().unwrap_or(DEFAULT_PWD)).unwrap();
	
	// protocol and server versions
	
	stream.write_all(&VERSION_1_0.to_le_bytes()).await?;
	stream.flush().await?;
	
	let (server_version, _) = ServerVersionMessage::recv_async(stream, &mut buf).await?;
	log::info!("{:?}", server_version);
	
	// client first message
	
	let client_nonce = gen_client_nonce();
	send_msg_async(stream, &mut buf, &[
		b"{\"protocol_version\":0, \"authentication_method\": \"SCRAM-SHA-256\",\"authentication\":\"n,,n=",
		user.as_bytes(),
		b",r=",
		&client_nonce,
		b"\"}\0"
	]).await?;
	
	// server first message
	
	let (server_first_message, new_buf) = ServerMessage::recv_async(stream, &mut buf).await?;
	
	let auth = server_first_message.authentication.unwrap();
	let combined_nonce = sasl_get(auth, "r=")
		.ok_or(Auth(InvalidReply))?
		.as_bytes();
	let (salt, new_buf) = base64_decode(sasl_get(auth, "s=")
		.ok_or(Auth(InvalidReply))?, new_buf)
		.map_err(|_| Auth(InvalidReply))?;
	let iterations = sasl_get(auth, "i=")
		.ok_or(Auth(InvalidReply))?
		.parse::<u32>()
		.map_err(|_| Auth(InvalidReply))?;
	
	if !password.is_empty() && iterations < MIN_ITERATIONS {
		return Err(Auth(InvalidIterationCount));
	} else if !combined_nonce.starts_with(&client_nonce) {
		return Err(Auth(InvalidServerNonce));
	}
	
	// client final message
	
	let auth_msg = concat_into(new_buf, &[
		b"n=",
		user.as_bytes(),
		b",r=",
		&client_nonce,
		b",",
		auth.as_bytes(),
		b",c=biws,r=",
		combined_nonce
	]);
	
	let (client_proof, server_signature) = gen_client_proof(
		password.as_bytes(), salt, iterations, auth_msg);
	let (base64_client_proof, new_buf) = base64_encode(&client_proof, new_buf);
	
	send_msg_async(stream, new_buf, &[
		b"{\"authentication\":\"c=biws,r=",
		combined_nonce,
		b",p=",
		base64_client_proof,
		b"\"}\0"
	]).await?;
	
	// server final message
	
	let (server_last_message, new_buf) = ServerMessage::recv_async(stream, &mut buf).await?;
	let recv_server_signature = sasl_get(server_last_message.authentication.unwrap(), "v=")
		.ok_or(Auth(InvalidReply))?;
	let (recv_server_signature, _) = base64_decode(recv_server_signature, new_buf)
		.map_err(|_| Auth(InvalidReply))?;
	
	if recv_server_signature != server_signature {
		return Err(Auth(InvalidServerSignature));
	}
	
	// cleanup
	
	for v in buf.iter_mut() { *v = 0; }
	log::info!("successfully authenticated as `{}` ({}ms)", user, start.elapsed().as_millis());
	Ok(())
}

fn send_msg(writer: &mut impl Write, buf: &mut [u8], src: &[&[u8]]) -> Result<()> {
	writer.write_all(concat_into(buf, src))?;
	writer.flush()?;
	Ok(())
}

fn recv_msg<'a>(reader: &mut impl Read, buf: &'a mut [u8]) -> Result<(&'a mut [u8], &'a mut [u8])> {
	let mut off = 0;
	
	loop {
		off += match reader.read(&mut buf[off..])? {
			0 => return Err(Error::Io(io::Error::from(io::ErrorKind::UnexpectedEof))),
			v => v
		};
		
		if  buf[off - 1] == 0 {
			return Ok(buf.split_at_mut(off - 1))
		}
	}
}

#[cfg(feature = "async")]
async fn send_msg_async(writer: &mut (impl smol::io::AsyncWriteExt + Unpin), buf: &mut [u8], src: &[&[u8]]) -> Result<()> {
	writer.write_all(concat_into(buf, src)).await?;
	writer.flush().await?;
	Ok(())
}

#[cfg(feature = "async")]
#[allow(clippy::needless_lifetimes)] // removing the lifetime results in a compiler error
async fn recv_msg_async<'a>(reader: &mut (impl smol::io::AsyncReadExt + Unpin), buf: &'a mut [u8]) -> Result<(&'a mut [u8], &'a mut [u8])> {
	let mut off = 0;
	
	loop {
		off += match reader.read(&mut buf[off..]).await? {
			0 => return Err(Error::Io(io::Error::from(io::ErrorKind::UnexpectedEof))),
			v => v
		};
		
		if  buf[off - 1] == 0 {
			return Ok(buf.split_at_mut(off - 1))
		}
	}
}

fn base64_decode<'a, T: ?Sized + AsRef<[u8]>>(input: &T, buf: &'a mut [u8]) -> std::result::Result<(&'a mut [u8], &'a mut [u8]), base64::DecodeError> {
	let len = base64::decode_config_slice(input, STANDARD, buf)?;
	Ok(buf.split_at_mut(len))
}

fn base64_encode<'a, T: AsRef<[u8]>>(input: &T, buf: &'a mut [u8]) -> (&'a mut [u8], &'a mut [u8]) {
	let len = base64::encode_config_slice(input, STANDARD, buf);
	buf.split_at_mut(len)
}

fn gen_client_nonce() -> [u8; 32] {
	use rand::Rng;
	let mut client_nonce = [0u8; 32];
	for e in &mut client_nonce {
		*e = rand::thread_rng().gen_range(0x30u8..0x5Bu8);
	}
	client_nonce
}

fn gen_client_proof(password: &[u8], salt: &[u8], i: u32, auth_msg: &[u8]) -> ([u8; 32], [u8; 32]) {
	let mut salted_pwd = [0u8; 32];
	pbkdf2::pbkdf2::<Hmac<Sha256>>(password, salt, i, &mut salted_pwd);
	let client_key       = gen_key(salted_pwd, b"Client Key");
	let server_key       = gen_key(salted_pwd, b"Server Key");
	let stored_key       = gen_stored_key(client_key);
	let client_signature = gen_signature(stored_key, auth_msg);
	let server_signature = gen_signature(server_key, auth_msg);
	let mut client_proof = [0u8; 32];
	
	for i in 0..32 {
		client_proof[i] = client_key[i] ^ client_signature[i];
	}
	
	(client_proof, server_signature)
}

fn concat_into<'a>(buf: &'a mut [u8], src: &[&[u8]]) -> &'a [u8] {
	let mut i = 0;
	src.iter().for_each(|v| {
		buf[i..i + v.len()].copy_from_slice(v);
		i += v.len();
	});
	&buf[..i]
}

fn gen_key(salted_pwd: [u8; 32], key: &[u8]) -> [u8; 32] {
	let mut hmac = Hmac::<Sha256>::new_from_slice(&salted_pwd).unwrap();
	hmac.update(key);
	hmac.finalize().into_bytes().into()
}

fn gen_stored_key(client_key: [u8; 32]) -> [u8; 32] {
	use sha2::Digest;
	let mut sha = Sha256::new();
	sha.write_all(&client_key).unwrap();
	*sha.finalize().as_ref()
}

fn gen_signature(key: [u8; 32], auth_msg: &[u8]) -> [u8; 32] {
	let mut hmac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
	hmac.update(auth_msg);
	hmac.finalize().into_bytes().into()
}

fn sasl_get<'a>(msg: &'a str, attr: &str) -> Option<&'a str> {
	let off = msg.find(attr)? + attr.len();
	let len = msg[off..].find(',').unwrap_or(msg.len() - off);
	Some(&msg[off..off + len])
}
