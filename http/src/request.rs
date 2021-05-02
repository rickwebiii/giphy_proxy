use async_std::io::ReadExt;
use std::collections::HashMap;
use url::Url;

use std::str::FromStr;

use crate::{
    Error,
    Result,
    common::{
        HttpVersion,
        Headers,
    }
};

#[derive(Debug, PartialEq)]
pub enum Method {
    GET,
    POST,
    PUT,
    HEAD,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

impl Method {
    pub fn parse(data: &str) -> Result<Method> {
        match data {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "HEAD" => Ok(Method::HEAD),
            "DELETE" => Ok(Method::DELETE),
            "CONNECT" => Ok(Method::CONNECT),
            "OPTIONS" => Ok(Method::OPTIONS),
            "TRACE" => Ok(Method::TRACE),
            "PATCH" => Ok(Method::PATCH),
            _ => Err(Error::InvalidMethod(data.to_owned())),
        }
    }
}

/// The host and port part of a url. Should only be used with OPTIONS verb
#[derive(Debug, PartialEq)]
pub struct Authority {
    pub domain: String,
    pub port: Option<u16>,
}

/// A set of limits on HTTP requests to mitigate slowloris attacks.
#[derive(Debug, Clone, Copy)]
pub struct ParseOptions {
    /// Maximum length of the target section of the start in the URL.
    max_target_len: usize,
    max_headers_section_len: usize,
    max_header_len: usize,
    max_body_len: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            max_target_len: 16 * 1024,
            max_headers_section_len: 16 * 1024,
            max_header_len: 1024,
            max_body_len: 2 * 1024 * 1024,
        }
    }
}

impl ParseOptions {
    /// Returns the maximum length of a start line with this configuration.
    pub fn max_start_line_len(&self) -> usize {
        const MAX_VERB_LEN: usize = "OPTIONS".len();
        const SPACES: usize = 2;
        const MAX_PROTOCOL_LEN: usize = "HTTP/1.1".len();

        self.max_target_len + MAX_VERB_LEN + SPACES + MAX_PROTOCOL_LEN
    }

    pub fn max_headers_section_len(&self) -> usize {
        self.max_headers_section_len
    }

    pub fn max_header_len(&self) -> usize {
        self.max_header_len
    }
}

/// The second field in the start line.
/// See https://developer.mozilla.org/en-US/docs/Web/HTTP/Messages
#[derive(Debug, PartialEq)]
pub enum Target {
    /// Absolute path form,
    /// Usually used with POST, POST, HEAD, and OPTIONS
    Path(String),

    /// Url form.
    /// Usually used with GET
    Url(Url),

    /// Used with OPTIONS verb to connect with a raw TCP socket. Because it's a proxy,
    /// we don't need to know what the protocol is.
    Authority(Authority),

    /// Used with OPTIONS verb.
    Glob,
}

impl Target {
    pub fn parse(target_str: &str) -> Result<Target> {
        if target_str.len() == 0 {
            return Err(Error::InvalidTarget);
        }

        // We've asserted the length is > 0, so we're guaranteed url has at least 1 char.
        if target_str.chars().next().unwrap() == '/' {
            return Ok(Self::Path(target_str.to_owned()));
        }

        if target_str.chars().next().unwrap() == '*' {
            return if target_str.len() == 1 {
                Ok(Self::Glob)
            } else {
                Err(Error::InvalidTarget)
            };
        }

        let authority_regex =
            regex::Regex::from_str(r"^((\d|[[:alpha:]])+\.)+(\d|[[:alpha:]])+(:\d+)?$").unwrap();

        if authority_regex.is_match(target_str) {
            let mut splits = target_str.split(":");

            // Should be unreachable, but panicking here is probably worse than just returning
            // invalid.
            let domain = splits.next().ok_or(Error::InvalidTarget)?;
            let port = splits.next();

            return Ok(Self::Authority(Authority {
                domain: domain.to_owned(),
                port: match port {
                    // Conversion errors may occur if port is out of range for u16.
                    Some(p) => Some(u16::from_str_radix(p, 10).map_err(|_| Error::InvalidTarget)?),
                    None => None,
                },
            }));
        }

        return Ok(Self::Url(
            Url::from_str(target_str).map_err(|_| Error::InvalidTarget)?,
        ));
    }
}


pub struct StartLine {
    pub method: Method,
    pub target: Target,
    pub version: HttpVersion,
}

impl StartLine {
    pub fn parse(data: &str) -> Result<Self> {
        let mut splits = data.split(' ');

        let method = Method::parse(splits.next().ok_or(Error::InvalidStartLine)?)?;
        let target = Target::parse(splits.next().ok_or(Error::InvalidStartLine)?)?;
        let version = HttpVersion::parse(splits.next().ok_or(Error::InvalidStartLine)?)?;

        Ok(StartLine {
            method,
            target,
            version,
        })
    }
}


pub struct Request {
    pub start_line: StartLine,
    pub headers: Headers,
}

enum RequestParseStateMachine {
    ParseStartLine,
    ParseHeaders(usize, StartLine, HashMap<String, String>),
}

impl Request {
    /// Consumes the stream and parses the request start and headers. Mitigates some aspects of slowloris
    /// attacks by aborting if reading too many characters in a given section of the request. Does not
    /// assume newlines will come before the limit is reached. In the event of failure, the stream will
    /// effectively be closed.
    /// TODO: use a timer to measure request bandwidth and enforce a minimum before just erroring.
    /// TODO: We assume enforce that the start line and headers are ASCII. The internet suggests this is correct,
    /// but I'm not sure and leaves an open question around how HTTP handles Internationalized Domain Names
    pub async fn parse<R>(mut data: R, parse_options: &ParseOptions) -> Result<Self>
    where
        R: ReadExt + Unpin,
    {
        let mut read_buffer = vec![0; 1];
        let mut current_line = vec![];

        let mut state = RequestParseStateMachine::ParseStartLine;

        loop {
            let num_stream_bytes = data.read(&mut read_buffer).await?;

            if num_stream_bytes == 0 {
                return Err(Error::UnexpectedEndOfStream);
            }

            // Check that we haven't exceeded limits
            match state {
                RequestParseStateMachine::ParseStartLine => {
                    if current_line.len() > parse_options.max_start_line_len() {
                        return Err(Error::StartLineExceedsMaxLength);
                    }
                }
                RequestParseStateMachine::ParseHeaders(ref size, ref _s, ref _h) => {
                    if parse_options.max_headers_section_len() < *size + current_line.len() {
                        return Err(Error::HeadersSectionTooLong);
                    } else if current_line.len() > parse_options.max_header_len() {
                        return Err(Error::HeaderTooLong);
                    }
                }
            };

            if !read_buffer[0].is_ascii() {
                return Err(Error::InvalidEncoding);
            }

            // Standard dictates CRLF, but that we can tolerate LF alone. 
            // If we get a CR, read the next character and assert it's a \n. Why would you
            // put CR into headers?
            // Since The next character must be newline, we don't need to recheck the line_size
            // because you can't put more than one CR in a row in the buffer.
            if read_buffer[0] == b'\r' {
                let num_stream_bytes = data.read(&mut read_buffer).await?;

                if num_stream_bytes == 0 {
                    return Err(Error::UnexpectedEndOfStream);
                }

                if read_buffer[0] != b'\n' {
                    return Err(Error::UnexpectedCR);
                }
            }
            
            if read_buffer[0] == b'\n' {
                // We've validated all the characters in the stream are ASCII, so the below is
                // sound.
                let current_line_str = unsafe { std::str::from_utf8_unchecked(&current_line) };

                state = match state {
                    RequestParseStateMachine::ParseStartLine => {
                        RequestParseStateMachine::ParseHeaders(
                            0,
                            StartLine::parse(current_line_str)?,
                            HashMap::new(),
                        )
                    }
                    RequestParseStateMachine::ParseHeaders(
                        headers_len,
                        start_line,
                        mut headers,
                    ) => {
                        // A blank line signals the end of headers and thus we return the response and the stream.
                        // The remainder of the stream may contain a body or in the case of CONNECT, data from the
                        // proxied connection.
                        if current_line_str.len() == 0 {
                            return Ok(
                                Self {
                                    start_line,
                                    headers: Headers::new(headers),
                                },
                            );
                        }

                        let (key, val) = Headers::parse_header(&current_line_str)?;

                        headers.insert(key.to_owned(), val.to_owned());

                        RequestParseStateMachine::ParseHeaders(headers_len, start_line, headers)
                    }
                };

                current_line.clear();
            } else {
                current_line.push(read_buffer[0]);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use async_std::io::{Cursor};
    
    use super::*;

    #[test]
    pub fn can_parse_start_line() {
        let start_line = StartLine::parse("CONNECT horse.billy:80 HTTP/1.1").unwrap();

        assert_eq!(start_line.method, Method::CONNECT);
        assert_eq!(start_line.target, Target::Authority(Authority { domain: "horse.billy".to_owned(), port: Some(80) }));

        let start_line = StartLine::parse("CONNECT horse.billy HTTP/1.1").unwrap();

        assert_eq!(start_line.method, Method::CONNECT);
        assert_eq!(start_line.target, Target::Authority(Authority { domain: "horse.billy".to_owned(), port: None }));
    }

    #[test]
    pub fn can_parse_header() {
        let header = Headers::parse_header(":");

        assert_eq!(header, Err(Error::InvalidHeader));

        let header = Headers::parse_header("  a : b");

        assert_eq!(header, Ok(("a", "b")));
    }

    #[test]
    pub fn can_parse_request() {
        let request_str = format!("{}{}{}",
            "CONNECT horse.billy:443 HTTP/1.1\r\n",
            "header1: horse\r\n",
            "\r\n"
        );

        let mut executor = futures::executor::LocalPool::default();

        let parsed = executor.run_until(async {
            Request::parse(Cursor::new(request_str.as_bytes()), &ParseOptions::default()).await.unwrap()
        });

        assert_eq!(parsed.start_line.method, Method::CONNECT);
        assert_eq!(parsed.start_line.target, Target::Authority(Authority { domain: "horse.billy".to_owned(), port: Some(443) }));
        assert_eq!(parsed.start_line.version, HttpVersion::Http1_1);
        assert_eq!(parsed.headers.get("header1").unwrap(), "horse");
    }
}
