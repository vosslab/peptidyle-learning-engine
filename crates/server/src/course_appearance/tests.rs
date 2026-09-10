use super::*;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use learning_data_access::{
    ClaimedCourseBannerUpload, PrepareCourseBannerPromotion, PreparedCourseBannerPromotion,
    PreparedCourseBannerRemoval, StageCourseBannerUpload,
};
use objects::{ObjectStoreError, SignedUrl, memory::MemoryObjectStore};

#[derive(Clone, Default)]
struct CompletionStore {
    completed: Arc<Mutex<Vec<question_model::ObjectId>>>,
    repairs: Arc<Mutex<usize>>,
    deletion_completes: Arc<Mutex<usize>>,
    stage_finalizes: Arc<Mutex<usize>>,
    calls: Arc<Mutex<Vec<&'static str>>>,
    fail_completion: Option<usize>,
    fail_stage_finalize: bool,
}

#[async_trait]
impl CourseBannerStore for CompletionStore {
    async fn stage_course_banner_upload(
        &self,
        _: SessionTokenHash,
        _: StageCourseBannerUpload,
    ) -> Result<learning_data_access::StagedCourseBannerUpload, StoreError> {
        Err(StoreError::Unavailable("unused".to_string()))
    }
    async fn finalize_course_banner_upload_stage(
        &self,
        _: SessionTokenHash,
        _: CourseId,
        _: CourseBannerUploadReference,
    ) -> Result<(), StoreError> {
        *self.stage_finalizes.lock().unwrap() += 1;
        if self.fail_stage_finalize {
            Err(StoreError::Unavailable(
                "injected stage finalize".to_string(),
            ))
        } else {
            Ok(())
        }
    }
    async fn prepare_course_banner_promotion(
        &self,
        _: SessionTokenHash,
        _: PrepareCourseBannerPromotion,
    ) -> Result<PreparedCourseBannerPromotion, StoreError> {
        Err(StoreError::Unavailable("unused".to_string()))
    }
    async fn complete_prepared_course_banner_object(
        &self,
        _: SessionTokenHash,
        _: CourseId,
        _: CourseBannerReference,
        object: question_model::ObjectId,
    ) -> Result<(), StoreError> {
        let mut completed = self.completed.lock().unwrap();
        completed.push(object);
        if self.fail_completion.is_some_and(|at| completed.len() == at) {
            Err(StoreError::Unavailable(
                "injected completion failure".to_string(),
            ))
        } else {
            Ok(())
        }
    }
    async fn require_course_banner_object_repair(
        &self,
        _: SessionTokenHash,
        _: CourseId,
        _: Option<CourseBannerReference>,
        _: question_model::ObjectId,
    ) -> Result<(), StoreError> {
        *self.repairs.lock().unwrap() += 1;
        Ok(())
    }
    async fn complete_course_banner_object_deletion(
        &self,
        _: SessionTokenHash,
        _: learning_data_access::CourseBannerDeleteWork,
    ) -> Result<(), StoreError> {
        *self.deletion_completes.lock().unwrap() += 1;
        Ok(())
    }
    async fn prepare_course_banner_object_deletion(
        &self,
        _: SessionTokenHash,
        _: Uuid,
    ) -> Result<learning_data_access::CourseBannerDeleteWork, StoreError> {
        self.calls.lock().unwrap().push("prepare-delete");
        Ok(learning_data_access::CourseBannerDeleteWork {
            work_id: Uuid::from_u128(1),
        })
    }
    async fn require_course_banner_deletion_repair(
        &self,
        _: SessionTokenHash,
        _: learning_data_access::CourseBannerDeleteWork,
    ) -> Result<(), StoreError> {
        self.calls.lock().unwrap().push("require-repair");
        *self.repairs.lock().unwrap() += 1;
        Ok(())
    }
    async fn record_course_banner_cleanup_check(
        &self,
        _: SessionTokenHash,
        _: learning_data_access::CourseBannerDeleteWork,
        _: bool,
        _: Option<Sha256Checksum>,
    ) -> Result<(), StoreError> {
        self.calls.lock().unwrap().push("record-check");
        Ok(())
    }
    async fn finalize_course_banner_promotion(
        &self,
        _: SessionTokenHash,
        _: CourseId,
        _: CourseBannerUploadReference,
        _: CourseBannerReference,
    ) -> Result<FinalizedCourseBannerPromotion, StoreError> {
        Err(StoreError::Unavailable("unused".to_string()))
    }
    async fn read_current_course_banner(
        &self,
        _: SessionTokenHash,
        _: CourseId,
    ) -> Result<Option<question_model::CourseBanner>, StoreError> {
        Err(StoreError::Unavailable("unused".to_string()))
    }
    async fn resolve_current_course_banner(
        &self,
        _: SessionTokenHash,
        _: CourseBannerReference,
    ) -> Result<CourseId, StoreError> {
        Err(StoreError::Unavailable("unused".to_string()))
    }
    async fn read_staged_course_banner_upload(
        &self,
        _: SessionTokenHash,
        _: CourseId,
        _: CourseBannerUploadReference,
    ) -> Result<ClaimedCourseBannerUpload, StoreError> {
        Err(StoreError::Unavailable("unused".to_string()))
    }
    async fn prepare_course_banner_removal(
        &self,
        _: SessionTokenHash,
        _: CourseId,
    ) -> Result<PreparedCourseBannerRemoval, StoreError> {
        Err(StoreError::Unavailable("unused".to_string()))
    }
}

#[derive(Clone, Default)]
struct FaultObjectStore {
    inner: MemoryObjectStore,
    fail_put: Option<usize>,
    fail_delete: bool,
    puts: Arc<Mutex<usize>>,
    deletes: Arc<Mutex<usize>>,
}

type PreparedFixtureObjects = Vec<(ObjectAddress, Vec<u8>, CourseBannerObjectMetadata)>;

#[async_trait]
impl ObjectStore for FaultObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        let put_number = {
            let mut puts = self.puts.lock().unwrap();
            *puts += 1;
            *puts
        };
        if self.fail_put == Some(put_number) {
            return Err(ObjectStoreError::Unavailable(
                "injected put failure".to_string(),
            ));
        }
        self.inner.put(request).await
    }
    async fn get(
        &self,
        address: &ObjectAddress,
    ) -> Result<objects::StoredObject, ObjectStoreError> {
        self.inner.get(address).await
    }
    async fn delete(&self, address: &ObjectAddress) -> Result<(), ObjectStoreError> {
        *self.deletes.lock().unwrap() += 1;
        if self.fail_delete {
            return Err(ObjectStoreError::Unavailable("injected delete".to_string()));
        }
        self.inner.delete(address).await
    }
    async fn signed_url(
        &self,
        address: &ObjectAddress,
        timestamp: Timestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.inner.signed_url(address, timestamp).await
    }
}

fn prepared_fixture() -> (CourseId, CourseBannerReference, PreparedFixtureObjects) {
    let course = CourseId::from_uuid(Uuid::from_u128(71));
    let banner = CourseBannerReference::from_uuid(Uuid::from_u128(72));
    let addresses = [
        ObjectAddress::CourseBannerSource { course, banner },
        ObjectAddress::CourseBannerRendition {
            course,
            banner,
            rendition: CourseBannerRendition::Hero,
        },
        ObjectAddress::CourseBannerRendition {
            course,
            banner,
            rendition: CourseBannerRendition::Card,
        },
    ];
    let values = addresses
        .into_iter()
        .enumerate()
        .map(|(index, address)| {
            let bytes = format!("banner-{index}").into_bytes();
            let metadata = banner_metadata(
                &address,
                &bytes,
                if index == 0 {
                    "image/png".to_string()
                } else {
                    "image/webp".to_string()
                },
            );
            (address, bytes, metadata)
        })
        .collect();
    (course, banner, values)
}

#[tokio::test]
async fn prepared_banner_writes_complete_only_after_each_exact_put() {
    let (course, banner, values) = prepared_fixture();
    let store = CompletionStore::default();
    let objects = FaultObjectStore::default();
    let prepared: Vec<_> = values
        .iter()
        .map(|(a, b, m)| (a, b.clone(), m.clone(), Uuid::from_u128(101)))
        .collect();
    assert!(
        write_prepared_objects(
            &store,
            &objects,
            SessionTokenHash::compute(b"test-session-7"),
            course,
            banner,
            &prepared
        )
        .await
        .is_ok()
    );
    assert_eq!(store.completed.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn prepared_banner_put_failure_never_completes_later_objects() {
    for failure in 1..=3 {
        let (course, banner, values) = prepared_fixture();
        let store = CompletionStore::default();
        let objects = FaultObjectStore {
            fail_put: Some(failure),
            ..Default::default()
        };
        let prepared: Vec<_> = values
            .iter()
            .map(|(a, b, m)| (a, b.clone(), m.clone(), Uuid::from_u128(102)))
            .collect();
        assert!(
            write_prepared_objects(
                &store,
                &objects,
                SessionTokenHash::compute(b"test-session-8"),
                course,
                banner,
                &prepared
            )
            .await
            .is_err()
        );
        assert_eq!(store.completed.lock().unwrap().len(), failure - 1);
    }
}

#[tokio::test]
async fn prepared_banner_completion_failure_stops_before_visibility_can_finalize() {
    let (course, banner, values) = prepared_fixture();
    let store = CompletionStore {
        fail_completion: Some(2),
        ..Default::default()
    };
    let objects = FaultObjectStore::default();
    let prepared: Vec<_> = values
        .iter()
        .map(|(a, b, m)| (a, b.clone(), m.clone(), Uuid::from_u128(103)))
        .collect();
    assert!(
        write_prepared_objects(
            &store,
            &objects,
            SessionTokenHash::compute(b"test-session-9"),
            course,
            banner,
            &prepared
        )
        .await
        .is_err()
    );
    assert_eq!(store.completed.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn staged_put_failure_repairs_without_finalizing() {
    let course = CourseId::from_uuid(Uuid::from_u128(81));
    let upload = CourseBannerUploadReference::from_uuid(Uuid::from_u128(82));
    let address = ObjectAddress::CourseBannerUpload { course, upload };
    let bytes = b"stage".to_vec();
    let metadata = banner_metadata(&address, &bytes, "image/png".to_string());
    let store = CompletionStore::default();
    let objects = FaultObjectStore {
        fail_put: Some(1),
        ..Default::default()
    };
    assert!(
        finalize_staged_upload(
            &store,
            &objects,
            SessionTokenHash::compute(b"stage"),
            course,
            upload,
            Uuid::from_u128(104),
            &address,
            bytes,
            "image/png".to_string(),
            &metadata
        )
        .await
        .is_err()
    );
    assert_eq!(*store.stage_finalizes.lock().unwrap(), 0);
    assert_eq!(*store.repairs.lock().unwrap(), 1);
}

#[tokio::test]
async fn staged_finalize_failure_deletes_and_repairs() {
    let course = CourseId::from_uuid(Uuid::from_u128(83));
    let upload = CourseBannerUploadReference::from_uuid(Uuid::from_u128(84));
    let address = ObjectAddress::CourseBannerUpload { course, upload };
    let bytes = b"stage".to_vec();
    let metadata = banner_metadata(&address, &bytes, "image/png".to_string());
    let store = CompletionStore {
        fail_stage_finalize: true,
        ..Default::default()
    };
    let objects = FaultObjectStore::default();
    assert!(
        finalize_staged_upload(
            &store,
            &objects,
            SessionTokenHash::compute(b"stage2"),
            course,
            upload,
            Uuid::from_u128(105),
            &address,
            bytes,
            "image/png".to_string(),
            &metadata
        )
        .await
        .is_err()
    );
    assert_eq!(*store.stage_finalizes.lock().unwrap(), 1);
    assert_eq!(*store.repairs.lock().unwrap(), 0);
    assert_eq!(*objects.deletes.lock().unwrap(), 1);
}

#[tokio::test]
async fn cleanup_confirms_only_a_successful_delete_and_repairs_a_failed_delete() {
    let course = CourseId::from_uuid(Uuid::from_u128(91));
    let banner = CourseBannerReference::from_uuid(Uuid::from_u128(92));
    let address = ObjectAddress::CourseBannerSource { course, banner };
    let seed = MemoryObjectStore::default();
    seed.put(PutObject {
        address: address.clone(),
        bytes: b"x".to_vec(),
        media_type: "image/png".to_string(),
        created_at: now(),
    })
    .await
    .unwrap();
    let objects = FaultObjectStore {
        inner: seed,
        ..Default::default()
    };
    let store = CompletionStore::default();
    cleanup_address_with(
        &store,
        &objects,
        SessionTokenHash::compute(b"cleanup"),
        course,
        Some(banner),
        &address,
        Uuid::from_u128(93),
    )
    .await;
    assert_eq!(*store.deletion_completes.lock().unwrap(), 1);
    assert_eq!(*store.repairs.lock().unwrap(), 0);
    let failed = FaultObjectStore {
        fail_delete: true,
        ..Default::default()
    };
    cleanup_address_with(
        &store,
        &failed,
        SessionTokenHash::compute(b"cleanup2"),
        course,
        Some(banner),
        &address,
        Uuid::from_u128(94),
    )
    .await;
    assert_eq!(*store.repairs.lock().unwrap(), 1);
    assert_eq!(
        *store.calls.lock().unwrap(),
        vec![
            "prepare-delete",
            "prepare-delete",
            "require-repair",
            "record-check"
        ]
    );
}

#[test]
fn storage_unavailability_is_not_concealed_as_membership_state() {
    let response = store_error_response(StoreError::Unavailable("database offline".to_string()));
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
}

#[test]
fn content_type_guard_accepts_json_parameters_only() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        "application/json; charset=utf-8".parse().unwrap(),
    );
    assert!(has_content_type(&headers, "application/json"));
    headers.insert("content-type", "text/plain".parse().unwrap());
    assert!(!has_content_type(&headers, "application/json"));
}
