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

/// Encodes all 128 random bits in a UUID-shaped private storage column.
pub(crate) fn uuid_storage_from_128_random_bits(bytes: [u8; 16]) -> Uuid {
    Uuid::from_bytes(bytes)
}

fn uuid_v4_from_bytes(mut bytes: [u8; 16]) -> Uuid {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use uuid::{Variant, Version};

    use super::*;

    #[test]
    fn uuid_v4_normalization_sets_zero_version_and_variant_bits() {
        let bytes = [0_u8; 16];

        assert_eq!(
            *uuid_v4_from_bytes(bytes).as_bytes(),
            [0, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn uuid_v4_normalization_masks_only_version_and_variant_bits() {
        let bytes = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0xff, 0x77, 0xff, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];

        assert_eq!(
            *uuid_v4_from_bytes(bytes).as_bytes(),
            [
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x4f, 0x77, 0xbf, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff,
            ]
        );
    }

    #[test]
    fn uuid_storage_encoding_preserves_all_random_bits() {
        let bytes = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];

        assert_eq!(*uuid_storage_from_128_random_bits(bytes).as_bytes(), bytes);
    }

    #[test]
    fn operating_system_random_uuid_is_v4_with_rfc_variant() {
        let value = random_uuid_v4(|error| error).expect("UUID randomness should be available");

        assert_eq!(value.get_version(), Some(Version::Random));
        assert_eq!(value.get_variant(), Variant::RFC4122);
    }
}
