use async_std::io::Cursor;
use futures::{
    AsyncRead,
    AsyncReadExt,
    AsyncWriteExt,
};

use crate::{
    common::{
        HttpVersion,
        Headers
    },
    error::Result
};

use std::collections::HashMap;

pub struct Response {
    status: Status,
    http_version: HttpVersion,
    headers: Headers,
    body: Box<dyn Send + Unpin + AsyncRead>,
}

impl Response {
    pub async fn write_to_stream<S: Unpin + AsyncWriteExt>(mut self, mut s: S) -> Result<()> {
        let ver = format!("{} ", self.http_version);
        s.write(ver.as_bytes()).await?;

        let status_code = format!("{} ", self.status.to_u16());
        s.write(status_code.as_bytes()).await?;

        let status_message = format!("{}\r\n", self.status.to_str());
        s.write(status_message.as_bytes()).await?;

        for (k, v) in self.headers.headers.iter() {
            let header_line = format!("{}:{}\r\n", k, v);
            s.write(header_line.as_bytes()).await?;
        }

        s.write("\r\n".as_bytes()).await?;

        loop {
            let mut data: Vec<u8> = vec![0; 128];

            let bytes_read = self.body.read(&mut data).await?;

            if bytes_read == 0 {
                break;
            }

            s.write(&data).await?;
        }

        Ok(())
    }

    pub fn new(status: Status, http_version: HttpVersion, headers: Headers, body: Box<dyn Send + Unpin + AsyncRead>) -> Self {
        Self {
            status,
            http_version,
            headers,
            body
        }
    }

    pub fn error_response(status: Status, message: &str) -> Response {
        let mut headers = HashMap::new();
        headers.insert("Content-length".to_owned(), format!("{}", message.len()));
    
        Response::new(status, HttpVersion::Http1_1, Headers::new(headers), Box::new(Cursor::new(message.to_owned())))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Ok,
    BadRequest,
    MethodNotAllowed,
    RequestHeaderFieldsTooLarge,
    UriTooLong,
    BadGateway,

    // TODO: Other status codes
}

impl Status {
    pub fn to_u16(&self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::MethodNotAllowed => 405,
            Self::BadRequest => 400,
            Self::RequestHeaderFieldsTooLarge => 431,
            Self::UriTooLong => 414,
            Self::BadGateway => 502,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Self::Ok => "OK",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::BadRequest => "Bad Request",
            Self::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            Self::UriTooLong => "URI Too Long",
            Self::BadGateway => "Bad Gateway",
        }
    }
}