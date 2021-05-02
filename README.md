# Giphy-proxy
This app serves as a tunnel for queries against giphy APIs. It features the following:
* Fully async using async_std and tokio.
* Http requests are parsed directly from streams with limits to help mitigate slowloris attacks.

It consists of 2 crates:
* `http` is a library defining HTTP requests and responses, as well as an HTTP server. The server depends on the tokio runtime, so you have to set up a `Runtime` to use it.
* `giphy_proxy` is a proxy server that listens for HTTP CONNECT requests and establishes a tunnel.

## Running tests:
`cargo test`

## Running proxy
I tested this on:
* Rust stable-aarch64-apple-darwin 1.51.0
* M1 Mac running macOS 11.2.3

You'll need a Giphy API key. Create an account and visit their documentation on how to acquire one.

Start proxy: `cargo run --release`

Visit

In Firefox:
1. Click Preferences.
2. Search for Proxy.
3. Click the `Settings...` under Network Settings.
4. Select the `Manual proxy configuration` radio button.
5. Set the HTTP proxy to `localhost` on port `12345`

In the browser bar, visit
`https://api.giphy.com/v1/gifs/search?q=<SEARCH_TERM>&api_key=<YOUR_API_KEY>`

You should see a bunch of JSON.