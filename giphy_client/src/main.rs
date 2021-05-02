mod lib;

use http::Result;
use structopt::StructOpt;

use crate::lib::client_main;
use lib::args::Args;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::from_args();

    client_main(args).await?;

    Ok(())
}
