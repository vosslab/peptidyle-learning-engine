//! Connected actual-role oracle for canonical Blueprint exchange.

use super::*;

pub(super) async fn assert_actual_role_round_trip(
    store: &PostgresBlueprintCourseStore,
    blueprint_reference: BlueprintCourseReference,
    owner_private: &learning_data_access::StoredBlueprintCourse,
) {
    // The canonical exchange is a stable reusable-content contract. Import
    // creates a distinct owner-owned Private lineage at Revision 1; it does
    // not transfer source lineage identity, visibility, or child identities.
    let exported = store
        .export_blueprint_course(token(), blueprint_reference)
        .await
        .expect("owner exports reusable Blueprint content");
    let imported = store
        .import_blueprint_course(
            token(),
            RequestChecksum::from_bytes([0x29; 32]),
            exported.clone(),
        )
        .await
        .expect("owner imports canonical reusable Blueprint content");
    assert_ne!(
        imported.blueprint_revision.reference, blueprint_reference,
        "canonical import creates a distinct local Blueprint lineage"
    );
    assert_eq!(
        imported.blueprint_revision.revision,
        BlueprintRevision::INITIAL,
        "canonical import starts the new lineage at Revision 1"
    );
    let imported_private = store
        .load_blueprint_course(token(), imported.blueprint_revision.reference)
        .await
        .expect("owner reads imported Private Blueprint");
    assert_eq!(
        store
            .load_blueprint_course(token(), blueprint_reference)
            .await
            .expect("owner reloads unchanged source Blueprint"),
        *owner_private,
        "canonical import does not mutate the source Blueprint lineage"
    );
    assert_eq!(
        imported_private.availability,
        BlueprintAvailability::Private
    );
    assert_eq!(imported_private.short_name, owner_private.short_name);
    assert_eq!(imported_private.long_name, owner_private.long_name);
    assert_eq!(
        imported_private.classification,
        owner_private.classification
    );
    let reexported = store
        .export_blueprint_course(token(), imported.blueprint_revision.reference)
        .await
        .expect("owner exports imported reusable Blueprint content");
    assert_eq!(reexported.metadata(), exported.metadata());
    assert_eq!(reexported.modules().len(), exported.modules().len());
    for (source_module, imported_module) in exported.modules().iter().zip(reexported.modules()) {
        assert_eq!(imported_module.label(), source_module.label());
        assert_eq!(
            imported_module.assessments().len(),
            source_module.assessments().len()
        );
        for (source_assessment, imported_assessment) in source_module
            .assessments()
            .iter()
            .zip(imported_module.assessments())
        {
            assert_eq!(
                imported_assessment.assessment_type(),
                source_assessment.assessment_type()
            );
            assert_eq!(imported_assessment.title(), source_assessment.title());
            assert_eq!(
                imported_assessment.instructions(),
                source_assessment.instructions()
            );
            assert_eq!(imported_assessment.defaults(), source_assessment.defaults());
            assert_eq!(
                imported_assessment.entries().len(),
                source_assessment.entries().len()
            );
            for (source_entry, imported_entry) in source_assessment
                .entries()
                .iter()
                .zip(imported_assessment.entries())
            {
                match (source_entry, imported_entry) {
                    (
                        question_model::CanonicalBlueprintAssessmentEntry::Fixed {
                            published_question: source_question,
                            points_possible: source_points,
                            scoring_rule: source_scoring,
                            question_attempt_limit: source_limit,
                            question_attempt_time_limit: source_time_limit,
                        },
                        question_model::CanonicalBlueprintAssessmentEntry::Fixed {
                            published_question: imported_question,
                            points_possible: imported_points,
                            scoring_rule: imported_scoring,
                            question_attempt_limit: imported_limit,
                            question_attempt_time_limit: imported_time_limit,
                        },
                    ) => {
                        assert_eq!(imported_question, source_question);
                        assert_eq!(imported_points, source_points);
                        assert_eq!(imported_scoring, source_scoring);
                        assert_eq!(imported_limit, source_limit);
                        assert_eq!(imported_time_limit, source_time_limit);
                    }
                    (
                        question_model::CanonicalBlueprintAssessmentEntry::Pool {
                            question_pool_revision: source_pool,
                            selection_count: source_count,
                            points_per_item: source_points,
                            scoring_rule: source_scoring,
                            selection_rule: source_selection,
                            question_attempt_limit: source_limit,
                            question_attempt_time_limit: source_time_limit,
                        },
                        question_model::CanonicalBlueprintAssessmentEntry::Pool {
                            question_pool_revision: imported_pool,
                            selection_count: imported_count,
                            points_per_item: imported_points,
                            scoring_rule: imported_scoring,
                            selection_rule: imported_selection,
                            question_attempt_limit: imported_limit,
                            question_attempt_time_limit: imported_time_limit,
                        },
                    ) => {
                        assert_ne!(
                            imported_pool.question_pool_id, source_pool.question_pool_id,
                            "canonical import creates a fresh local Pool identity"
                        );
                        assert_eq!(
                            question_pool_member_pins(imported_pool).await,
                            question_pool_member_pins(source_pool).await,
                            "canonical import preserves the exact ordered Pool member pins"
                        );
                        assert_eq!(imported_pool.revision_number, source_pool.revision_number);
                        assert_eq!(imported_count, source_count);
                        assert_eq!(imported_points, source_points);
                        assert_eq!(imported_scoring, source_scoring);
                        assert_eq!(imported_selection, source_selection);
                        assert_eq!(imported_limit, source_limit);
                        assert_eq!(imported_time_limit, source_time_limit);
                    }
                    _ => panic!("canonical import changed an ordered entry kind"),
                }
            }
        }
    }
}
