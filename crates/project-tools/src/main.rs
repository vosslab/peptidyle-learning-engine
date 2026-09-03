//! Project build, generation, database, fixture, and acceptance tools.

mod application;
mod database;
mod fixtures;
mod pilot_content;
mod tsgen;

fn main() -> anyhow::Result<()> {
    application::run()
}
