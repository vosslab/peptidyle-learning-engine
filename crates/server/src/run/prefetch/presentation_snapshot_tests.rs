use super::*;
use question_model::answer::SelectionCardinality;
use question_model::presentation::rebuild_public_presentation_v1;
use question_model::response::{ChoiceId, HotspotRegion, ResponseDefinition};
use question_model::{AssetId, VersionId, generation::Seed};
use uuid::Uuid;

#[test]
fn hotspot_receipt_snapshot_rebuilds_only_with_its_public_asset_binding() {
    let asset = AssetId::from_uuid(Uuid::from_u128(1));
    let envelope = QuestionEnvelope {
        version: VersionId::from_uuid(Uuid::from_u128(2)),
        seed: Seed::new(3),
        title: "Protein hotspot".to_string(),
        prompt: Vec::new(),
        response: ResponseDefinition::Hotspot {
            surface: AssetRef {
                asset,
                checksum: "a".repeat(64),
            },
            description: "A protein surface.".to_string(),
            regions: vec![HotspotRegion {
                id: ChoiceId::new("active-site"),
                label: vec![ContentBlock::Text {
                    markdown: "Active site".to_string(),
                }],
                x: 1_000,
                y: 2_000,
                width: 3_000,
                height: 2_000,
            }],
            selection: SelectionCardinality::ExactlyOne,
        },
    };

    // A backend that issues hotspots provides the measured public asset
    // dimensions. The receipt preserves those descriptor inputs verbatim;
    // it does not infer a size from mutable object metadata on replay.
    let issued = build_presentation_v1(
        &envelope,
        &[AssetBindingV1 {
            asset,
            authored_checksum: "a".repeat(64),
            rendition_checksum: "b".repeat(64),
            intrinsic_width: Some(1_024),
            intrinsic_height: Some(768),
        }],
    )
    .expect("asset-backed hotspot is presentable");
    let replayed = rebuild_public_presentation_v1(&issued.envelope, &issued.asset_bindings)
        .expect("receipt snapshot retains every descriptor input");
    assert_eq!(replayed.digest, issued.digest);
    assert_eq!(issued.asset_bindings.len(), 1);
    assert!(rebuild_public_presentation_v1(&issued.envelope, &[]).is_err());
}
