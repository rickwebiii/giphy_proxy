use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "giphy_client",
    about = "Proxies a search for gifs to maintain privacy."
)]
pub struct Args {
    #[structopt(short = "p", long = "proxy")]
    pub proxy: String,

    #[structopt(short = "k", long = "api-key")]
    pub api_key: String,

    #[structopt(short = "r", long = "port")]
    pub port: u16,

    #[structopt(parse(from_str))]
    pub tags: Vec<String>,
}
