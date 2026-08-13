#[path = "external_tool/broker.rs"]
mod broker;
#[path = "external_tool/fixtures.rs"]
mod fixtures;
#[path = "external_tool/tests.rs"]
mod tests;

pub(super) use fixtures::external_tool_fixture;
