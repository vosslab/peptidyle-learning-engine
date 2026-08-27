use super::super::*;

#[test]
fn assignment_revision_requires_one_positive_strong_etag() {
    let accepted = HeaderMap::from_iter([(IF_MATCH, HeaderValue::from_static("\"7\""))]);
    assert_eq!(
        required_assignment_revision(&accepted).expect("strong revision"),
        "7".parse().expect("revision")
    );
    for value in ["7", "W/\"7\"", "\"0\"", "\"-1\"", "\"9223372036854775808\""] {
        let headers =
            HeaderMap::from_iter([(IF_MATCH, HeaderValue::from_str(value).expect("test header"))]);
        assert_eq!(
            required_assignment_revision(&headers),
            Err(AssignmentRevisionHeaderError::Malformed)
        );
    }
    assert_eq!(
        required_assignment_revision(&HeaderMap::new()),
        Err(AssignmentRevisionHeaderError::Missing)
    );
}
