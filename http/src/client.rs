use async_std::net::{SocketAddr, TcpStream};
use url::Url;

use crate::{
    Error,
    Result,
    request::{
        Authority,
        Request,
    }
};

pub struct HttpClient {
    host: Authority,
}

impl HttpClient {
    pub fn new(host: &Authority) -> Self {
        HttpClient {
            host: host.clone(),
        }
    }

    /// Sends HTTP request headers and returns the underlying connection.
    pub async fn send_request(&self, request: &Request) -> Result<TcpStream> {
        let addr = format!(
            "{}:{}",
            self.host.domain,
            self.host.port.ok_or(Error::MissingPort)?
        );

        let mut socket = TcpStream::connect(addr).await?;

        request.write_to_stream(&mut socket).await?;

        Ok(socket)
    }
}
