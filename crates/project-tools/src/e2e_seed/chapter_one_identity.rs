//! Fresh publication identities and non-question fixture convergence keys.

use super::chapter_one::PilotQuestionSpec;
use super::*;
use learning_data_access::PublishedProblemRecord;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const CHAPTER_ONE_STATISTICS_FIXTURE_SLUG: &str = "biochemistry-functional-groups-flat-mc";

pub(super) struct QuestionIds {
    pub(super) workspace: WorkspaceId,
    pub(super) problem: ProblemId,
    pub(super) version: VersionId,
    pub(super) workspace_source: ObjectId,
    pub(super) published_source: ObjectId,
}

impl QuestionIds {
    pub(super) fn generate() -> Self {
        Self {
            workspace: WorkspaceId::generate(),
            problem: ProblemId::generate(),
            version: VersionId::generate(),
            workspace_source: ObjectId::generate(),
            published_source: ObjectId::generate(),
        }
    }

    pub(super) fn from_published(record: &PublishedProblemRecord) -> Self {
        Self {
            workspace: record.question.workspace,
            problem: record.problem,
            version: record.version,
            workspace_source: ObjectId::generate(),
            published_source: ObjectId::generate(),
        }
    }
}

/// The reviewed question and immutable published identity used by the
/// disposable catalog-statistics journey. This is created only while the
/// normal Chapter 1 publisher is handling the tracked manifest, so adjunct
/// activity cannot derive a different problem/version identity from a slug.
pub(super) struct ChapterOneStatisticsFixture {
    pub(super) reference: ProblemVersionRef,
    pub(super) source: &'static [u8],
}

pub(super) fn is_catalog_statistics_fixture(question: &PilotQuestionSpec) -> bool {
    question.slug == CHAPTER_ONE_STATISTICS_FIXTURE_SLUG
}

pub(super) fn pilot_uuid(slug: &str, purpose: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-single-installation-chapter-one-pilot-v1:");
    hasher.update(slug.as_bytes());
    hasher.update(b":");
    hasher.update(purpose.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
