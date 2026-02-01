mod domain;
mod frameworks;
mod interface_adapters;
mod use_cases;

use frameworks::server;

#[tokio::main]
async fn main() {
    // Delegate to the server framework entry point.
    server::run().await;
}
