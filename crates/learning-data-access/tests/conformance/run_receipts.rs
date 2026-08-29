//! Durable receipt-conformance fixture assembly.

pub(super) use super::*;

#[path = "run_receipts/cross_run_finalization.rs"]
mod cross_run_finalization;
#[path = "run_receipts/fixture_setup.rs"]
mod fixture_setup;
#[path = "run_receipts/fixtures.rs"]
mod fixtures;
#[path = "run_receipts/issued_snapshot_validation.rs"]
mod issued_snapshot_validation;
#[path = "run_receipts/receipt_lifecycle.rs"]
mod receipt_lifecycle;
#[path = "run_receipts/routing_binding.rs"]
mod routing_binding;
#[path = "run_receipts/terminal_receipt.rs"]
mod terminal_receipt;

pub(super) use fixture_setup::{
    exercise_run_api_receipts, exercise_run_api_receipts_with_grade_policy,
};
pub(super) use fixtures::{grading_envelope, receipt_next_attempt, receipt_presentation};
use terminal_receipt::TerminalReceiptFixture;
