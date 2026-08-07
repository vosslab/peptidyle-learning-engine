//! The four run policies (WP-C3, MOD-RUN).
//!
//! Completion requirement, grade policy, continued practice, and variation
//! policy are independent enums that compose freely. They are kept independent
//! deliberately: collapsing them into one "mode" enum is what makes a system
//! unable to express "mastery required, highest score kept, practice allowed
//! after completion with fresh seeds", which is the behavior the owner
//! observed students actually using.
