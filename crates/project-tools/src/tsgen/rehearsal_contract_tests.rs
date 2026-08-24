use std::fs;

use super::run;

fn temporary_output_dir(label: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("test clock should be after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ple-tsgen-{label}-{}-{nonce}", std::process::id()))
}

#[test]
fn rehearsal_public_generation_excludes_legacy_outcome_and_private_dependencies() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_dir = manifest_dir.join("../question_model/src");
    let out_dir = temporary_output_dir("rehearsal-public-contract");

    run(&model_dir, &out_dir).expect("question model generation succeeds");
    assert!(
        !out_dir.join("RehearsalPublicOutcome.ts").exists(),
        "internal Store outcomes must never become generated browser contracts"
    );

    for name in [
        "RehearsalRouteViewV1.ts",
        "RehearsalActiveScreenV1.ts",
        "RehearsalQuestionPresentationV1.ts",
        "RehearsalSubmissionRequestV1.ts",
        "RehearsalSubmissionResultV1.ts",
    ] {
        let generated = fs::read_to_string(out_dir.join(name))
            .unwrap_or_else(|error| panic!("{name} must be generated: {error}"));
        assert!(
            !generated.contains("RehearsalPublicOutcome"),
            "{name} must use the V1 route DTO family"
        );
        assert!(
            !generated.contains("AssetId"),
            "{name} must not expose storage asset identities"
        );
        assert!(
            !generated.contains("{ DisclosedFeedback }"),
            "{name} must not reach the legacy feedback DTO"
        );
    }

    fs::remove_dir_all(out_dir).expect("temporary output should be removed");
}
