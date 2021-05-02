use http::Result;

mod lib;

use crate::lib::server_main;

#[tokio::main]
async fn main() -> Result<()> {
    server_main().await?;

    Ok(())
}
