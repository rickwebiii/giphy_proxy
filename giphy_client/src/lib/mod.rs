use http::{
    HttpClient,
    Headers,
    HttpVersion,
    request::*,
    Result
};

pub mod args;

use std::collections::HashMap;

use args::Args;

const GIPHY_API: &str = "api.giphy.com";

pub async fn client_main(args: Args) -> Result<()> {
    let client = HttpClient::new(&Authority {
        domain: args.proxy,
        port: Some(args.port)
    });

    let request = Request {
        start_line: StartLine {
            method: Method::CONNECT,
            target: Target::Authority(Authority { domain: GIPHY_API.to_owned(), port: Some(443) }),
            version: HttpVersion::Http1_1,
        },
        headers: Headers {
            headers: HashMap::new()
        }
    };

    client.send_request(&request).await?;

    hyper::client::conn

    Ok(())
}
