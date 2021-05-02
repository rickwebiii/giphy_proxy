use http::{
    Result,
};

mod lib;

use crate::lib::do_main;

#[tokio::main]
async fn main() -> Result<()> {
    do_main().await?;

    Ok(())
}
