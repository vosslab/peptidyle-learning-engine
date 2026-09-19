use super::super::*;

#[test]
fn submission_observations_preserve_issued_blank_and_credit_buckets() {
    let mut statistics = QuestionRevisionStatistics::empty();
    statistics
        .record(QuestionStatisticsObservation::blank())
        .expect("blank records");
    statistics
        .record(QuestionStatisticsObservation::correct())
        .expect("correct records");
    statistics
        .record(QuestionStatisticsObservation::answered(60_000_000).expect("partial credit"))
        .expect("partial records");
    statistics
        .record(QuestionStatisticsObservation::incorrect())
        .expect("incorrect records");

    assert_eq!(statistics.issued_count(), 4);
    assert_eq!(statistics.blank_count(), 1);
    assert_eq!(statistics.answered_count(), 3);
    assert_eq!(statistics.correct_count(), 1);
    assert_eq!(statistics.partial_count(), 1);
    assert_eq!(statistics.incorrect_count(), 1);
    assert_eq!(statistics.credit_sum_e8(), 160_000_000);
    assert_eq!(statistics.credit_sum_sq_e8(), 136_000_000);
}

#[test]
fn pool_and_member_counters_increment_once() {
    let mut pool = QuestionPoolStatistics::empty();
    let mut member = QuestionPoolMemberStatistics::empty();
    pool.record_issue().expect("pool issue");
    member.record_selection().expect("member selection");
    assert_eq!(pool.issued_count(), 1);
    assert_eq!(member.selected_count(), 1);
}
