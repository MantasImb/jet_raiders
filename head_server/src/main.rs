mod domain;
mod frameworks;
mod interface_adapters;
mod use_cases;

use frameworks::server;

#[tokio::main]
async fn main() {
    if let Err(failure) = server::run().await {
        std::process::exit(failure.exit_code());
    }
}
