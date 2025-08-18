use anyhow::Context as _;
use axum::routing::get;
use envconfig::Envconfig;
use tokio::signal;
use tower::Layer as _;
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracking_setup();

    let config = Config::init_from_env().context("failed to get the config")?;

    let addr = std::net::SocketAddrV4::new(config.addr, config.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to listen on address: {}", addr))?;

    tracing::info!("serve at {}", addr);

    let not_found_service = ServeFile::new(
        config
            .served_dir_path
            .join(&config.not_found_page_file_path),
    );
    let serve_dir = ServeDir::new(&config.served_dir_path).not_found_service(not_found_service);

    let app = axum::Router::new()
        .route("/healthcheck/", get(async || "healthy"))
        .fallback_service(serve_dir)
        .layer(TraceLayer::new_for_http());

    let app = NormalizePathLayer::append_trailing_slash().layer(app);
    let app = axum::ServiceExt::<axum::extract::Request>::into_make_service(app);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("failed to serve")
}

#[derive(Clone, Envconfig)]
pub struct Config {
    /// The IP address the server listens on.
    #[envconfig(from = "MY_SITE_WEB_ADDR", default = "0.0.0.0")]
    pub addr: std::net::Ipv4Addr,
    /// The port the server listens on.
    #[envconfig(from = "MY_SITE_WEB_PORT", default = "5000")]
    pub port: u16,
    /// The directory path to serve files from.
    #[envconfig(from = "MY_SITE_WEB_SERVED_DIR_PATH", default = "/data")]
    pub served_dir_path: std::path::PathBuf,
    /// The file to serve when a requested file is not found.
    #[envconfig(
        from = "MY_SITE_WEB_NOT_FOUND_PAGE_FILE_PATH",
        default = "not_found.html"
    )]
    pub not_found_page_file_path: std::path::PathBuf,
}

fn tracking_setup() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .inspect_err(|err| {
            tracing::warn!(
                error = ?err,
                "failed to read env RUST_LOG, fallback to default value"
            );
        })
        .unwrap_or_else(|_| "debug,hyper=off".into());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .pretty();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
