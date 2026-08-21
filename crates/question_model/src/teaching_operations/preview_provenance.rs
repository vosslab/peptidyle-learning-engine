//! Safe browser provenance for server-resolved teaching-policy fields.

use serde::Serialize;

use crate::{CourseGroupReference, CourseMembershipReference};

use super::{MAX_COURSE_GROUP_MEMBERS, TeachingDisplayLabel};

/// One human-readable group that contributed to a resolved policy field.
///
/// It intentionally carries only the public `G-` route reference and a safe
/// display label, never an internal group ID or membership roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingPreviewGroupSource {
    pub group: CourseGroupReference,
    pub label: TeachingDisplayLabel,
}

/// Ordered, nonempty, unique, bounded group sources for one resolved field.
///
/// The order is resolver provenance order. It is never sorted or deduplicated
/// silently, so a server mapping must preserve every contributing group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TeachingPreviewGroupSources(Vec<TeachingPreviewGroupSource>);

impl TeachingPreviewGroupSources {
    /// Returns every group source in resolver provenance order.
    pub fn as_slice(&self) -> &[TeachingPreviewGroupSource] {
        &self.0
    }
}

impl TryFrom<Vec<TeachingPreviewGroupSource>> for TeachingPreviewGroupSources {
    type Error = &'static str;

    fn try_from(groups: Vec<TeachingPreviewGroupSource>) -> Result<Self, Self::Error> {
        if groups.is_empty() {
            return Err("preview group sources must not be empty");
        }
        if groups.len() > MAX_COURSE_GROUP_MEMBERS {
            return Err("preview group sources may contain at most 100 groups");
        }
        if groups.iter().enumerate().any(|(index, source)| {
            groups[..index]
                .iter()
                .any(|prior| prior.group == source.group)
        }) {
            return Err("preview group sources must have unique group references");
        }
        Ok(Self(groups))
    }
}

/// Safe human source for one resolved S3 field.
///
/// The group variants map exactly to the ordered domain provenance unions:
/// `GroupScheduleOffsets(Vec<CourseGroupId>)` and
/// `GroupAccommodations(Vec<CourseGroupId>)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TeachingPreviewFieldSource {
    Base {
        label: TeachingDisplayLabel,
    },
    GroupScheduleOffsets {
        groups: TeachingPreviewGroupSources,
    },
    GroupAccommodations {
        groups: TeachingPreviewGroupSources,
    },
    Membership {
        membership: CourseMembershipReference,
        label: TeachingDisplayLabel,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(reference: &str, label: &str) -> TeachingPreviewGroupSource {
        TeachingPreviewGroupSource {
            group: reference.parse().unwrap(),
            label: TeachingDisplayLabel::try_from(label.to_owned()).unwrap(),
        }
    }

    #[test]
    fn group_sources_reject_empty_duplicate_and_oversized_inputs() {
        assert!(TeachingPreviewGroupSources::try_from(Vec::new()).is_err());
        assert!(
            TeachingPreviewGroupSources::try_from(vec![group("G-1", "Lab"), group("G-1", "Lab")])
                .is_err()
        );
        let oversized: Vec<_> = (1..=MAX_COURSE_GROUP_MEMBERS + 1)
            .map(|value| group(&format!("G-{value}"), "Lab"))
            .collect();
        assert!(TeachingPreviewGroupSources::try_from(oversized).is_err());
    }

    #[test]
    fn accommodation_provenance_serializes_every_group_in_order() {
        let groups = vec![group("G-4", "Extra time"), group("G-7", "Testing center")]
            .try_into()
            .unwrap();
        let value =
            serde_json::to_value(TeachingPreviewFieldSource::GroupAccommodations { groups })
                .unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "kind":"groupAccommodations",
                "groups":[
                    {"group":"G-4","label":"Extra time"},
                    {"group":"G-7","label":"Testing center"}
                ]
            })
        );
    }
}
