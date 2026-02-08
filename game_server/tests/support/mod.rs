// Shared primitives for one-time server bootstrapping across integration tests.
use std::{
    // `Arc` shares data between threads; `OnceLock` writes a value only once.
    sync::{Arc, OnceLock},
    // Sleep durations are used in readiness polling loops.
    time::Duration,
};

// Global base URL used by all tests after the server publishes its bound address.
static SERVER_URL: OnceLock<String> = OnceLock::new();
// One-time guard that ensures the server bootstrap path runs only once.
static SERVER_READY: OnceLock<()> = OnceLock::new();

// Ensure the test server is running and return the shared base URL.
pub fn ensure_server() -> &'static str {
    // Run initialization exactly once even if multiple tests call this function.
    SERVER_READY.get_or_init(|| {
        // Local one-time slot where the server thread publishes its selected URL.
        let published_url = Arc::new(OnceLock::<String>::new());
        // Clone so the spawned thread can write into the same shared slot.
        let published_url_thread = Arc::clone(&published_url);
        // Spawn an OS thread so the server outlives individual `#[tokio::test]` runtimes.
        std::thread::spawn(move || {
            // Each server thread owns its own Tokio runtime.
            let runtime = tokio::runtime::Runtime::new().expect("test runtime");
            // Run async server startup and serving on this dedicated runtime.
            runtime.block_on(async move {
                // Bind to an ephemeral port to avoid collisions with local services.
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("bind ephemeral test port");
                // Capture the exact address that was assigned by the OS.
                let addr = listener.local_addr().expect("get local addr");
                // Publish the final base URL so test code can target the right server.
                let _ = published_url_thread.set(format!("http://{}", addr));
                // Start serving requests until the test process exits.
                game_server::run(listener).await.expect("server failed");
            });
        });
        // Block until URL is published and the bound port starts accepting connections.
        wait_for_server_url_and_readiness(published_url);
    });

    // Return the stable shared URL used by all tests in this binary.
    SERVER_URL
        .get()
        .expect("server url should be initialized")
        .as_str()
}

// Wait for URL publication and then wait for the server socket to accept TCP connections.
fn wait_for_server_url_and_readiness(published_url: Arc<OnceLock<String>>) {
    // Poll until the server thread publishes the base URL.
    let base_url = loop {
        // If the URL is published, clone it and stop waiting.
        if let Some(url) = published_url.get() {
            break url.clone();
        }
        // Avoid a tight loop while waiting for the background thread.
        std::thread::sleep(Duration::from_millis(10));
    };

    // Persist the URL globally so every test gets the same endpoint.
    let _ = SERVER_URL.set(base_url.clone());

    // Strip the scheme so we can use host:port for raw TCP readiness checks.
    let addr = base_url
        .strip_prefix("http://")
        .expect("base url should use http://");

    // Retry for a short period to avoid racing server bind/accept.
    for _ in 0..100 {
        // Successful connect means the server socket is accepting connections.
        if std::net::TcpStream::connect(addr).is_ok() {
            return;
        }
        // Wait briefly before the next readiness probe.
        std::thread::sleep(Duration::from_millis(20));
    }

    // Fail fast if startup never reached an accepting state.
    panic!("server did not become ready in time");
}
