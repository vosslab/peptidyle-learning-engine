//! Project build, generation, database, fixture, and acceptance tools (WP-F2).

mod application;
mod database;
mod e2e_seed;
mod fixtures;
mod pilot_content;
mod tsgen;

fn main() -> anyhow::Result<()> {
    application::run()
}
