//! Project build, generation, database, fixture, and acceptance tools.

mod application;
mod curriculum_content;
mod database;
mod database_coordinator;
mod fixtures;
mod installation_data;
mod installation_data_activity;
mod installation_data_blueprint;
mod libpq_environment;
mod pilot_content;

fn main() -> anyhow::Result<()> {
    application::run()
}
