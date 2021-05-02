use async_std::net::{TcpListener, TcpStream, SocketAddr};
use log::{debug, error};
use futures::{
    Future,
    channel::oneshot::{Sender},
    executor::{ThreadPool},
    stream::{StreamExt},
};

use std::cell::Cell;

use crate::request::{ParseOptions, Request};
use crate::response::Response;
use crate::error::{Error, Result};

pub struct HttpServerBuilder {
    parse_options: ParseOptions,
    bind_addr: Option<SocketAddr>,
    notify_start: Option<Sender<()>>,
}

impl HttpServerBuilder {
    pub fn new() -> Self {
        Self {
            parse_options: ParseOptions::default(),
            bind_addr: None,
            notify_start: None,
        }
    }

    /// Define limits on what the server is willing to parse.
    pub fn parse_options(self, options: ParseOptions) -> Self {
        Self {
            parse_options: options,
            ..self
        }
    }

    pub fn bind_addr(self, addr: SocketAddr) -> Self {
        Self {
            bind_addr: Some(addr),
            ..self
        }
    }

    pub fn notify_start(self, sender: Sender<()>) -> Self {
        Self {
            notify_start: Some(sender),
            ..self
        }
    }

    pub fn build(self) -> Result<HttpServer> {
        Ok(HttpServer {
            parse_options: self.parse_options,
            bind_addr: self.bind_addr.ok_or(Error::NoBindAddress)?,
            executor: ThreadPool::new()?,
            notify_start: Cell::from(self.notify_start),
        })
    }
}

pub struct HttpServer {
    parse_options: ParseOptions,
    bind_addr: SocketAddr,
    executor: ThreadPool,
    notify_start: Cell<Option<Sender<()>>>,
}

impl HttpServer {
    pub async fn run<Fut>(&self, handler: fn(Request, TcpStream) -> Fut) -> Result<()> 
        where Fut: 'static + Send + Future<Output = Result<Response>>
    {
        let listener = TcpListener::bind(self.bind_addr).await?;

        {
            let mut notify = self.notify_start.take();

            if let Some(s) = notify.take() {
                match s.send(()) {
                    Ok(()) => {},
                    Err(e) => {
                        error!("Failed to notify receiver that service started: {:?}", e);
                    }
                };
            }
        }
        
        let parse_options = self.parse_options.clone();

        listener.incoming().for_each_concurrent(None, |conn| async move {
            let stream = match conn {
                Ok(s) => s,
                Err(e) => {
                    debug!("{:?}", e);
                    return;
                }
            };    

            self.executor.spawn_ok(async move {
                 match Request::parse(stream.clone(), &parse_options).await {
                    Ok(req) => {
                        let mut response = match handler(req, stream.clone()).await {
                            Ok(res) => res,
                            Err(e) => {
                                debug!("{:?}", e);
                                return;
                            }
                        };

                        match response.write_to_stream(stream).await {
                            Ok(_) => {},
                            Err(e) => {
                                debug!("{:?}", e);
                                return;
                            }
                        };
                    },
                    Err(e) => {
                        debug!("Failed to parse HTTP request {:?}", e);
                        return;
                    }
                }
            })
        }).await;
        
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::request::*;
    use crate::response::*;
    use crate::common::*;

    use async_std::{
        net::ToSocketAddrs,
        io::Cursor,
    };
    use futures::{
        channel::oneshot,
        executor::LocalPool
    };

    use std::collections::HashMap;

    #[test]
    pub fn can_receive_requests() {
        async fn handle_request(req: Request, _stream: TcpStream) -> Result<Response> {
            assert_eq!(req.start_line.method, Method::GET);
            assert_eq!(req.start_line.target, Target::Path("/".to_owned()));

            let body = "hello world.";

            let mut headers = HashMap::new();
            headers.insert("Content-length".to_owned(), format!("{}", body.len()));

            Ok(Response::new(
                Status::Ok,
                HttpVersion::Http1_1,
                Headers::new(headers),
                Box::new(Cursor::new("Hello world."))
            ))
        }

        let addrs = 
            LocalPool::default().run_until(async {
                 "127.0.0.1:12345".to_socket_addrs().await
            })
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let (tx, rx) = oneshot::channel::<()>();

        std::thread::spawn(move || {
            LocalPool::default().run_until(async {
                HttpServerBuilder::new()
                    .bind_addr(addrs)
                    .notify_start(tx)
                    .build()
                    .unwrap()
                    .run(handle_request)
                    .await
                    .unwrap();
            });
        });

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        
        runtime.block_on(async {
            rx.await.unwrap();

            let response = reqwest::get("http://localhost:12345").await.unwrap();

            assert_eq!(response.status().as_u16(), 200);
            assert_eq!(response.text().await.unwrap(), "Hello world.");
        });
    }
}