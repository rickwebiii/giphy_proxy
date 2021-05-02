use http::{
    Error,
    HttpServerBuilder,
    Result,
    request::*,
    response::*,
};

use async_std::{
    net::{TcpStream, ToSocketAddrs},
    prelude::FutureExt,
};
use futures::{
    AsyncReadExt,
    AsyncWriteExt,
};

pub async fn do_main() -> Result<()> {
    // TODO, make the address configurable
    let addrs = "127.0.0.1:12345".to_socket_addrs()
        .await?
        .into_iter()
        .next()
        .unwrap();

    HttpServerBuilder::new()
        .bind_addr(addrs)
        .build()?
        .run(handle_proxy).await;

    Ok(())
}

/// CONNECT is kind of a weird use case of HTTP. This function will never return with a success.
/// We parse the request, open a socket to the destination (if valid), then proxy data in both
/// directions until either stream closes. We then return a ConnectionClosed error, but the client
/// should have received what it wanted.
async fn handle_proxy(request: Request, mut stream: TcpStream) -> Result<Response> {
    if request.start_line.method != Method::CONNECT {
        return Ok(Response::error_response(Status::MethodNotAllowed, ""));
    }

    let host = match request.start_line.target {
        Target::Authority(a) => a,
        _ => {
            return Ok(Response::error_response(Status::BadRequest, "Invalid proxy target"));
        }
    };

    if host.domain != "api.giphy.com:443" {
        return Ok(Response::error_response(Status::BadRequest, "Invalid proxy target"));
    }

    let mut proxied_connection = match TcpStream::connect(host.domain).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(Response::error_response(Status::BadGateway, "Failed to proxy to remote service"));
        }
    };

    let mut proxy_buf: Vec<u8> = vec![0; 1024];
    let mut client_buf: Vec<u8> = vec![0; 1024];

    loop {
        
        let proxy_fut = tagged_read(&mut proxied_connection, &mut proxy_buf, "proxy");
        let stream_fut = tagged_read(&mut stream, &mut client_buf, "client");
        
        let (bytes_read, tag) = proxy_fut.race(stream_fut).await?;

        if bytes_read == 0 {
            return Err(Error::ConnectionClosed)
        }

        if tag == "proxy" {
            let (data, _) = proxy_buf.split_at(bytes_read);

            stream.write_all(data).await?;
        } else {
            let (data, _) = proxy_buf.split_at(bytes_read);

            proxied_connection.write_all(data).await?;
        }
    }
}

async fn tagged_read<'a, R: Unpin + AsyncReadExt>(s: &mut R, buf: &mut [u8], tag: &'a str) -> Result<(usize, &'a str)> {
    let bytes_read = s.read(buf).await?;

    Ok((bytes_read, tag))
}