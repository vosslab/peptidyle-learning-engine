//! Browser-safe bounded course-student membership pages.

use serde::Serialize;

use super::CourseGroupMemberView;

/// Bounded authorized course-student membership page for access controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseStudentMembershipsPage {
    pub students: Vec<CourseGroupMemberView>,
    pub next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::teaching_operations::{
        TeachingDisplayLabel, TeachingMembershipRole, TeachingMembershipStatus,
    };

    #[test]
    fn page_is_safe_and_exact() {
        let page = CourseStudentMembershipsPage {
            students: vec![CourseGroupMemberView {
                reference: "M-1".parse().expect("membership reference"),
                display: TeachingDisplayLabel::try_from("Student Example".to_owned())
                    .expect("safe display"),
                role: TeachingMembershipRole::Student,
                status: TeachingMembershipStatus::Active,
            }],
            next_cursor: None,
        };
        let json = serde_json::to_value(page).expect("safe membership page JSON");
        assert_eq!(
            json,
            serde_json::json!({"students":[{"reference":"M-1","display":"Student Example","role":"student","status":"active"}],"nextCursor":null})
        );
        assert!(!json.to_string().contains("email"));
        assert!(!json.to_string().contains("uuid"));
    }
}
