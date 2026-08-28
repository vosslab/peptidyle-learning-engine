use super::super::*;

impl MemoryStore {
    /// Captures the complete private Memory state for preview-plane
    /// conformance.  This is an opaque, non-route test seam: callers can only
    /// prove equality or the one permitted derived-preview audit delta.
    pub fn preview_plane_state_effect_fingerprint(
        &self,
    ) -> Result<MemoryPreviewPlaneStateEffectFingerprint, StoreError> {
        let state = self.read_state()?;
        Ok(MemoryPreviewPlaneStateEffectFingerprint::from(&*state))
    }
}

/// Opaque Memory-only conformance snapshot for WP-INST-T3 state effects.
///
/// Application code has no route-callable state snapshot.  Keeping the full
/// state private makes the oracle resilient to new state collections: a
/// preview operation must preserve every collection and current pointer except
/// for one appended private derived-subject audit.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct MemoryPreviewPlaneStateEffectFingerprint {
    state_without_preview_audits: String,
    preview_audits: Vec<crate::PreviewSubjectAudit>,
}

impl From<&State> for MemoryPreviewPlaneStateEffectFingerprint {
    fn from(state: &State) -> Self {
        let mut without_preview_audits = state.clone();
        without_preview_audits.preview_subject_audits.clear();
        Self {
            // The State value is private and has no application serialization.
            // Its complete Debug representation is kept opaque here solely so
            // this conformance seam observes every Memory collection and
            // current pointer without granting test code mutable access.
            state_without_preview_audits: format!("{without_preview_audits:?}"),
            preview_audits: state.preview_subject_audits.clone(),
        }
    }
}

impl MemoryPreviewPlaneStateEffectFingerprint {
    /// Returns whether two Store calls preserved all Memory state exactly.
    pub fn is_unchanged_from(&self, before: &Self) -> bool {
        self.state_without_preview_audits == before.state_without_preview_audits
            && self.preview_audits == before.preview_audits
    }

    /// Returns whether the only state effect is one appended preview audit.
    pub fn has_one_appended_preview_subject_audit_from(&self, before: &Self) -> bool {
        let Some((last, prefix)) = self.preview_audits.split_last() else {
            return false;
        };
        let _ = last;
        self.state_without_preview_audits == before.state_without_preview_audits
            && self.preview_audits.len() == before.preview_audits.len() + 1
            && prefix == before.preview_audits.as_slice()
    }
}
