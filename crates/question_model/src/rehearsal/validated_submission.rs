//! Private-server validation boundary for rendered rehearsal submissions.
//!
//! The browser sends a public digest prefix and rendered identifiers. This
//! type is constructed only after those inputs have been checked against the
//! active screen and retains the full server commitment needed by downstream
//! grading. It intentionally has no wire or persistence representation.

use std::fmt;

use crate::{
    RehearsalActiveScreenV1, RehearsalPresentationDigestV1, RehearsalSubmissionRequestV1,
    RehearsalWireValidationError, StudentResponse,
};

/// A rendered rehearsal response validated against one active screen.
///
/// This is an owned, private-server boundary value. It has no `Clone`,
/// serialization, or durable identifiers: callers must move it into the
/// sealed grading flow after `try_from_active_screen` binds the request to the
/// full presentation commitment. This keeps the public prefix token from
/// becoming the authority for a rendered response.
pub struct ValidatedRehearsalRenderedSubmissionV1 {
    response: StudentResponse,
    presentation_commitment: RehearsalPresentationDigestV1,
}

impl ValidatedRehearsalRenderedSubmissionV1 {
    /// Validates a browser request against the exact active screen and binds
    /// the accepted response to that screen's full private commitment.
    ///
    /// `validate_for_screen` is intentionally the sole response-schema
    /// authority here. It performs the bounded, exact identifier and
    /// cardinality checks at the trusted server boundary (ASVS V2.2.1,
    /// V2.2.2), while the retained complete commitment prevents a caller from
    /// advancing the submission workflow with only the browser-safe prefix
    /// (ASVS V2.3.1).
    pub fn try_from_active_screen(
        request: RehearsalSubmissionRequestV1,
        screen: &RehearsalActiveScreenV1,
    ) -> Result<Self, RehearsalWireValidationError> {
        request.validate_for_screen(screen)?;
        let presentation_commitment = screen.commitment()?;

        Ok(Self {
            response: request.response,
            presentation_commitment,
        })
    }

    /// Borrows the original rendered response after exact screen validation.
    pub fn response(&self) -> &StudentResponse {
        &self.response
    }

    /// Returns the full server-only commitment of the active screen.
    pub fn presentation_commitment(&self) -> RehearsalPresentationDigestV1 {
        self.presentation_commitment
    }

    /// Transfers the rendered response and its full screen commitment to the
    /// next private-server boundary.
    pub fn into_parts(self) -> (StudentResponse, RehearsalPresentationDigestV1) {
        (self.response, self.presentation_commitment)
    }
}

impl fmt::Debug for ValidatedRehearsalRenderedSubmissionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedRehearsalRenderedSubmissionV1")
            .field("response", &"<redacted>")
            .field("presentation_commitment", &"<redacted>")
            .finish()
    }
}
