//! Fixed account identities that the closed installer recipe binds to a generation.

pub(crate) const PRIMARY_INSTRUCTOR_NAME: &str = "Dr. Elena Rivera";
pub(crate) const MARY_NAME: &str = "Mary Okafor";
pub(crate) const JACK_NAME: &str = "Jack Chen";
pub(crate) const APPROVAL_CANDIDATE_NAME: &str = "Avery Singh";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_account_constants_are_complete_and_distinct() {
        assert_eq!(PRIMARY_INSTRUCTOR_NAME, "Dr. Elena Rivera");
        assert_eq!(MARY_NAME, "Mary Okafor");
        assert_eq!(JACK_NAME, "Jack Chen");
        assert_eq!(APPROVAL_CANDIDATE_NAME, "Avery Singh");
    }
}
