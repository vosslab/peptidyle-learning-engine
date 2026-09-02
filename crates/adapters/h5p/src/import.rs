//! Key-free H5P import into the internal question model (MOD-ADP-H5P).
//!
//! Native H5P evaluates responses in the browser.  It is consequently a
//! practice format, not a server-graded backend: this module deliberately
//! has no dependency on `grading` and never accepts, derives, or returns an
//! answer key.  Supported package content is translated to a key-free,
//! unpublished draft payload that a renderer can display.  An author who needs
//! a graded activity must author or convert it as a server-graded internal
//! question, outside this adapter.

use std::collections::BTreeSet;
use std::fmt;

use async_trait::async_trait;
use question_model::ObjectId;
use question_model::QuestionContentBlock;
use question_model::answer::ResponseSelectionRule;
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::generation::QuestionVariationRule;
use question_model::question_content::{QuestionGradingRule, QuestionMetadata};
use question_model::response::{QuestionChoice, QuestionResponseFormat, ResponseItemReference};
use sha2::{Digest, Sha256};

/// Version of the persisted H5P import record schema.
///
/// This is deliberately independent of the repository CalVer release version:
/// it changes only when the durable import record needs a migration.
pub const IMPORT_SCHEMA_VERSION: u16 = 2;
/// The only native H5P content type currently converted by this small,
/// explicit importer.
pub const MULTI_CHOICE_CONTENT_TYPE: &str = "H5P.MultiChoice";

/// Archival identity and trusted import location for an H5P package.
///
/// The object-store package is authoritative for re-import. The remote
/// reference is import-location metadata only and is intentionally kept out of
/// Question Backend records.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct H5pPackageImportReference {
    /// H5P library name, such as `H5P.MultiChoice`.
    pub content_type: String,
    /// Original remote package reference supplied by the trusted importer.
    pub remote_package_reference: String,
    /// SHA-256 of the exact H5P package bytes verified by the import worker.
    ///
    /// This is retained with the sandbox import so a later re-import can prove
    /// which remote package was converted even if the remote URL is replaced.
    pub package_sha256: String,
    /// Immutable object-store record for the exact package bytes.
    ///
    /// This is mandatory for an accepted import. A mutable remote URL is
    /// import-location metadata only: the publication worker must be able to retrieve these
    /// bytes after the remote host changes or disappears. Remote-only content
    /// may be displayed as a degraded draft preview, but cannot become an
    /// `ImportedH5pQuestion` or enter publication.
    pub stored_package_object: ObjectId,
}

/// Exact H5P package bytes retrieved from the trusted archival store.
///
/// This deliberately contains no location or signed URL. The resolver owns
/// the mapping from an immutable [`ObjectId`] to the physical object-store
/// key, keeping storage topology out of the H5P conversion boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedH5pPackage {
    /// Object identity requested by the import record.
    pub object: ObjectId,
    /// Unmodified `.h5p` package bytes.
    pub bytes: Vec<u8>,
}

/// Failure from the trusted object-storage lookup boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum H5pArchiveError {
    /// The immutable archival record no longer resolves.
    NotFound,
    /// The archival backend is unavailable without disclosing infrastructure.
    Unavailable(String),
}

impl fmt::Display for H5pArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "archived H5P package was not found"),
            Self::Unavailable(message) => {
                write!(formatter, "archived H5P package is unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for H5pArchiveError {}

/// Trusted lookup from an immutable H5P object identity to archived bytes.
///
/// The worker or storage layer implements this trait with object storage. It
/// must resolve the same immutable object record, never fetch the mutable
/// remote URL in [`H5pPackageImportReference`].
#[async_trait]
pub trait H5pArchiveResolver: Send + Sync {
    /// Retrieves exactly the requested archival object.
    async fn get_archived_h5p(
        &self,
        object: ObjectId,
    ) -> Result<ArchivedH5pPackage, H5pArchiveError>;
}

/// Deterministic identity for the exact remote source an import came from.
///
/// It is a SHA-256 digest of length-delimited UTF-8 fields, rather than a
/// delimiter-joined string, so distinct field pairs cannot collide through a
/// separator appearing in a URI or content type.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct H5pPackageImportFingerprint(String);

impl H5pPackageImportFingerprint {
    /// Returns the hexadecimal SHA-256 source identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One visible option from a supported H5P multiple-choice activity.
///
/// There is intentionally no correct-answer field.  Native H5P's answer
/// evaluation remains client-side practice behavior and never becomes a key
/// in this platform's server-graded model.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct H5pChoice {
    /// Stable source-local identifier for this option.
    pub id: String,
    /// Restricted Markdown shown to the student.
    pub markdown: String,
}

/// A source feature that this narrow H5P conversion deliberately does not map.
///
/// The archive/parser boundary records every such feature before calling this
/// importer.  Conversion then fails closed, leaving the original package and
/// this structured report available for an instructor to keep as native H5P
/// practice or simplify before re-importing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct H5pUnsupportedFeature {
    /// Source path where the feature was found, such as `content.params.media`.
    pub location: String,
    /// Stable source-feature label suitable for UI filtering.
    pub feature: String,
    /// Why this adapter cannot preserve the feature faithfully.
    pub reason: String,
    /// Concrete next action for the author.
    pub recovery: String,
}

/// The trusted, already-extracted fields accepted by the H5P importer.
///
/// Archive fetching, validation, and package persistence stay outside this
/// adapter's pure conversion boundary.  This allows worker code to apply its
/// hostile-input protections before constructing this value.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct H5pImportRequest {
    /// Archived package import retained for deterministic re-import.
    pub package_import: H5pPackageImportReference,
    /// Browser-safe Question Library metadata.
    pub metadata: QuestionMetadata,
    /// Prompt in restricted Markdown.
    pub prompt_markdown: String,
    /// Visible choices from the H5P activity.
    pub choices: Vec<H5pChoice>,
    /// Features detected by the archive/parser boundary that this conversion
    /// does not map.  They are retained as structured data and cause a safe
    /// refusal rather than being silently discarded.
    pub unsupported_features: Vec<H5pUnsupportedFeature>,
    /// Practice retry policy.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// Timing policy requested by the source.
    ///
    /// Native H5P timing is not claimed by this adapter, so any timed request
    /// is rejected explicitly rather than silently downgraded.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
}

/// A converted, unpublished H5P sandbox draft.
///
/// This intentionally is not [`QuestionRevision`]: an H5P import has no
/// published Question or version identity. The Question Library's publication transition
/// owns that identity assignment, preserving the invariant that sandbox
/// imports cannot masquerade as immutable published content.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedH5pQuestion {
    /// Version of the internal record shape used for this import.
    pub import_schema_version: u16,
    /// Authoritative archived H5P package reference retained for re-import.
    pub package_import: H5pPackageImportReference,
    /// Prompt ready for the browser-safe renderer.
    pub prompt: Vec<QuestionContentBlock>,
    /// Response shape, without a correct answer.
    pub response: QuestionResponseFormat,
    /// Practice retry behavior.
    pub question_attempt_limit: QuestionAttemptLimit,
    /// The only timing policy currently supported by the adapter.
    pub question_attempt_time_limit: QuestionAttemptTimeLimit,
    /// H5P imports are static until a server-owned generator is selected.
    pub question_variation_rule: QuestionVariationRule,
    /// Always `Ungraded` for native H5P practice.
    pub grading: QuestionGradingRule,
    /// Browser-safe title, Question Classification, and licensing metadata.
    pub metadata: QuestionMetadata,
    /// Deterministic identity of the exact source package and reference.
    pub package_import_fingerprint: H5pPackageImportFingerprint,
}

/// H5P Package Import boundary.
#[derive(Debug, Default, Clone, Copy)]
pub struct H5pImporter;

impl H5pImporter {
    /// Converts one supported H5P activity into a key-free internal question.
    ///
    /// # Errors
    ///
    /// Fails closed for unsupported content types, malformed source identity
    /// inputs, unsupported timing, and invalid multiple-choice structure.
    pub fn import(&self, request: H5pImportRequest) -> Result<ImportedH5pQuestion, H5pImportError> {
        let package_import = validate_and_normalize_package_import(request.package_import)?;
        if package_import.content_type != MULTI_CHOICE_CONTENT_TYPE {
            return Err(H5pImportError::UnsupportedContentType(
                package_import.content_type,
            ));
        }
        if !request.unsupported_features.is_empty() {
            return Err(H5pImportError::UnsupportedFeatures(
                request.unsupported_features,
            ));
        }
        if !matches!(
            request.question_attempt_time_limit,
            QuestionAttemptTimeLimit::Unlimited
        ) {
            return Err(H5pImportError::UnsupportedTiming);
        }
        if request.prompt_markdown.trim().is_empty() {
            return Err(H5pImportError::EmptyPrompt);
        }

        let choices = normalize_choices(request.choices)?;
        let package_import_fingerprint = package_import_fingerprint(&package_import);
        Ok(ImportedH5pQuestion {
            import_schema_version: IMPORT_SCHEMA_VERSION,
            package_import: package_import.clone(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: request.prompt_markdown,
            }],
            response: QuestionResponseFormat::MultipleChoice {
                choices,
                selection: ResponseSelectionRule::ExactlyOne,
            },
            question_attempt_limit: request.question_attempt_limit,
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            question_variation_rule: QuestionVariationRule::Static,
            grading: QuestionGradingRule::Ungraded,
            metadata: request.metadata,
            package_import_fingerprint,
        })
    }

    /// Retrieves an archived package, verifies its bytes, then converts the
    /// parser result that was derived from those verified bytes.
    ///
    /// The parser closure is intentionally supplied by the hostile-input
    /// worker: this small adapter does not parse H5P ZIP files itself. The
    /// closure receives only the verified package bytes and the validated H5P
    /// Package Import Reference. Its returned request must retain that reference
    /// exactly, preventing a parser from accidentally reattaching a different archive.
    ///
    /// # Errors
    ///
    /// Refuses missing objects, resolver identity mismatches, checksum
    /// mismatches, and parser requests that do not retain the verified source.
    pub async fn reimport_from_archive<R, P>(
        &self,
        resolver: &R,
        package_import: H5pPackageImportReference,
        parse_verified_package: P,
    ) -> Result<ImportedH5pQuestion, H5pImportError>
    where
        R: H5pArchiveResolver + ?Sized,
        P: FnOnce(&[u8], H5pPackageImportReference) -> Result<H5pImportRequest, H5pImportError>,
    {
        let package_import = validate_and_normalize_package_import(package_import)?;
        let archived = resolver
            .get_archived_h5p(package_import.stored_package_object)
            .await
            .map_err(H5pImportError::Archive)?;
        if archived.object != package_import.stored_package_object {
            return Err(H5pImportError::ArchiveObjectMismatch {
                expected: package_import.stored_package_object,
                actual: archived.object,
            });
        }

        let retrieved_h5p_package_checksum = sha256_hex(&archived.bytes);
        if retrieved_h5p_package_checksum != package_import.package_sha256 {
            return Err(H5pImportError::ArchiveChecksumMismatch {
                accepted_h5p_package_checksum: package_import.package_sha256,
                retrieved_h5p_package_checksum,
            });
        }

        let request = parse_verified_package(&archived.bytes, package_import.clone())?;
        let parsed_package_import =
            validate_and_normalize_package_import(request.package_import.clone())?;
        if parsed_package_import != package_import {
            return Err(H5pImportError::ReimportPackageImportMismatch);
        }
        self.import(request)
    }
}

/// A rejected source or conversion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum H5pImportError {
    /// The required archival source object could not be resolved.
    Archive(H5pArchiveError),
    /// A resolver returned a different object than the one requested.
    ArchiveObjectMismatch {
        /// Immutable object identity retained by the archived H5P package reference.
        expected: ObjectId,
        /// Object identity returned by the resolver.
        actual: ObjectId,
    },
    /// The retrieved H5P Package Checksum differs from the accepted H5P
    /// Package Checksum retained by the archived H5P package reference.
    ArchiveChecksumMismatch {
        /// Accepted H5P Package Checksum retained at accepted import.
        accepted_h5p_package_checksum: String,
        /// Retrieved H5P Package Checksum recomputed from the archive bytes.
        retrieved_h5p_package_checksum: String,
    },
    /// A re-import parser attempted to substitute a different archived H5P package.
    ReimportPackageImportMismatch,
    /// The external package reference was absent or whitespace-only.
    EmptyRemotePackageReference,
    /// The package checksum was missing or was not a SHA-256 hexadecimal digest.
    InvalidPackageSha256,
    /// The H5P library name was absent or whitespace-only.
    EmptyContentType,
    /// This adapter has no safe conversion for the supplied H5P library.
    UnsupportedContentType(String),
    /// Conversion refused because otherwise-visible source features would be
    /// dropped.  The complete parser report is retained for author recovery.
    UnsupportedFeatures(Vec<H5pUnsupportedFeature>),
    /// The imported prompt cannot render a meaningful question.
    EmptyPrompt,
    /// Native H5P timing cannot honestly be declared through this adapter.
    UnsupportedTiming,
    /// A multiple-choice activity needs at least two options.
    TooFewChoices,
    /// A choice lacks an opaque source-local identifier.
    EmptyChoiceId,
    /// Choice identifiers must be unique so a student response is unambiguous.
    DuplicateChoiceId(String),
    /// A choice has no visible student-facing text.
    EmptyChoiceBody(String),
}

impl fmt::Display for H5pImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(error) => write!(
                formatter,
                "could not retrieve archived H5P package: {error}"
            ),
            Self::ArchiveObjectMismatch { expected, actual } => write!(
                formatter,
                "archived H5P resolver returned object `{actual}` instead of `{expected}`"
            ),
            Self::ArchiveChecksumMismatch {
                accepted_h5p_package_checksum,
                retrieved_h5p_package_checksum,
            } => write!(
                formatter,
                "retrieved H5P Package Checksum `{retrieved_h5p_package_checksum}` differs from accepted H5P Package Checksum `{accepted_h5p_package_checksum}`; preserve the original package and investigate storage integrity"
            ),
            Self::ReimportPackageImportMismatch => write!(
                formatter,
                "H5P re-import parser did not retain the verified archival source"
            ),
            Self::EmptyRemotePackageReference => {
                write!(formatter, "H5P remote package reference must not be empty")
            }
            Self::InvalidPackageSha256 => write!(
                formatter,
                "H5P package SHA-256 must be exactly 64 hexadecimal characters; verify the downloaded package and re-import it"
            ),
            Self::EmptyContentType => write!(formatter, "H5P content type must not be empty"),
            Self::UnsupportedContentType(content_type) => write!(
                formatter,
                "H5P content type `{content_type}` is not supported; keep it as native ungraded H5P practice or convert it to {MULTI_CHOICE_CONTENT_TYPE} and re-import"
            ),
            Self::UnsupportedFeatures(features) => write!(
                formatter,
                "H5P import would drop {} unsupported feature(s); keep it as native ungraded H5P practice or remove the listed features and re-import",
                features.len()
            ),
            Self::EmptyPrompt => write!(formatter, "H5P multiple-choice prompt must not be empty"),
            Self::UnsupportedTiming => write!(
                formatter,
                "H5P timing is not supported by this ungraded-practice adapter"
            ),
            Self::TooFewChoices => write!(
                formatter,
                "H5P multiple-choice import requires at least two visible choices"
            ),
            Self::EmptyChoiceId => write!(formatter, "H5P choice identifier must not be empty"),
            Self::DuplicateChoiceId(id) => {
                write!(formatter, "H5P choice identifier `{id}` is duplicated")
            }
            Self::EmptyChoiceBody(id) => {
                write!(formatter, "H5P choice `{id}` must have visible text")
            }
        }
    }
}

impl std::error::Error for H5pImportError {}

fn validate_and_normalize_package_import(
    mut package_import: H5pPackageImportReference,
) -> Result<H5pPackageImportReference, H5pImportError> {
    if package_import.content_type.trim().is_empty() {
        return Err(H5pImportError::EmptyContentType);
    }
    if package_import.remote_package_reference.trim().is_empty() {
        return Err(H5pImportError::EmptyRemotePackageReference);
    }
    if package_import.package_sha256.len() != 64
        || !package_import
            .package_sha256
            .as_bytes()
            .iter()
            .all(u8::is_ascii_hexdigit)
    {
        return Err(H5pImportError::InvalidPackageSha256);
    }
    package_import.package_sha256.make_ascii_lowercase();
    Ok(package_import)
}

fn package_import_fingerprint(
    package_import: &H5pPackageImportReference,
) -> H5pPackageImportFingerprint {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-h5p-source-identity-v2");
    for value in [
        &package_import.content_type,
        &package_import.remote_package_reference,
        &package_import.package_sha256,
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    let value = package_import.stored_package_object.to_string();
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut hexadecimal = String::with_capacity(digest.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        hexadecimal.push(HEX[usize::from(byte >> 4)] as char);
        hexadecimal.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    H5pPackageImportFingerprint(hexadecimal)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hexadecimal = String::with_capacity(digest.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        hexadecimal.push(HEX[usize::from(byte >> 4)] as char);
        hexadecimal.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    hexadecimal
}

fn normalize_choices(choices: Vec<H5pChoice>) -> Result<Vec<QuestionChoice>, H5pImportError> {
    if choices.len() < 2 {
        return Err(H5pImportError::TooFewChoices);
    }
    let mut ids = BTreeSet::new();
    choices
        .into_iter()
        .map(|choice| {
            if choice.id.trim().is_empty() {
                return Err(H5pImportError::EmptyChoiceId);
            }
            if !ids.insert(choice.id.clone()) {
                return Err(H5pImportError::DuplicateChoiceId(choice.id));
            }
            if choice.markdown.trim().is_empty() {
                return Err(H5pImportError::EmptyChoiceBody(choice.id));
            }
            Ok(QuestionChoice {
                id: ResponseItemReference::new(choice.id),
                body: vec![QuestionContentBlock::Text {
                    markdown: choice.markdown,
                }],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::QuestionContentBlock;
    use question_model::assignment_activity_rules::QuestionAttemptTimeLimit;
    use question_model::classification::QuestionLicense;
    use question_model::question_content::QuestionGradingRule;
    use uuid::Uuid;

    const ARCHIVE_BYTES: &[u8] = b"fixture h5p archive bytes";

    struct FixtureArchiveResolver {
        package: Option<ArchivedH5pPackage>,
    }

    #[async_trait::async_trait]
    impl H5pArchiveResolver for FixtureArchiveResolver {
        async fn get_archived_h5p(
            &self,
            _object: ObjectId,
        ) -> Result<ArchivedH5pPackage, H5pArchiveError> {
            self.package.clone().ok_or(H5pArchiveError::NotFound)
        }
    }

    fn request() -> H5pImportRequest {
        H5pImportRequest {
            package_import: H5pPackageImportReference {
                content_type: MULTI_CHOICE_CONTENT_TYPE.to_string(),
                remote_package_reference: "https://h5p.example.edu/content/peptide-bonds-1.h5p"
                    .to_string(),
                package_sha256: "f760739feba129fa2861ab5badca6327d5235eef5ef6b67b149b20aa7b512077"
                    .to_string(),
                stored_package_object: ObjectId::from_uuid(Uuid::from_u128(0x81)),
            },
            metadata: QuestionMetadata {
                title: "Peptide bonds practice".to_string(),
                question_description: "Instructor-facing peptide-bond practice summary."
                    .to_string(),
                tags: Vec::new(),
                classifications: Vec::new(),
                question_license: Some(QuestionLicense::CcBy4_0),
                question_citation: None,
                language: "en-US".to_string(),
            },
            prompt_markdown: "Which linkage joins amino acids?".to_string(),
            choices: vec![
                H5pChoice {
                    id: "amide".to_string(),
                    markdown: "Amide bond".to_string(),
                },
                H5pChoice {
                    id: "ester".to_string(),
                    markdown: "Ester bond".to_string(),
                },
            ],
            unsupported_features: Vec::new(),
            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        }
    }

    #[test]
    fn supported_h5p_import_becomes_a_key_free_ungraded_internal_question() {
        let mut import_request = request();
        import_request
            .package_import
            .package_sha256
            .make_ascii_uppercase();
        let imported = H5pImporter
            .import(import_request)
            .expect("supported H5P imports");
        assert_eq!(imported.grading, QuestionGradingRule::Ungraded);
        assert_eq!(imported.import_schema_version, IMPORT_SCHEMA_VERSION);
        assert_eq!(imported.package_import, request().package_import);
        assert_eq!(
            imported.question_attempt_time_limit,
            QuestionAttemptTimeLimit::Unlimited
        );
        assert!(matches!(
            imported.prompt.as_slice(),
            [QuestionContentBlock::Text { markdown }] if markdown == "Which linkage joins amino acids?"
        ));
        assert!(matches!(
            imported.response,
            QuestionResponseFormat::MultipleChoice {
                selection: ResponseSelectionRule::ExactlyOne,
                ..
            }
        ));
    }

    #[test]
    fn package_import_fingerprint_is_deterministic_and_binds_exact_package_bytes() {
        let importer = H5pImporter;
        let first = importer.import(request()).expect("first import");
        let repeat = importer.import(request()).expect("repeat import");
        assert_eq!(
            first.package_import_fingerprint,
            repeat.package_import_fingerprint
        );
        assert_eq!(first.package_import_fingerprint.as_str().len(), 64);

        let mut altered_bytes = request();
        altered_bytes.package_import.package_sha256 =
            "6af75b2ddb2dc6d40d11e1c184b10b2a695bc2ebf84b87d90b8c35f8a76d9dc5".to_string();
        let changed = importer
            .import(altered_bytes)
            .expect("same URL with different package bytes imports distinctly");
        assert_ne!(
            first.package_import_fingerprint,
            changed.package_import_fingerprint
        );
    }

    #[test]
    fn accepted_import_requires_and_retains_the_ple_controlled_package_object() {
        let request = request();
        let object = request.package_import.stored_package_object;
        let imported = H5pImporter
            .import(request)
            .expect("stored H5P package imports");
        assert_eq!(imported.package_import.stored_package_object, object);
    }

    #[tokio::test]
    async fn reimport_retrieves_and_reverifies_the_archived_package_before_conversion() {
        let source = request().package_import;
        let resolver = FixtureArchiveResolver {
            package: Some(ArchivedH5pPackage {
                object: source.stored_package_object,
                bytes: ARCHIVE_BYTES.to_vec(),
            }),
        };

        let imported = H5pImporter
            .reimport_from_archive(&resolver, source.clone(), |bytes, verified_source| {
                assert_eq!(bytes, ARCHIVE_BYTES);
                let mut parsed = request();
                parsed.package_import = verified_source;
                Ok(parsed)
            })
            .await
            .expect("verified archive can be re-imported");

        assert_eq!(imported.package_import, source);
        assert_eq!(imported.grading, QuestionGradingRule::Ungraded);
    }

    #[tokio::test]
    async fn reimport_refuses_a_missing_archival_object() {
        let error = H5pImporter
            .reimport_from_archive(
                &FixtureArchiveResolver { package: None },
                request().package_import,
                |_bytes, _source| unreachable!("parser runs only after retrieval"),
            )
            .await
            .expect_err("a remote URL is not an archival fallback");
        assert_eq!(error, H5pImportError::Archive(H5pArchiveError::NotFound));
    }

    #[tokio::test]
    async fn reimport_refuses_bytes_that_do_not_match_the_retained_checksum() {
        let source = request().package_import;
        let error = H5pImporter
            .reimport_from_archive(
                &FixtureArchiveResolver {
                    package: Some(ArchivedH5pPackage {
                        object: source.stored_package_object,
                        bytes: b"changed package bytes".to_vec(),
                    }),
                },
                source.clone(),
                |_bytes, _source| unreachable!("parser runs only after checksum verification"),
            )
            .await
            .expect_err("changed archived bytes must not be converted");
        assert!(matches!(
            error,
            H5pImportError::ArchiveChecksumMismatch {
                accepted_h5p_package_checksum,
                retrieved_h5p_package_checksum,
            } if accepted_h5p_package_checksum == source.package_sha256
                && retrieved_h5p_package_checksum == sha256_hex(b"changed package bytes")
        ));
    }

    #[test]
    fn unsupported_features_fail_explicitly_instead_of_being_silently_downgraded() {
        let mut unsupported_type = request();
        unsupported_type.package_import.content_type = "H5P.DragText".to_string();
        assert_eq!(
            H5pImporter.import(unsupported_type),
            Err(H5pImportError::UnsupportedContentType(
                "H5P.DragText".to_string()
            ))
        );

        let mut timed = request();
        timed.question_attempt_time_limit = QuestionAttemptTimeLimit::Limited {
            seconds: 30,
            grace_seconds: 2,
        };
        assert_eq!(
            H5pImporter.import(timed),
            Err(H5pImportError::UnsupportedTiming)
        );

        let mut rich_multiple_choice = request();
        rich_multiple_choice.unsupported_features = vec![H5pUnsupportedFeature {
            location: "content.params.behaviour.enableRetry".to_string(),
            feature: "retry-behavior".to_string(),
            reason: "the initial conversion has no faithful equivalent for H5P retry behavior"
                .to_string(),
            recovery: "Keep this activity as native ungraded H5P practice, or remove retry behavior and re-import."
                .to_string(),
        }];
        let error = H5pImporter
            .import(rich_multiple_choice)
            .expect_err("unmapped nested feature must not be silently dropped");
        assert!(matches!(
            error,
            H5pImportError::UnsupportedFeatures(features)
                if features.len() == 1
                    && features[0].location == "content.params.behaviour.enableRetry"
                    && features[0].feature == "retry-behavior"
        ));
    }

    #[test]
    fn malformed_multiple_choice_is_rejected_before_it_reaches_the_model() {
        let mut duplicate_choice = request();
        duplicate_choice.choices[1].id = "amide".to_string();
        assert_eq!(
            H5pImporter.import(duplicate_choice),
            Err(H5pImportError::DuplicateChoiceId("amide".to_string()))
        );
    }

    #[test]
    fn package_import_rejects_blank_source_fields() {
        let mut blank_remote_reference = request();
        blank_remote_reference
            .package_import
            .remote_package_reference = " \t".to_string();
        assert_eq!(
            H5pImporter.import(blank_remote_reference),
            Err(H5pImportError::EmptyRemotePackageReference)
        );

        for content_type in ["", " \n"] {
            let mut blank_content_type = request();
            blank_content_type.package_import.content_type = content_type.to_string();
            assert_eq!(
                H5pImporter.import(blank_content_type),
                Err(H5pImportError::EmptyContentType)
            );
        }
    }

    #[test]
    fn malformed_package_checksum_is_rejected_with_a_recovery_action() {
        let mut malformed = request();
        malformed.package_import.package_sha256 = "not-a-sha256".to_string();
        let error = H5pImporter
            .import(malformed)
            .expect_err("checksum is a source-integrity boundary");
        assert_eq!(error, H5pImportError::InvalidPackageSha256);
        assert!(error.to_string().contains("verify the downloaded package"));
    }
}
