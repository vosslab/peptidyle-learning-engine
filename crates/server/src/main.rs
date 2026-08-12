//! MOD-SRV: production binary entry point.

mod application;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    application::run().await
}
