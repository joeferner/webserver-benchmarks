use axum::{Router, routing::get};
use signal_hook::iterator::Signals;
use tokio::net::TcpListener;
use tower_http::services::ServeFile;

#[tokio::main]
async fn main() {
    let mut signals =
        Signals::new([signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM]).unwrap();

    std::thread::spawn(move || {
        if let Some(signal) = (&mut signals).into_iter().next() {
            match signal {
                signal_hook::consts::SIGINT => {
                    println!("Received SIGINT! Shutting down gracefully.");
                    std::process::exit(0);
                }
                signal_hook::consts::SIGTERM => {
                    println!("Received SIGTERM! Shutting down.");
                    std::process::exit(0);
                }
                _ => unreachable!(),
            }
        }
    });

    let app = Router::new()
        .route("/benchmark/health", get(get_benchmark_health))
        .route("/benchmark/plain-text", get(get_plain_text))
        .route_service(
            "/benchmark/download-binary",
            ServeFile::new_with_mime("/assets/download-binary.png", &mime::IMAGE_PNG),
        );

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn get_benchmark_health() -> &'static str {
    ""
}

async fn get_plain_text() -> &'static str {
    "Hello, World!"
}
