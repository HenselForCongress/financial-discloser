// src/main.rs
mod build_index;
mod pdf_download;

use std::env;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    init_logger();

    // Initialize Sentry if SENTRY_DSN is set
    let _guard = init_sentry();

    // Send a test event to Sentry
    // send_test_event();

    info!("Starting report generation...");

    if let Err(e) = run_reports().await {
        error!("Error running reports: {:?}", e);

        // Capture the error in Sentry
        sentry::capture_message(
            &format!("Error running reports: {:?}", e),
            sentry::Level::Error
        );
    } else {
        info!("Report generation completed successfully.");
    }

    // Flush Sentry events (if any) before exiting
    if let Some(guard) = _guard {
        guard.flush(Some(std::time::Duration::from_secs(2)));
    }
}

fn send_test_event() {
    // Uncomment the following line to send a test event to Sentry
    sentry::capture_message("This is a test event from Rust application!", sentry::Level::Info);
}

async fn run_reports() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Index Check...");
    build_index::get_updated_index().await?;
    info!("Index Check completed.");

    info!("Starting PDF Downloader...");
    pdf_download::get_pdf_reports().await?;
    info!("PDF Download complete.");

    Ok(())
}

fn init_logger() {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_filter = match log_level.as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_filter)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
}

fn init_sentry() -> Option<sentry::ClientInitGuard> {
    if let Ok(dsn) = env::var("SENTRY_DSN") {
        if !dsn.is_empty() {
            let guard = sentry::init((dsn, sentry::ClientOptions {
                release: sentry::release_name!(),
                // Optionally, set other options here
                ..Default::default()
            }));
            return Some(guard);
        }
    }
    None
}
