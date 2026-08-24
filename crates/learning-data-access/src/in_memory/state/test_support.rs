#[cfg(feature = "test-support")]
mod corruption;
mod fingerprints;

#[cfg(feature = "test-support")]
pub use corruption::{
    MemoryRehearsalClaimTestSnapshot, MemoryRehearsalIntegrityTestCorruption,
    MemoryRehearsalTestSnapshot,
};
