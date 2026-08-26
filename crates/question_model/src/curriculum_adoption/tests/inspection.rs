use super::*;

fn source() -> ObservedAlphaSource {
    ObservedAlphaSource {
        reference: AlphaCourseReference::new(4).expect("Alpha reference"),
        revision: "3".parse().expect("Alpha revision"),
    }
}

fn term() -> CourseTerm {
    CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term")
}

fn witness() -> CourseScheduleWitness {
    CourseScheduleWitness::new(
        CourseReference::new(7).expect("course reference"),
        CourseScheduleRevision::new(4).expect("schedule revision"),
        vec![ObservedAssignmentRevision {
            assignment: AssignmentReference::new(9).expect("assignment reference"),
            revision: "2".parse().expect("assignment revision"),
        }],
    )
    .expect("course witness")
}

fn destination_witness() -> CourseScheduleWitness {
    CourseScheduleWitness::new(
        CourseReference::new(12).expect("course reference"),
        CourseScheduleRevision::new(6).expect("schedule revision"),
        [11_u64, 12, 13]
            .into_iter()
            .map(|reference| ObservedAssignmentRevision {
                assignment: AssignmentReference::new(reference).expect("assignment reference"),
                revision: "2".parse().expect("assignment revision"),
            })
            .collect(),
    )
    .expect("destination witness")
}

fn rollover_source(assignment: ObservedAssignmentRevision) -> CurriculumAssignmentImportSourceView {
    let source_witness = witness();
    CurriculumAssignmentImportSourceView::Rollover {
        source: RolloverAssignmentSourceView::new(&source_witness, assignment)
            .expect("assignment belongs to rollover witness"),
    }
}

fn reusable_import() -> CurriculumImportView {
    CurriculumImportView {
        assignment: AssignmentReference::new(11).expect("assignment reference"),
        source: CurriculumAssignmentImportSourceView::Reusable {
            definition: alpha_assignment_source(source(), 2, 3),
        },
        revision: "5".parse().expect("import revision"),
        reusable_meaning_matches_baseline: true,
    }
}

fn inspection(
    origin: CurriculumCourseImportOriginView,
    assignments: Vec<CurriculumImportView>,
) -> CurriculumCourseImportView {
    CurriculumCourseImportView::new(destination_witness(), origin, term(), assignments)
        .expect("bounded inspection")
}

#[test]
fn course_import_origins_are_closed_answer_free_variants() {
    let rollover = RolloverCourseImportOriginView {
        source_schedule: witness(),
    };
    for origin in [
        CurriculumCourseImportOriginView::Ordinary,
        CurriculumCourseImportOriginView::Alpha { source: source() },
        CurriculumCourseImportOriginView::Rollover { source: rollover },
    ] {
        let wire = serde_json::to_value(inspection(origin, vec![reusable_import()]))
            .expect("inspection serializes");
        assert!(
            ["tenant", "actor", "uuid", "answer", "question", "private"]
                .iter()
                .all(|forbidden| !wire.to_string().contains(forbidden))
        );
        assert_eq!(wire["witness"]["course"], "C-12");
        assert!(wire.get("course").is_none() && wire.get("scheduleRevision").is_none());
        assert!(serde_json::from_value::<CurriculumCourseImportView>(wire).is_ok());
    }
}

#[test]
fn rollover_assignment_serializes_only_assignment_local_evidence() {
    let source_witness = witness();
    let rollover = RolloverAssignmentSourceView::new(
        &source_witness,
        source_witness.assignment_revisions()[0],
    )
    .expect("bound rollover assignment");
    let wire =
        serde_json::to_value(CurriculumAssignmentImportSourceView::Rollover { source: rollover })
            .expect("source serializes");
    assert!(wire["source"].get("sourceSchedule").is_none());
    assert!(serde_json::from_value::<CurriculumAssignmentImportSourceView>(wire).is_ok());
}

#[test]
fn inspection_requires_nonempty_unique_assignments_and_rejects_unknown_fields() {
    let valid = serde_json::to_value(inspection(
        CurriculumCourseImportOriginView::Ordinary,
        vec![reusable_import()],
    ))
    .expect("inspection serializes");
    let mut with_unknown = valid.clone();
    with_unknown["actor"] = serde_json::json!("U-7");
    assert!(serde_json::from_value::<CurriculumCourseImportView>(with_unknown).is_err());

    let mut source_with_unknown =
        serde_json::to_value(rollover_source(witness().assignment_revisions()[0]))
            .expect("source serializes");
    source_with_unknown["source"]["sourceSchedule"] =
        serde_json::to_value(witness()).expect("witness serializes");
    assert!(
        serde_json::from_value::<CurriculumAssignmentImportSourceView>(source_with_unknown)
            .is_err()
    );

    let empty = CurriculumCourseImportView::new(
        destination_witness(),
        CurriculumCourseImportOriginView::Ordinary,
        term(),
        vec![],
    );
    assert_eq!(
        empty,
        Err(CurriculumCourseImportViewError::EmptyAssignments)
    );

    let duplicate = CurriculumCourseImportView::new(
        destination_witness(),
        CurriculumCourseImportOriginView::Ordinary,
        term(),
        vec![reusable_import(), reusable_import()],
    );
    assert_eq!(
        duplicate,
        Err(CurriculumCourseImportViewError::DuplicateAssignment)
    );

    let oversized = CurriculumCourseImportView::new(
        destination_witness(),
        CurriculumCourseImportOriginView::Ordinary,
        term(),
        vec![reusable_import(); MAX_ASSIGNMENT_ORDERED_ENTRIES + 1],
    );
    assert_eq!(
        oversized,
        Err(CurriculumCourseImportViewError::TooManyAssignments)
    );
}

#[test]
fn inspection_witness_covers_current_teaching_while_imports_remain_a_subset() {
    let view = inspection(
        CurriculumCourseImportOriginView::Ordinary,
        vec![reusable_import()],
    );
    let unimported = AssignmentReference::new(13).expect("assignment reference");
    assert!(
        view.witness
            .assignment_revisions()
            .iter()
            .any(|assignment| assignment.assignment == unimported)
            && view
                .assignments()
                .iter()
                .all(|import| import.assignment != unimported)
    );

    let incomplete_witness = CourseScheduleWitness::new(
        CourseReference::new(12).expect("course reference"),
        CourseScheduleRevision::new(6).expect("schedule revision"),
        vec![ObservedAssignmentRevision {
            assignment: AssignmentReference::new(13).expect("assignment reference"),
            revision: "2".parse().expect("assignment revision"),
        }],
    )
    .expect("destination witness");
    let absent = CurriculumCourseImportView::new(
        incomplete_witness,
        CurriculumCourseImportOriginView::Ordinary,
        term(),
        vec![reusable_import()],
    );
    assert_eq!(
        absent,
        Err(CurriculumCourseImportViewError::ImportAbsentFromWitness)
    );
}

#[test]
fn inspection_binds_rollover_assignments_to_the_course_origin_witness() {
    let source_assignment = witness().assignment_revisions()[0];
    let rollover_origin = CurriculumCourseImportOriginView::Rollover {
        source: RolloverCourseImportOriginView {
            source_schedule: witness(),
        },
    };
    let rollover_import = CurriculumImportView {
        assignment: AssignmentReference::new(12).expect("assignment reference"),
        source: rollover_source(source_assignment),
        revision: "5".parse().expect("import revision"),
        reusable_meaning_matches_baseline: true,
    };
    let valid = inspection(
        rollover_origin.clone(),
        vec![rollover_import.clone(), reusable_import()],
    );
    let wire = serde_json::to_value(valid).expect("inspection serializes");
    assert_eq!(
        wire["origin"]["source"]["sourceSchedule"],
        serde_json::to_value(witness()).expect("witness serializes")
    );
    assert!(
        wire["assignments"][0]["source"]["source"]
            .get("sourceSchedule")
            .is_none()
    );

    let mut cross_origin_wire = wire.clone();
    cross_origin_wire["origin"] = serde_json::to_value(CurriculumCourseImportOriginView::Ordinary)
        .expect("ordinary origin serializes");
    assert!(serde_json::from_value::<CurriculumCourseImportView>(cross_origin_wire).is_err());

    let mut absent_wire = wire;
    absent_wire["assignments"][0]["source"]["source"]["assignment"] =
        serde_json::to_value(ObservedAssignmentRevision {
            assignment: AssignmentReference::new(10).expect("assignment reference"),
            revision: "2".parse().expect("assignment revision"),
        })
        .expect("source assignment serializes");
    assert!(serde_json::from_value::<CurriculumCourseImportView>(absent_wire).is_err());

    let cross_origin = CurriculumCourseImportView::new(
        destination_witness(),
        CurriculumCourseImportOriginView::Ordinary,
        term(),
        vec![rollover_import.clone()],
    );
    assert_eq!(
        cross_origin,
        Err(CurriculumCourseImportViewError::RolloverSourceWithoutCourseOrigin)
    );

    let missing_source = CurriculumImportView {
        source: rollover_source(source_assignment),
        ..rollover_import
    };
    let other_witness = CourseScheduleWitness::new(
        CourseReference::new(8).expect("course reference"),
        CourseScheduleRevision::new(4).expect("schedule revision"),
        vec![ObservedAssignmentRevision {
            assignment: AssignmentReference::new(10).expect("assignment reference"),
            revision: "2".parse().expect("assignment revision"),
        }],
    )
    .expect("course witness");
    let absent = CurriculumCourseImportView::new(
        destination_witness(),
        CurriculumCourseImportOriginView::Rollover {
            source: RolloverCourseImportOriginView {
                source_schedule: other_witness,
            },
        },
        term(),
        vec![missing_source],
    );
    assert_eq!(
        absent,
        Err(CurriculumCourseImportViewError::RolloverSourceAbsentFromOrigin)
    );
}
