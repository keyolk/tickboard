mod app;
mod config;
mod error;
mod keymap;
mod models;
mod services;
mod tasks;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    app::App::new().run().await
}
