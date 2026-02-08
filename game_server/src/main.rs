use game_server::frameworks::server::run_with_config;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    run_with_config().await
}
