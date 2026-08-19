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
use question_model::answer::SelectionCardinality;
use question_model::capability::{BackendCapabilities, Capability};
use question_model::definition::{GradingDefinition, QuestionMetadata, QuestionSource};
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use sha2::{Digest, Sha256};

/// Version of the persisted H5P import record schema.
///
/// This is deliberately independent of the repository CalVer release version:
/// it changes only when the durable import record needs a migration.
pub const IMPORT_SCHEMA_VERSION: u16 = 2;
/// The only native H5P content type currently converted by this small,
/// explicit importer.
pub const MULTI_CHOICE_CONTENT_TYPE: &str = "H5P.MultiChoice";

/// Provenance and archival identity for an H5P package.
///
/// The object-store package is authoritative for re-import. The remote
/// reference is provenance only and is intentionally kept out of the
/// browser-safe [`QuestionSource`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct H5pSourceReference {
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
    /// provenance only: the publication worker must be able to retrieve these
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
/// remote URL in [`H5pSourceReference`].
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
pub struct H5pSourceIdentity(String);

impl H5pSourceIdentity {
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
    /// Restricted Markdown shown to the learner.
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
    /// Remote source retained for deterministic re-import.
    pub source: H5pSourceReference,
    /// Browser-safe catalog metadata.
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
    pub attempt_policy: AttemptPolicy,
    /// Timing policy requested by the source.
    ///
    /// Native H5P timing is not claimed by this adapter, so any timed request
    /// is rejected explicitly rather than silently downgraded.
    pub timing_policy: TimingPolicy,
}

/// A converted, unpublished H5P sandbox draft.
///
/// This intentionally is not [`QuestionDefinition`]: an H5P import has no
/// published problem or version identity.  The catalog's publish transition
/// owns that identity assignment, preserving the invariant that sandbox
/// imports cannot masquerade as immutable published content.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedH5pQuestion {
    /// Version of the internal record shape used for this import.
    pub import_schema_version: u16,
    /// Authoritative source record retained for re-import and provenance.
    pub source_reference: H5pSourceReference,
    /// The source family safe for downstream adapter dispatch.
    pub source: QuestionSource,
    /// Prompt ready for the browser-safe renderer.
    pub prompt: Vec<ContentBlock>,
    /// Response shape, without a correct answer.
    pub response: ResponseDefinition,
    /// Practice retry behavior.
    pub attempt_policy: AttemptPolicy,
    /// The only timing policy currently supported by the adapter.
    pub timing_policy: TimingPolicy,
    /// H5P imports are static until a server-owned generator is selected.
    pub randomization: RandomizationDefinition,
    /// Always `Ungraded` for native H5P practice.
    pub grading: GradingDefinition,
    /// Browser-safe title, taxonomy, and licensing metadata.
    pub metadata: QuestionMetadata,
    /// Deterministic identity of the exact source package and reference.
    pub source_identity: H5pSourceIdentity,
}

/// H5P adapter boundary.  It advertises only capabilities that this adapter
/// can actually enforce.
#[derive(Debug, Default, Clone, Copy)]
pub struct H5pImporter;

impl H5pImporter {
    /// Native H5P capabilities.
    ///
    /// `serverGrading` is absent by design.  `perQuestionTiming`, export,
    /// offline operation, hints, partial credit, and parameter generation are
    /// likewise not promised by this initial supported conversion.
    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::from_iter([Capability::ClientRendering])
    }

    /// Converts one supported H5P activity into a key-free internal question.
    ///
    /// # Errors
    ///
    /// Fails closed for unsupported content types, malformed source identity
    /// inputs, unsupported timing, and invalid multiple-choice structure.
    pub fn import(&self, request: H5pImportRequest) -> Result<ImportedH5pQuestion, H5pImportError> {
        let source = canonicalize_source(request.source)?;
        if source.content_type != MULTI_CHOICE_CONTENT_TYPE {
            return Err(H5pImportError::UnsupportedContentType(source.content_type));
        }
        if !request.unsupported_features.is_empty() {
            return Err(H5pImportError::UnsupportedFeatures(
                request.unsupported_features,
            ));
        }
        if !matches!(request.timing_policy, TimingPolicy::Untimed) {
            return Err(H5pImportError::UnsupportedTiming);
        }
        if request.prompt_markdown.trim().is_empty() {
            return Err(H5pImportError::EmptyPrompt);
        }

        let choices = normalize_choices(request.choices)?;
        let source_identity = source_identity(&source);
        Ok(ImportedH5pQuestion {
            import_schema_version: IMPORT_SCHEMA_VERSION,
            source_reference: source.clone(),
            source: QuestionSource::H5p {
                content_type: source.content_type,
            },
            prompt: vec![ContentBlock::Text {
                markdown: request.prompt_markdown,
            }],
            response: ResponseDefinition::MultipleChoice {
                choices,
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: request.attempt_policy,
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::Ungraded,
            metadata: request.metadata,
            source_identity,
        })
    }

    /// Retrieves an archived package, verifies its bytes, then converts the
    /// parser result that was derived from those verified bytes.
    ///
    /// The parser closure is intentionally supplied by the hostile-input
    /// worker: this small adapter does not parse H5P ZIP files itself. The
    /// closure receives only the verified package bytes and the canonical
    /// source reference. Its returned request must retain that source exactly,
    /// preventing a parser from accidentally reattaching a different archive.
    ///
    /// # Errors
    ///
    /// Refuses missing objects, resolver identity mismatches, checksum
    /// mismatches, and parser requests that do not retain the verified source.
    pub async fn reimport_from_archive<R, P>(
        &self,
        resolver: &R,
        source: H5pSourceReference,
        parse_verified_package: P,
    ) -> Result<ImportedH5pQuestion, H5pImportError>
    where
        R: H5pArchiveResolver + ?Sized,
        P: FnOnce(&[u8], H5pSourceReference) -> Result<H5pImportRequest, H5pImportError>,
    {
        let source = canonicalize_source(source)?;
        let archived = resolver
            .get_archived_h5p(source.stored_package_object)
            .await
            .map_err(H5pImportError::Archive)?;
        if archived.object != source.stored_package_object {
            return Err(H5pImportError::ArchiveObjectMismatch {
                expected: source.stored_package_object,
                actual: archived.object,
            });
        }

        let actual_sha256 = sha256_hex(&archived.bytes);
        if actual_sha256 != source.package_sha256 {
            return Err(H5pImportError::ArchiveChecksumMismatch {
                expected: source.package_sha256,
                actual: actual_sha256,
            });
        }

        let request = parse_verified_package(&archived.bytes, source.clone())?;
        let parsed_source = canonicalize_source(request.source.clone())?;
        if parsed_source != source {
            return Err(H5pImportError::ReimportSourceMismatch);
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
        /// Immutable object identity retained by the source record.
        expected: ObjectId,
        /// Object identity returned by the resolver.
        actual: ObjectId,
    },
    /// Retrieved bytes differ from the canonical package checksum retained by
    /// the source record.
    ArchiveChecksumMismatch {
        /// Canonical SHA-256 retained at accepted import.
        expected: String,
        /// Canonical SHA-256 recomputed from the retrieved archive bytes.
        actual: String,
    },
    /// A re-import parser attempted to substitute a different source record.
    ReimportSourceMismatch,
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
    /// A choice has no visible learner-facing text.
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
            Self::ArchiveChecksumMismatch { expected, actual } => write!(
                formatter,
                "archived H5P checksum mismatch: expected `{expected}`, got `{actual}`; preserve the original package and investigate storage integrity"
            ),
            Self::ReimportSourceMismatch => write!(
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

fn canonicalize_source(
    mut source: H5pSourceReference,
) -> Result<H5pSourceReference, H5pImportError> {
    if source.content_type.trim().is_empty() {
        return Err(H5pImportError::EmptyContentType);
    }
    if source.remote_package_reference.trim().is_empty() {
        return Err(H5pImportError::EmptyRemotePackageReference);
    }
    if source.package_sha256.len() != 64
        || !source
            .package_sha256
            .as_bytes()
            .iter()
            .all(u8::is_ascii_hexdigit)
    {
        return Err(H5pImportError::InvalidPackageSha256);
    }
    source.package_sha256.make_ascii_lowercase();
    Ok(source)
}

fn source_identity(source: &H5pSourceReference) -> H5pSourceIdentity {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-h5p-source-identity-v2");
    for value in [
        &source.content_type,
        &source.remote_package_reference,
        &source.package_sha256,
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    let value = source.stored_package_object.to_string();
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut hexadecimal = String::with_capacity(digest.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        hexadecimal.push(HEX[usize::from(byte >> 4)] as char);
        hexadecimal.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    H5pSourceIdentity(hexadecimal)
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

fn normalize_choices(choices: Vec<H5pChoice>) -> Result<Vec<ChoiceOption>, H5pImportError> {
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
            Ok(ChoiceOption {
                id: ChoiceId::new(choice.id),
                body: vec![ContentBlock::Text {
                    markdown: choice.markdown,
                }],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::capability::Capability;
    use question_model::definition::{GradingDefinition, QuestionSource};
    use question_model::envelope::ContentBlock;
    use question_model::run_policy::TimingPolicy;
    use question_model::taxonomy::License;
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
            source: H5pSourceReference {
                content_type: MULTI_CHOICE_CONTENT_TYPE.to_string(),
                remote_package_reference: "https://h5p.example.edu/content/peptide-bonds-1.h5p"
                    .to_string(),
                package_sha256: "f760739feba129fa2861ab5badca6327d5235eef5ef6b67b149b20aa7b512077"
                    .to_string(),
                stored_package_object: ObjectId::from_uuid(Uuid::from_u128(0x81)),
            },
            metadata: QuestionMetadata {
                title: "Peptide bonds practice".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
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
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
        }
    }

    #[test]
    fn capability_declaration_is_honest_about_ungraded_h5p() {
        let capabilities = H5pImporter.capabilities();
        assert!(capabilities.supports(Capability::ClientRendering));
        assert!(!capabilities.supports(Capability::ServerGrading));
        assert_eq!(
            capabilities.missing_from(Capability::ALL),
            vec![
                Capability::AlgorithmicGeneration,
                Capability::ServerGrading,
                Capability::PartialCredit,
                Capability::Hints,
                Capability::PerQuestionTiming,
                Capability::PrintExport,
                Capability::OfflinePreview,
            ]
        );
    }

    #[test]
    fn supported_h5p_import_becomes_a_key_free_ungraded_internal_question() {
        let imported = H5pImporter
            .import(request())
            .expect("supported H5P imports");
        assert_eq!(
            imported.source,
            QuestionSource::H5p {
                content_type: MULTI_CHOICE_CONTENT_TYPE.to_string(),
            }
        );
        assert_eq!(imported.grading, GradingDefinition::Ungraded);
        assert_eq!(imported.import_schema_version, IMPORT_SCHEMA_VERSION);
        assert_eq!(imported.source_reference, request().source);
        assert_eq!(imported.timing_policy, TimingPolicy::Untimed);
        assert!(matches!(
            imported.prompt.as_slice(),
            [ContentBlock::Text { markdown }] if markdown == "Which linkage joins amino acids?"
        ));
        assert!(matches!(
            imported.response,
            ResponseDefinition::MultipleChoice {
                selection: SelectionCardinality::ExactlyOne,
                ..
            }
        ));
    }

    #[test]
    fn source_identity_is_deterministic_and_binds_exact_package_bytes() {
        let importer = H5pImporter;
        let first = importer.import(request()).expect("first import");
        let repeat = importer.import(request()).expect("repeat import");
        assert_eq!(first.source_identity, repeat.source_identity);
        assert_eq!(first.source_identity.as_str().len(), 64);

        let mut altered_bytes = request();
        altered_bytes.source.package_sha256 =
            "6af75b2ddb2dc6d40d11e1c184b10b2a695bc2ebf84b87d90b8c35f8a76d9dc5".to_string();
        let changed = importer
            .import(altered_bytes)
            .expect("same URL with different package bytes imports distinctly");
        assert_ne!(first.source_identity, changed.source_identity);
    }

    #[test]
    fn accepted_import_requires_and_retains_the_ple_controlled_package_object() {
        let request = request();
        let object = request.source.stored_package_object;
        let imported = H5pImporter
            .import(request)
            .expect("stored H5P package imports");
        assert_eq!(imported.source_reference.stored_package_object, object);
    }

    #[tokio::test]
    async fn reimport_retrieves_and_reverifies_the_archived_package_before_conversion() {
        let source = request().source;
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
                parsed.source = verified_source;
                Ok(parsed)
            })
            .await
            .expect("verified archive can be re-imported");

        assert_eq!(imported.source_reference, source);
        assert_eq!(imported.grading, GradingDefinition::Ungraded);
    }

    #[tokio::test]
    async fn reimport_refuses_a_missing_archival_object() {
        let error = H5pImporter
            .reimport_from_archive(
                &FixtureArchiveResolver { package: None },
                request().source,
                |_bytes, _source| unreachable!("parser runs only after retrieval"),
            )
            .await
            .expect_err("a remote URL is not an archival fallback");
        assert_eq!(error, H5pImportError::Archive(H5pArchiveError::NotFound));
    }

    #[tokio::test]
    async fn reimport_refuses_bytes_that_do_not_match_the_retained_checksum() {
        let source = request().source;
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
            H5pImportError::ArchiveChecksumMismatch { expected, actual }
                if expected == source.package_sha256 && actual == sha256_hex(b"changed package bytes")
        ));
    }

    #[test]
    fn unsupported_features_fail_explicitly_instead_of_being_silently_downgraded() {
        let mut unsupported_type = request();
        unsupported_type.source.content_type = "H5P.DragText".to_string();
        assert_eq!(
            H5pImporter.import(unsupported_type),
            Err(H5pImportError::UnsupportedContentType(
                "H5P.DragText".to_string()
            ))
        );

        let mut timed = request();
        timed.timing_policy = TimingPolicy::PerQuestion {
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
    fn malformed_package_checksum_is_rejected_with_a_recovery_action() {
        let mut malformed = request();
        malformed.source.package_sha256 = "not-a-sha256".to_string();
        let error = H5pImporter
            .import(malformed)
            .expect_err("checksum is a source-integrity boundary");
        assert_eq!(error, H5pImportError::InvalidPackageSha256);
        assert!(error.to_string().contains("verify the downloaded package"));
    }
}
