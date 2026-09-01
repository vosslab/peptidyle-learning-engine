//! Private normalized source-item fingerprint for immutable QTI import binding.

use objects::Sha256Checksum;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{QtiProfileContractError, QtiProfileId, QtiProfileVersion};

/// Content-derived identity for one normalized QTI item.
///
/// This supports duplicate detection and QTI import binding. It does not verify
/// separately stored bytes, so it is intentionally a fingerprint rather than a
/// checksum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedQtiItemFingerprint(Sha256Checksum);

impl NormalizedQtiItemFingerprint {
    fn compute(bytes: &[u8]) -> Self {
        Self(Sha256Checksum::compute(bytes))
    }

    pub(crate) fn from_normalized_bytes(bytes: &[u8]) -> Self {
        Self::compute(bytes)
    }
}

impl std::fmt::Display for NormalizedQtiItemFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

pub(super) struct NormalizedQtiItemFingerprintInput<'a> {
    pub(super) profile: QtiProfileId,
    pub(super) profile_version: QtiProfileVersion,
    pub(super) title: &'a str,
    pub(super) prompt_markdown: &'a str,
    pub(super) choices: &'a [(&'a str, &'a str)],
    pub(super) correct_vendor_choice_id: &'a str,
    pub(super) canonical_points: &'a str,
    pub(super) blackboard_defaulted_points: bool,
}

pub(super) fn normalized_qti_item_fingerprint(
    input: &NormalizedQtiItemFingerprintInput<'_>,
) -> Result<NormalizedQtiItemFingerprint, QtiProfileContractError> {
    let points = if input.blackboard_defaulted_points {
        json!({ "kind": "blackboardDefaulted" })
    } else {
        json!({ "kind": "canvasDeclared", "value": input.canonical_points })
    };
    let choices = input
        .choices
        .iter()
        .map(|(vendor_choice_id, visible_text)| {
            json!({ "vendor_choice_id": vendor_choice_id, "visible_text": visible_text })
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&json!({
        "schema": "normalized-profile-item/v1",
        "value": {
            "profile": input.profile.as_str(),
            "profile_version": input.profile_version.as_str(),
            "title": input.title,
            "prompt_markdown": input.prompt_markdown,
            "choices": choices,
            "correct_vendor_choice_id": input.correct_vendor_choice_id,
            "points": points,
        },
    }))
    .map_err(|error| QtiProfileContractError::Serialization(error.to_string()))?;
    Ok(NormalizedQtiItemFingerprint::compute(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(
        profile: QtiProfileId,
        title: &str,
        prompt: &str,
        choices: &[(&str, &str)],
        correct: &str,
        points: &str,
        defaulted: bool,
    ) -> NormalizedQtiItemFingerprint {
        normalized_qti_item_fingerprint(&NormalizedQtiItemFingerprintInput {
            profile,
            profile_version: profile.version(),
            title,
            prompt_markdown: prompt,
            choices,
            correct_vendor_choice_id: correct,
            canonical_points: points,
            blackboard_defaulted_points: defaulted,
        })
        .expect("normalized fingerprint")
    }

    #[test]
    fn normalized_fingerprint_is_golden_and_deterministic() {
        let first = digest(
            QtiProfileId::CANVAS,
            "Favorite color",
            "What is my favorite color?",
            &[("blue_vendor", "Blue"), ("red_vendor", "Red")],
            "blue_vendor",
            "1.0",
            false,
        );
        let second = digest(
            QtiProfileId::CANVAS,
            "Favorite color",
            "What is my favorite color?",
            &[("blue_vendor", "Blue"), ("red_vendor", "Red")],
            "blue_vendor",
            "1.0",
            false,
        );
        assert_eq!(first, second);
        assert_eq!(
            first.to_string(),
            "a55eb724152f79f0ec1c40c0515820a229a16072ac7db33b6ecab3b423e23888"
        );
    }

    #[test]
    fn normalized_fingerprint_is_sensitive_to_retained_source_semantics() {
        let base = digest(
            QtiProfileId::CANVAS,
            "Title",
            "Prompt",
            &[("a", "A"), ("b", "B")],
            "a",
            "1.0",
            false,
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::CANVAS,
                "Other",
                "Prompt",
                &[("a", "A"), ("b", "B")],
                "a",
                "1.0",
                false
            )
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::CANVAS,
                "Title",
                "Other",
                &[("a", "A"), ("b", "B")],
                "a",
                "1.0",
                false
            )
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::CANVAS,
                "Title",
                "Prompt",
                &[("b", "B"), ("a", "A")],
                "a",
                "1.0",
                false
            )
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::CANVAS,
                "Title",
                "Prompt",
                &[("a", "A"), ("other-vendor", "B")],
                "a",
                "1.0",
                false
            )
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::CANVAS,
                "Title",
                "Prompt",
                &[("a", "Other visible text"), ("b", "B")],
                "a",
                "1.0",
                false
            )
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::CANVAS,
                "Title",
                "Prompt",
                &[("a", "A"), ("b", "B")],
                "b",
                "1.0",
                false
            )
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::CANVAS,
                "Title",
                "Prompt",
                &[("a", "A"), ("b", "B")],
                "a",
                "2.0",
                false
            )
        );
        assert_ne!(
            base,
            digest(
                QtiProfileId::BLACKBOARD,
                "Title",
                "Prompt",
                &[("a", "A"), ("b", "B")],
                "a",
                "1.0",
                true
            )
        );
    }
}
