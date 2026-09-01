use super::SourceObjectChecksum;

#[test]
fn source_object_checksum_accepts_only_canonical_sha256() {
    let checksum = SourceObjectChecksum::parse("a".repeat(64)).expect("canonical checksum");
    assert_eq!(checksum.as_str(), "a".repeat(64));
    assert_eq!(
        serde_json::from_str::<SourceObjectChecksum>(
            "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""
        )
        .expect("canonical checksum deserializes"),
        checksum
    );

    for invalid in [
        "A".repeat(64),
        "a".repeat(63),
        format!("{}g", "a".repeat(63)),
    ] {
        assert!(SourceObjectChecksum::parse(invalid.clone()).is_err());
        assert!(serde_json::from_str::<SourceObjectChecksum>(&format!("\"{invalid}\"")).is_err());
    }
}
