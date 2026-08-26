use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationPreviewRequest,
    AssignmentTeachingSettingsFailureReason, CourseTerm, CurriculumAdoptionTitle,
    CurriculumPinReplacements, LocalTimeOfDay, ObservedAlphaSource, RelativeAssignmentSchedule,
    RelativeScheduleMoment,
};

use super::*;

#[tokio::test]
async fn preview_returns_dst_correction_and_corrected_alpha_instantiates() {
    let fixture = Fixture::new().await;
    let mut definition = fixture.alpha_input.clone();
    definition.modules[0].definitions[0].schedule = RelativeAssignmentSchedule {
        available_at: Some(RelativeScheduleMoment {
            day_offset: 0,
            local_time: LocalTimeOfDay::parse("02:30:00.000").expect("gap time"),
        }),
        due_at: None,
        closes_at: None,
    };
    let gap = fixture
        .store
        .replace_alpha_course(
            fixture.context,
            fixture.session,
            ReplaceAlphaCourseCommand {
                reference: Some(fixture.alpha.reference),
                expected_revision: Some(fixture.alpha.revision),
                definition: definition.clone(),
            },
        )
        .await
        .expect("gap source revision");
    let target =
        CourseTerm::from_parts("2026-03-08", "2026-11-01", "America/Chicago").expect("DST term");
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.session,
            AlphaInstantiationPreviewRequest {
                source: ObservedAlphaSource {
                    reference: gap.reference,
                    revision: gap.revision,
                },
                title: CurriculumAdoptionTitle::parse("DST correction").expect("title"),
                target_term: target.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    assert!(preview.corrections.iter().any(|correction| {
        correction.correction.reason
            == AssignmentTeachingSettingsFailureReason::NonexistentLocalTime
    }));
    definition.modules[0].definitions[0].schedule.available_at = Some(RelativeScheduleMoment {
        day_offset: 0,
        local_time: LocalTimeOfDay::parse("03:30:00.000").expect("corrected time"),
    });
    let corrected = fixture
        .store
        .replace_alpha_course(
            fixture.context,
            fixture.session,
            ReplaceAlphaCourseCommand {
                reference: Some(gap.reference),
                expected_revision: Some(gap.revision),
                definition,
            },
        )
        .await
        .expect("corrected source revision");
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.session,
            AlphaInstantiationPreviewRequest {
                source: ObservedAlphaSource {
                    reference: corrected.reference,
                    revision: corrected.revision,
                },
                title: CurriculumAdoptionTitle::parse("DST corrected").expect("title"),
                target_term: target,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("corrected preview");
    assert!(preview.corrections.is_empty());
    fixture
        .store
        .apply_alpha_instantiation(
            fixture.context,
            fixture.session,
            AlphaInstantiationCommand::from_preview(&preview, key("dst-corrected"))
                .expect("corrected command"),
        )
        .await
        .expect("corrected instantiation");
}
