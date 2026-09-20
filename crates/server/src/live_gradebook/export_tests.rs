//! Stable file interoperability and spreadsheet-injection protection contracts.

use learning_data_access::{
    CourseGradebook, CourseGradebookStudentWork, LiveAssessmentAttemptScore,
};
use question_model::AssessmentAttemptCompletion::{Completed, InProgress};

use super::{Format, encode};

fn row(roster_id: &str) -> CourseGradebookStudentWork {
    CourseGradebookStudentWork {
        roster_id: roster_id.to_owned(),
        roster_name: "Synthetic Student".to_owned(),
        assessment_id: "A7K3M2QAS".parse().unwrap(),
        assessment_title: "Protein structure".to_owned(),
        assessment_attempt_completion: None,
        expired_submitting: false,
        score: None,
    }
}

fn gradebook(student_work: Vec<CourseGradebookStudentWork>) -> CourseGradebook {
    CourseGradebook {
        course_instance_id: "CI7K3M2QAZ".parse().unwrap(),
        student_work,
    }
}

fn records(gradebook: &CourseGradebook, format: Format) -> Vec<Vec<String>> {
    let bytes = encode(gradebook, format).unwrap();
    assert!(bytes.starts_with(b"\"roster_id\""));
    assert!(bytes.ends_with(b"\r\n"));
    csv::ReaderBuilder::new()
        .delimiter(format.delimiter())
        .has_headers(false)
        .from_reader(bytes.as_slice())
        .records()
        .map(|record| record.unwrap().iter().map(str::to_owned).collect())
        .collect()
}

#[test]
fn point_exports_preserve_states_points_order_and_empty_courses() {
    let mut submitted = row("01");
    submitted.assessment_id = "AABCDEFG8".parse().unwrap();
    submitted.assessment_attempt_completion = Some(Completed);
    submitted.score = Some(LiveAssessmentAttemptScore {
        points_earned: 0.0,
        points_possible: 3.125,
    });
    let mut open = row("02");
    open.assessment_attempt_completion = Some(InProgress);
    let mut expired = row("03");
    expired.assessment_attempt_completion = Some(InProgress);
    expired.expired_submitting = true;
    let mut pending = row("04");
    pending.assessment_attempt_completion = Some(Completed);
    let mut bonus = row("05");
    bonus.assessment_attempt_completion = Some(Completed);
    bonus.score = Some(LiveAssessmentAttemptScore {
        points_earned: 1.2345678901234567,
        points_possible: -0.0,
    });
    let mut extra = row("06");
    extra.assessment_attempt_completion = Some(Completed);
    extra.score = Some(LiveAssessmentAttemptScore {
        points_earned: 5.0,
        points_possible: 2.0,
    });
    let projection = gradebook(vec![
        extra,
        submitted,
        pending,
        bonus,
        expired,
        open,
        row("01"),
    ]);
    for format in [Format::Csv, Format::Tsv] {
        let output = records(&projection, format);
        assert_eq!(
            output[0],
            [
                "roster_id",
                "roster_name",
                "assessment_id",
                "assessment_title",
                "status",
                "points_earned",
                "points_possible"
            ]
        );
        assert_eq!(output.len(), 8);
        assert_eq!(&output[1][2], "A7K3M2QAS");
        assert_eq!(&output[2][2], "AABCDEFG8");
        let states_and_points: Vec<_> = output[1..].iter().map(|row| &row[4..]).collect();
        assert_eq!(
            states_and_points,
            [
                ["not_started", "", ""],
                ["submitted", "0", "3.125"],
                ["in_progress", "", ""],
                ["expired_submitting", "", ""],
                ["submitted", "", ""],
                ["submitted", "1.2345678901234567", "0"],
                ["submitted", "5", "2"],
            ]
        );
        assert_eq!(records(&gradebook(vec![]), format), vec![output[0].clone()]);
    }
}

#[test]
fn exports_quote_cells_and_protect_formulas_without_row_injection() {
    let dangerous = [
        "=1+1",
        "+1",
        "-1",
        "@SUM(A1)",
        "\tlabel",
        "\rlabel",
        "\nlabel",
        "\0label",
        "  =1",
        "\u{0001} +1",
        "\u{2003}-1",
        " \u{007f}@x",
    ];
    for format in [Format::Csv, Format::Tsv] {
        for label in dangerous {
            let mut work = row("-001");
            work.roster_name = label.to_owned();
            work.assessment_title = label.to_owned();
            let output = records(&gradebook(vec![work]), format);
            assert_eq!(output.len(), 2);
            assert_eq!(output[1].len(), 7);
            assert_eq!(output[1][0], "'-001");
            assert_eq!(output[1][1], format!("'{label}"));
            assert_eq!(output[1][3], format!("'{label}"));
        }
        let mut work = row("0001");
        let label = "Normal, \"quoted\"\ttab\r\nnext line \u{00e9}";
        work.roster_name = label.to_owned();
        work.assessment_title = label.to_owned();
        let projection = gradebook(vec![work]);
        let output = records(&projection, format);
        assert_eq!(output.len(), 2);
        assert_eq!(output[1][0], "0001");
        assert_eq!(output[1][1], label);
        assert_eq!(output[1][3], label);
        let bytes = String::from_utf8(encode(&projection, format).unwrap()).unwrap();
        assert!(bytes.contains("\"\"quoted\"\""));
        assert!(bytes.contains(&format!(
            "\"{}\"{}\"",
            "0001",
            char::from(format.delimiter())
        )));
    }
}

#[test]
fn exports_fail_closed_for_unsubmitted_or_invalid_scores() {
    for completion in [None, Some(InProgress)] {
        let mut work = row("01");
        work.assessment_attempt_completion = completion;
        work.score = Some(LiveAssessmentAttemptScore {
            points_earned: 0.0,
            points_possible: 1.0,
        });
        assert!(encode(&gradebook(vec![work]), Format::Csv).is_err());
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
        for (earned, possible) in [(invalid, 1.0), (1.0, invalid)] {
            let mut work = row("01");
            work.assessment_attempt_completion = Some(Completed);
            work.score = Some(LiveAssessmentAttemptScore {
                points_earned: earned,
                points_possible: possible,
            });
            assert!(encode(&gradebook(vec![work]), Format::Tsv).is_err());
        }
    }
}
