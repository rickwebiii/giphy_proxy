use http::{request::*, response::*, Error, HttpServerBuilder, Result};

use async_std::{
    net::{TcpStream, ToSocketAddrs},
};
use log::{debug, error, info};
use futures::{AsyncReadExt, AsyncWriteExt};

pub async fn server_main() -> Result<()> {
    simple_logger::SimpleLogger::new().init().unwrap();

    info!("Starting server..");

    // TODO, make the address configurable
    let addrs = "127.0.0.1:12345"
        .to_socket_addrs()
        .await?
        .into_iter()
        .next()
        .unwrap();

    HttpServerBuilder::new()
        .bind_addr(addrs)
        .build()?
        .run(handle_proxy)
        .await?;

    Ok(())
}

/// CONNECT is kind of a weird use case of HTTP. This function will never return with a success.
/// We parse the request, open a socket to the destination (if valid), then proxy data in both
/// directions until either stream closes. We then return a ConnectionClosed error, but the client
/// should have received what it wanted.
async fn handle_proxy(request: Request, stream: TcpStream) -> Result<Response> {
    info!("Got request: {:?}", request);

    if request.start_line.method != Method::CONNECT {
        error!("Method is not CONNECT");
        return Ok(Response::error_response(Status::MethodNotAllowed, ""));
    }

    let host = match request.start_line.target {
        Target::Authority(a) => a,
        _ => {
            error!("Invalid proxy target");
            return Ok(Response::error_response(
                Status::BadRequest,
                "Invalid proxy target",
            ));
        }
    };

    if let Some(port) = host.port {
        if port != 443 {
            error!("Invalid port {}", port);
            return Ok(Response::error_response(
                Status::BadRequest,
                "Invalid port. Must use 443",
            ));    
        }
    }

    if host.domain != "api.giphy.com" || host.port.is_none() {
        error!("Invalid target domain: {}", host.domain);
        return Ok(Response::error_response(
            Status::BadRequest,
            "Invalid proxy target",
        ));
    }

    let addr = format!("{}:{}", host.domain, host.port.unwrap_or(0)).to_socket_addrs().await?
        .into_iter()
        .next();

    let addr = match addr {
        Some(s) => s,
        None => {
            error!("DNS lookup failed.");
            return Ok(Response::error_response(
                Status::BadGateway,
                "Failed to proxy to remote service",
            ));
        }
    };

    let proxied_connection = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to connect to remote service. {:?}", e);
            return Ok(Response::error_response(
                Status::BadGateway,
                "Failed to proxy to remote service",
            ));
        }
    };

    info!("Connection established");

    let ok_response = Response::error_response(Status::Ok, "");
    ok_response.write_to_stream(stream.clone()).await?;

    let s1 = proxied_connection.clone();
    let s2 = stream.clone();

    let read_proxy = tokio::spawn(async move {
        let _ = stream_copy(s1, s2).await;
    });

    let read_client = tokio::spawn(async move {
        let _ = stream_copy(stream, proxied_connection).await;
    });

    let _ = read_client.await;
    let _ = read_proxy.await;

    Err(Error::ConnectionClosed)
}

async fn stream_copy(mut s1: TcpStream, mut s2: TcpStream) -> Result<()> {
    let mut buf: Vec<u8> = vec![0; 1024];

    debug!("Connecting streams...");

    loop {
        match s1.read(&mut buf).await {
            Ok(bytes_read) => {
                info!("Got {} bytes from {:?}", bytes_read, s1.local_addr());
                if bytes_read == 0 {
                    info!("Connection closed.");
                    break;
                }

                let (data, _) = buf.split_at(bytes_read);

                match s2.write_all(data).await {
                    Ok(_) => {},
                    Err(e) => {
                        error!("Write failed: {:?}", e);
                    }
                };
            },
            Err(e) => {
                error!("{:?}", e);
                break;
            }
        }
    }

    Err(Error::ConnectionClosed)
}