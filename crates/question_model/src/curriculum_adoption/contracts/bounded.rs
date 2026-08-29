//! Allocation-bounded sequence decoding for curriculum-adoption browser contracts.

use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, de};

use super::CurriculumPinReplacement;
use crate::{
    MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES, QuestionId,
};

pub(super) fn deserialize_bounded_vec<'de, D, T, const MAX: usize>(
    deserializer: D,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct BoundedVecVisitor<T, const MAX: usize>(PhantomData<T>);

    impl<'de, T, const MAX: usize> de::Visitor<'de> for BoundedVecVisitor<T, MAX>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "a sequence with at most {MAX} entries")
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(MAX));
            while values.len() < MAX {
                let Some(value) = sequence.next_element()? else {
                    return Ok(values);
                };
                values.push(value);
            }
            if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::invalid_length(MAX.saturating_add(1), &self));
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(BoundedVecVisitor::<T, MAX>(PhantomData))
}

pub(super) fn deserialize_replacement_questions<'de, D>(
    deserializer: D,
) -> Result<Vec<QuestionId>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_vec::<D, _, MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP>(deserializer)
}

pub(super) fn deserialize_pin_replacements<'de, D>(
    deserializer: D,
) -> Result<Vec<CurriculumPinReplacement>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_vec::<D, _, MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES>(deserializer)
}

pub(super) fn deserialize_course_instance_corrections<'de, D>(
    deserializer: D,
) -> Result<Vec<super::course_instance::CourseInstanceScheduleCorrection>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_vec::<D, _, MAX_ASSIGNMENT_ORDERED_ENTRIES>(deserializer)
}

pub(super) fn deserialize_course_instance_provenance<'de, D>(
    deserializer: D,
) -> Result<Vec<super::course_instance::BlueprintAssignmentProvenance>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_vec::<D, _, MAX_ASSIGNMENT_ORDERED_ENTRIES>(deserializer)
}
