//! Question Backend capability declarations.
//!
//! Every backend states what it can honestly do. The platform uses that
//! declaration to refuse an assignment configuration *before* publication,
//! rather than failing in front of a student.
//!
//! Design rule from `docs/RUST_STYLE.md` section 9: a capability is a variant
//! of [`Capability`], and the question "does this backend support it?" has
//! exactly one implementation, [`QuestionBackendCapabilities::supports`]. Adding a
//! capability means adding a variant, and every exhaustive `match` over it
//! stops compiling until it is handled, so the compiler finds the call sites
//! that need updating.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// One thing a question backend either can or cannot do.
///
/// The eight variants are the specification's capability set. They are an enum
/// rather than eight struct fields so a caller cannot invent a ninth, and so
/// violations can name the capability they are about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Capability {
    /// Generates fresh parameters per seed, so each student sees a variant.
    AlgorithmicGeneration,
    /// Renders in the browser without a server round trip.
    ClientRendering,
    /// Grades on the server, where answer keys live.
    ///
    /// A backend without this one cannot carry a graded assignment.
    ServerGrading,
    /// Awards partial credit rather than all or nothing.
    PartialCredit,
    /// Supplies hints during an attempt.
    Hints,
    /// Enforces a time limit on a single question.
    QuestionAttemptTimeLimit,
    /// Renders to a printable artifact (DOCX, PDF).
    PrintExport,
    /// Previews offline, with no backend reachable.
    OfflinePreview,
}

impl Capability {
    /// Every capability, in declaration order.
    ///
    /// Used by capability validation and by the instructor UI, which lists
    /// what a backend supports. Keeping the list here means a new variant is
    /// added in exactly one place.
    pub const ALL: [Capability; 8] = [
        Capability::AlgorithmicGeneration,
        Capability::ClientRendering,
        Capability::ServerGrading,
        Capability::PartialCredit,
        Capability::Hints,
        Capability::QuestionAttemptTimeLimit,
        Capability::PrintExport,
        Capability::OfflinePreview,
    ];

    /// A stable machine-readable name, used in violation messages and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::AlgorithmicGeneration => "algorithmicGeneration",
            Capability::ClientRendering => "clientRendering",
            Capability::ServerGrading => "serverGrading",
            Capability::PartialCredit => "partialCredit",
            Capability::Hints => "hints",
            Capability::QuestionAttemptTimeLimit => "questionAttemptTimeLimit",
            Capability::PrintExport => "printExport",
            Capability::OfflinePreview => "offlinePreview",
        }
    }
}

/// What one backend declares it can do.
///
/// A `BTreeSet` rather than a bitfield or eight booleans: iteration order is
/// deterministic, which matters because these values are serialized into the
/// reproducibility record and compared.
///
/// It is a newtype rather than a struct with a named field so serde and the
/// TypeScript generator both treat it as the underlying set: the generated
/// type is `Array<Capability>`, not `{ supported: Array<Capability> }`.
///
/// # Examples
///
/// ```
/// use question_model::capability::{QuestionBackendCapabilities, Capability};
///
/// let browser_renderer = QuestionBackendCapabilities::from_iter([Capability::ClientRendering]);
/// assert!(browser_renderer.supports(Capability::ClientRendering));
///
/// assert!(!browser_renderer.supports(Capability::ServerGrading));
/// assert_eq!(
///     browser_renderer.missing_from([Capability::ServerGrading, Capability::ClientRendering]),
///     vec![Capability::ServerGrading],
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QuestionBackendCapabilities(BTreeSet<Capability>);

impl QuestionBackendCapabilities {
    /// Declares a backend that supports nothing.
    ///
    /// The empty set is the honest default: a backend advertises what it has
    /// implemented, and anything it forgets to declare is treated as absent
    /// rather than assumed present.
    pub fn none() -> Self {
        QuestionBackendCapabilities(BTreeSet::new())
    }

    /// Whether this backend supports one capability.
    pub fn supports(&self, capability: Capability) -> bool {
        self.0.contains(&capability)
    }

    /// Which of the required capabilities this backend lacks.
    ///
    /// Returns every missing capability rather than the first, because an
    /// instructor fixing an assignment wants the whole list, not one error per
    /// save.
    pub fn missing_from(&self, required: impl IntoIterator<Item = Capability>) -> Vec<Capability> {
        required
            .into_iter()
            .filter(|capability| !self.supports(*capability))
            .collect()
    }

    /// Every capability this backend declares, in a deterministic order.
    pub fn declared(&self) -> impl Iterator<Item = Capability> + '_ {
        self.0.iter().copied()
    }
}

impl FromIterator<Capability> for QuestionBackendCapabilities {
    fn from_iter<I: IntoIterator<Item = Capability>>(iter: I) -> Self {
        QuestionBackendCapabilities(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_capability_is_supported() {
        let backend = QuestionBackendCapabilities::from_iter([
            Capability::ServerGrading,
            Capability::AlgorithmicGeneration,
        ]);
        assert!(backend.supports(Capability::ServerGrading));
        assert!(!backend.supports(Capability::Hints));
    }

    #[test]
    fn missing_from_reports_every_gap_not_just_the_first() {
        let backend = QuestionBackendCapabilities::from_iter([Capability::ClientRendering]);
        let missing = backend.missing_from([
            Capability::ServerGrading,
            Capability::ClientRendering,
            Capability::PartialCredit,
        ]);
        assert_eq!(
            missing,
            vec![Capability::ServerGrading, Capability::PartialCredit]
        );
    }

    #[test]
    fn an_undeclared_backend_supports_nothing() {
        let backend = QuestionBackendCapabilities::none();
        assert_eq!(backend.missing_from(Capability::ALL).len(), 8);
    }
}
