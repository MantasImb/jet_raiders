use game_server::frameworks::server::run_with_config;

#[tokio::main]
async fn main() {
    if let Err(failure) = run_with_config().await {
        std::process::exit(failure.exit_code());
    }
}
