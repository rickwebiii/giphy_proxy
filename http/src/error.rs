pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct IOErrorWrapper {
    pub err: std::io::Error
}

impl PartialEq for IOErrorWrapper {
    fn eq(&self, b: &Self) -> bool {
        return self.err.to_string() == b.err.to_string()
    }
}

#[derive(Debug, PartialEq)]
pub enum Error {
    /// The string is not a legal HTTP verb.
    InvalidMethod(String),

    /// The HTTP request/response body is not the legal US-ASCII
    InvalidEncoding,

    /// Not yet implemented.
    NotImplemented,

    /// An underlying IO error occurred
    IOError(IOErrorWrapper),

    /// The start line exceeds the maximum length
    StartLineExceedsMaxLength,

    /// The stream contained no bytes when is should
    UnexpectedEndOfStream,

    /// As a whole, the entire headers section of the request is too long.
    HeadersSectionTooLong,

    /// The current header is too long.
    HeaderTooLong,

    /// The header is invalid.
    InvalidHeader,

    /// Not a valid HTTP version string.
    InvalidHttpVersion,

    /// Not a legal HTTP start line.
    InvalidStartLine,

    /// Not a legal target.
    InvalidTarget,

    /// Received a carriage return not followed by a LF.
    UnexpectedCR,

    /// Failed to specify a bind address for the server.
    NoBindAddress,

    /// Connection closed
    ConnectionClosed,

    /// The specified URL doesnt' have a port.
    MissingPort,

    DnsLookupFailed,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(IOErrorWrapper { err })
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {

}