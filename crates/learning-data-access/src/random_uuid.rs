//! Standards-correct cryptographically random UUID generation.

use uuid::Uuid;

pub(crate) fn random_128_bits<E>(
    map_error: impl FnOnce(getrandom::Error) -> E,
) -> Result<[u8; 16], E> {
    let mut bytes = [0_u8; 16];
    // ASVS 11.2.1 and 11.5.1: use the operating-system CSPRNG through getrandom.
    getrandom::fill(&mut bytes).map_err(map_error)?;
    Ok(bytes)
}

pub(crate) fn random_uuid_v4<E>(map_error: impl FnOnce(getrandom::Error) -> E) -> Result<Uuid, E> {
    random_128_bits(map_error).map(uuid_v4_from_bytes)
}

/// Draws a uniformly random integer from the operating-system CSPRNG.
pub(crate) fn random_u64<E>(map_error: impl FnOnce(getrandom::Error) -> E) -> Result<u64, E> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).map_err(map_error)?;
    Ok(u64::from_be_bytes(bytes))
}

const MAX_BROWSER_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

fn browser_safe_question_seed(value: u64) -> u64 {
    value & MAX_BROWSER_SAFE_INTEGER
}

/// Draws a uniformly random Question seed that remains exact in browser JSON.
///
/// Question variation is evidence retained with an Issued Question, so its
/// value is minted by the trusted application boundary rather than inferred
/// by PostgreSQL from mutable Assessment state.
pub(crate) fn random_question_seed<E>(
    map_error: impl FnOnce(getrandom::Error) -> E,
) -> Result<u64, E> {
    // Bounding the operating-system random value matches the existing
    // JavaScript-number contract for persisted evidence.
    random_u64(map_error).map(browser_safe_question_seed)
}

fn uuid_v4_from_bytes(mut bytes: [u8; 16]) -> Uuid {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
