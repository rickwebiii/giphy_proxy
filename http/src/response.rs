
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

pub struct Response {
    status: Status,
    http_version: HttpVersion,
    headers: Headers,
    body: Box<dyn Send + Unpin + AsyncRead>,
}

impl Response {
    pub async fn write_to_stream<S: Unpin + AsyncWriteExt>(&mut self, mut s: S) -> Result<()> {
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
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Ok

    // TODO: Other status codes
}

impl Status {
    pub fn to_u16(&self) -> u16 {
        match self {
            Self::Ok => 200,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Self::Ok => "OK",
        }
    }
}