use std::collections::HashMap;

use crate::error::{Error, Result};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum HttpVersion {
    Http1_0,
    Http1_1,
    Http2_0,
}

impl HttpVersion {
    pub fn parse(data: &str) -> Result<HttpVersion> {
        match data {
            "HTTP/1.0" => Ok(HttpVersion::Http1_0),
            "HTTP/1.1" => Ok(HttpVersion::Http1_1),
            "HTTP/2.0" => Ok(HttpVersion::Http2_0),
            _ => Err(Error::InvalidHttpVersion),
        }
    }
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::Http1_0 => write!(f, "HTTP/1.0"),
            Self::Http1_1 => write!(f, "HTTP/1.1"),
            Self::Http2_0 => write!(f, "HTTP/2.0"),
        }?;

        Ok(())
    }
}

pub struct Headers {
    pub headers: HashMap<String, String>,
}

impl Headers {
    pub fn new(h: HashMap<String, String>) -> Self {
        Self { headers: h }
    }

    pub fn parse_header(data: &str) -> Result<(&str, &str)> {
        let mut splits = data.split(':');

        let key = splits.next().ok_or(Error::InvalidHeader)?;
        let val = splits.next().ok_or(Error::InvalidHeader)?;

        if key.len() == 0 || val.len() == 0 {
            return Err(Error::InvalidHeader);
        }

        Ok((key.trim(), val.trim()))
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }
}