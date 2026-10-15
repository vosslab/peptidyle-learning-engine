//! Project build, generation, database, fixture, and acceptance tools.

mod application;
mod database;
mod database_coordinator;
mod installation_data;
mod installation_data_activity;
mod libpq_environment;

fn main() -> anyhow::Result<()> {
    application::run()
}
