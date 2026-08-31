//! Safe Assignment Policy Source for server-resolved teaching-policy fields.

use serde::Serialize;

use crate::CourseMembershipReference;

use super::TeachingDisplayLabel;

/// A safe source for one resolved policy field. Direct accommodations are
/// represented by their Student Course Membership, never by a shared cohort.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AssignmentPolicySource {
    Base {
        label: TeachingDisplayLabel,
    },
    Accommodation {
        membership: CourseMembershipReference,
        label: TeachingDisplayLabel,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accommodation_assignment_policy_source_contains_only_the_direct_membership() {
        let value = serde_json::to_value(AssignmentPolicySource::Accommodation {
            membership: "M-4".parse().expect("membership reference"),
            label: TeachingDisplayLabel::try_from("Extended due time".to_owned()).expect("label"),
        })
        .expect("wire");
        assert_eq!(value["kind"], "accommodation");
        assert_eq!(value["membership"], "M-4");
    }
}
