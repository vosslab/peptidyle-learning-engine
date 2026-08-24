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

    /// Captures a rehearsal-local effect snapshot without exposing Memory state.
    ///
    /// The snapshot proves that rehearsal calls leave every ordinary learner,
    /// gradebook, analysis, catalog, export, job, and audit collection intact.
    /// It supports comparison only; neither it nor this method exposes the
    /// private state captured for that comparison.
    ///
    /// ```
    /// use learning_data_access::in_memory::MemoryStore;
    ///
    /// let store = MemoryStore::default();
    /// let fingerprint = store
    ///     .rehearsal_state_effect_fingerprint()
    ///     .expect("Memory state is available");
    /// assert_eq!(format!("{fingerprint:?}"), "MemoryRehearsalStateEffectFingerprint([REDACTED])");
    /// ```
    pub fn rehearsal_state_effect_fingerprint(
        &self,
    ) -> Result<MemoryRehearsalStateEffectFingerprint, StoreError> {
        let state = self.read_state()?;
        Ok(MemoryRehearsalStateEffectFingerprint::from(&*state))
    }
}

/// Opaque Memory-only conformance snapshot for WP-PROF-T3 state effects.
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

/// Opaque conformance snapshot for the dedicated rehearsal namespace.
#[doc(hidden)]
#[derive(Clone, PartialEq, Eq)]
pub struct MemoryRehearsalStateEffectFingerprint {
    ordinary_state: String,
    rehearsal_state: String,
}

impl std::fmt::Debug for MemoryRehearsalStateEffectFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("MemoryRehearsalStateEffectFingerprint([REDACTED])")
    }
}

impl From<&State> for MemoryRehearsalStateEffectFingerprint {
    fn from(state: &State) -> Self {
        let mut ordinary = state.clone();
        ordinary.next_rehearsal_reference = 0;
        ordinary.rehearsal_runs.clear();
        ordinary.rehearsal_by_reference.clear();
        ordinary.rehearsal_active_by_owner.clear();
        ordinary.rehearsal_frozen_items.clear();
        ordinary.rehearsal_frozen_source_snapshots.clear();
        ordinary.rehearsal_frozen_private_execution.clear();
        ordinary.rehearsal_evidence.clear();
        ordinary.rehearsal_submission_claims.clear();
        ordinary.rehearsal_start_operations.clear();
        ordinary.rehearsal_delivery_operations.clear();
        ordinary.rehearsal_delivery_retries.clear();
        let rehearsal_state = format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{}{:?}{:?}{:?}{:?}",
            state.next_rehearsal_reference,
            state.rehearsal_runs,
            state.rehearsal_by_reference,
            state.rehearsal_active_by_owner,
            state.rehearsal_frozen_items,
            state.rehearsal_frozen_source_snapshots,
            state.rehearsal_frozen_private_execution,
            rehearsal_evidence_fingerprint(&state.rehearsal_evidence),
            state.rehearsal_submission_claims,
            state.rehearsal_start_operations,
            state.rehearsal_delivery_operations,
            state.rehearsal_delivery_retries
        );
        Self {
            ordinary_state: format!("{ordinary:?}"),
            rehearsal_state,
        }
    }
}

fn rehearsal_evidence_fingerprint(
    evidence: &BTreeMap<
        (TenantId, question_model::RehearsalRunId),
        super::super::rehearsal::StoredRehearsalEvidence,
    >,
) -> String {
    let mut value = String::new();
    for (key, entries) in evidence {
        use std::fmt::Write as _;
        let _ = write!(&mut value, "{key:?}");
        for entry in &entries.0 {
            let _ = write!(
                &mut value,
                "{:?}{}",
                entry.record,
                domain::private_payload_digest(&entry.payload).to_hex()
            );
        }
    }
    value
}

impl MemoryRehearsalStateEffectFingerprint {
    /// Returns whether a rehearsal operation left ordinary learner, scoring,
    /// gradebook, analysis, catalog, export, job, and audit state unchanged.
    ///
    /// Unlike [`Self::is_unchanged_from`], this intentionally permits a
    /// lawful immutable rehearsal evidence or execution-artifact append.
    pub fn has_no_ordinary_effects_from(&self, before: &Self) -> bool {
        self.ordinary_state == before.ordinary_state
    }

    /// Returns whether a refused rehearsal call preserved all state.
    pub fn is_unchanged_from(&self, before: &Self) -> bool {
        self.ordinary_state == before.ordinary_state
            && self.rehearsal_state == before.rehearsal_state
    }

    /// Returns whether a successful rehearsal call changed only rehearsal state.
    pub fn has_only_rehearsal_effects_from(&self, before: &Self) -> bool {
        self.has_no_ordinary_effects_from(before) && self.rehearsal_state != before.rehearsal_state
    }
}
