// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::{self, Write};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, Value};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum Message {
    Request(RequestMessage),
    Response(ResponseMessage),
    Notofication(NotificationMessage),
}

impl Message {
    fn from_slice(buf: &[u8]) -> serde_json::Result<Message> {
        from_slice::<Message>(buf)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RequestMessage {
    pub id: u64,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ResponseMessage {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Serialize)]
pub(crate) struct JsonRpcRequestMessage<'a> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    params: Value,
}

#[allow(unused)]
pub(crate) fn write_request(
    writer: &mut impl Write,
    id: u64,
    method: &str,
    params: Value,
) -> anyhow::Result<()> {
    let message = JsonRpcRequestMessage {
        jsonrpc: "2.0",
        id: id,
        method: method,
        params: params,
    };
    write_message(writer, message)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ResponseError {
    pub(crate) fn new(code: ErrorCodes, message: String) -> ResponseError {
        ResponseError {
            code: code.into(),
            message: message,
            data: None,
        }
    }
}

#[allow(unused)]
pub(crate) enum ErrorCodes {
    // Defined by JSON RPC
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    #[allow(non_camel_case_types)]
    serverErrorStart,
    #[allow(non_camel_case_types)]
    serverErrorEnd,
    ServerNotInitialized,
    UnknownErrorCode,

    // Defined by the protocol
    RequestCancelled,
    ContentModified,
}

impl From<ErrorCodes> for i32 {
    fn from(code: ErrorCodes) -> i32 {
        match code {
            ErrorCodes::ParseError => -32700,
            ErrorCodes::InvalidRequest => -32600,
            ErrorCodes::MethodNotFound => -32601,
            ErrorCodes::InvalidParams => -32602,
            ErrorCodes::InternalError => -32603,
            ErrorCodes::serverErrorStart => -32099,
            ErrorCodes::serverErrorEnd => -32000,
            ErrorCodes::ServerNotInitialized => -32002,
            ErrorCodes::UnknownErrorCode => -32001,
            ErrorCodes::RequestCancelled => -32800,
            ErrorCodes::ContentModified => -32801,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NotificationMessage {
    pub method: String,
    pub params: Value,
}

// https://microsoft.github.io/language-server-protocol/specification#header-part
#[derive(Debug)]
struct Header {
    pub content_length: usize,
}

fn read_header(reader: &mut impl io::BufRead) -> io::Result<Header> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "No header"));
        }
        if line == "\r\n" {
            break;
        }

        let header_fields = line.trim().split(": ").collect::<Vec<_>>();
        if header_fields.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid header",
            ));
        }
        let name = header_fields[0].to_ascii_lowercase();
        let value = header_fields[1];

        if name == "content-length" {
            let value = match value.parse::<usize>() {
                Ok(n) => n,
                Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
            };
            content_length = Some(value);
        }
    }

    content_length
        .map(|n| Header { content_length: n })
        .ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No content length",
        ))
}

pub(crate) fn read_message(reader: &mut impl io::BufRead) -> anyhow::Result<Message> {
    let header = read_header(reader)?;
    let mut buf = vec![0; header.content_length];
    reader.read_exact(&mut buf)?;
    match Message::from_slice(&buf) {
        Ok(message) => Ok(message),
        Err(_) => Err(anyhow!("Failed to parse message")),
    }
}

#[derive(Serialize)]
struct JsonRpcResponseMessage<'a> {
    jsonrpc: &'a str,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

fn write_message<M: Serialize>(writer: &mut impl Write, message: M) -> anyhow::Result<()> {
    let message = serde_json::to_string(&message)?;
    let content_length = message.len();

    write!(writer, "Content-Length: {}\r\n\r\n", content_length)?;
    writer.write_all(message.as_bytes())?;
    writer.flush()?;
    Ok(())
}

pub(crate) fn write_success_result<R>(
    writer: &mut impl Write,
    id: u64,
    res: R,
) -> anyhow::Result<()>
where
    R: serde::Serialize,
{
    let res = serde_json::to_value(&res)?;
    write_success_response(writer, id, res)
}

pub(crate) fn write_success_response(
    writer: &mut impl Write,
    id: u64,
    result: Value,
) -> anyhow::Result<()> {
    let message = JsonRpcResponseMessage {
        jsonrpc: "2.0",
        id: id,
        result: Some(result),
        error: None,
    };
    write_message(writer, message)
}

pub(crate) fn write_error_response(
    writer: &mut impl Write,
    id: u64,
    error: ResponseError,
) -> anyhow::Result<()> {
    let message = JsonRpcResponseMessage {
        jsonrpc: "2.0",
        id: id,
        result: None,
        error: Some(error),
    };
    write_message(writer, message)
}

#[derive(Serialize)]
struct JsonRpcNotificationMessage<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: Value,
}

pub(crate) fn write_notification(
    writer: &mut impl Write,
    method: &str,
    params: Value,
) -> anyhow::Result<()> {
    let message = JsonRpcNotificationMessage {
        jsonrpc: "2.0",
        method: method,
        params: params,
    };
    write_message(writer, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_header() {
        let input = b"content-length: 208\r\n\r\n";
        let mut reader = io::BufReader::new(&input[..]);
        let header = read_header(&mut reader).unwrap();
        assert_eq!(208, header.content_length);
    }
}
