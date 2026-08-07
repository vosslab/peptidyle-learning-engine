//! Score aggregation and the summary projection (MOD-SCORE).
//!
//! Implemented in M2. The summary projection is a pure function of a run
//! transition, which is what lets the store apply a run update and its summary
//! row in one transaction. Keeping it pure here is what keeps the gradebook's
//! default view to a single summary query no matter how many runs a student
//! has taken.
