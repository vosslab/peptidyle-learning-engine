//! Course Instance to Blueprint Course publication contract.

use async_trait::async_trait;
use question_model::{
    CourseInstanceReference, CreateBlueprintCourseReceipt, CreateBlueprintFromCourseInstanceInput,
    RequestChecksum,
};

use crate::{SessionTokenHash, StoreError};

/// Atomically copies current reusable Course structure into a new Private Blueprint.
#[async_trait]
pub trait CourseBlueprintPublicationStore: Send + Sync {
    async fn create_blueprint_from_course_instance(
        &self,
        session: SessionTokenHash,
        source_course: CourseInstanceReference,
        request_checksum: RequestChecksum,
        input: CreateBlueprintFromCourseInstanceInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError>;
}
