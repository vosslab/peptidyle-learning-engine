//! Current-pair Blueprint comparison using shared Question IDs only.
use super::canonical_exchange::CanonicalBlueprintAssessment;
use crate::{
    BlueprintAssessmentEntryContent, BlueprintAssessmentId, BlueprintCourseContent,
    BlueprintModuleId, QuestionId, QuestionPoolEditNumber, QuestionRevisionTuple,
};
use std::collections::{BTreeMap, BTreeSet};

/// One authored module; its handle is local to its comparison side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintComparisonModule {
    /// Side-local module handle, not matching evidence.
    pub blueprint_module_id: BlueprintModuleId,
    /// Zero-based authored module position.
    pub position: usize,
    /// Authored module label.
    pub label: String,
}

/// Complete reusable Assessment and resolved Question lineage set.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintComparisonAssessment {
    /// Side-local Assessment handle, not matching evidence.
    pub blueprint_assessment_id: BlueprintAssessmentId,
    /// Side-local containing module handle.
    pub blueprint_module_id: BlueprintModuleId,
    /// Zero-based authored position within its module.
    pub position: usize,
    /// Complete answer-free canonical reusable content.
    pub content: CanonicalBlueprintAssessment,
    /// Sorted unique fixed and exact-Pool-member Question IDs, excluding Revisions.
    pub question_ids: Vec<QuestionId>,
}

/// Complete side inventory in authored module and Assessment order.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintComparisonInventory {
    /// Every authored module.
    pub modules: Vec<BlueprintComparisonModule>,
    /// Every Assessment, in module order then within-module order.
    pub assessments: Vec<BlueprintComparisonAssessment>,
}

/// Nonempty Question-ID intersection, allowing Assessment splits and merges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintAssessmentRelationship {
    /// Assessment handle in the left inventory.
    pub left_assessment_id: BlueprintAssessmentId,
    /// Assessment handle in the right inventory.
    pub right_assessment_id: BlueprintAssessmentId,
    /// Sorted unique Question IDs shared by this pair.
    pub shared_question_ids: Vec<QuestionId>,
}

/// Answer-free comparison of two independently selected Blueprint Revisions.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintComparison {
    /// Complete left-side authored inventory.
    pub left: BlueprintComparisonInventory,
    /// Complete right-side authored inventory.
    pub right: BlueprintComparisonInventory,
    /// Every related pair, in left then right authored Assessment order.
    pub relationships: Vec<BlueprintAssessmentRelationship>,
    /// Sorted unique Question IDs present on both sides.
    pub shared_question_ids: Vec<QuestionId>,
    /// Sorted unique Question IDs present only on the left.
    pub left_only_question_ids: Vec<QuestionId>,
    /// Sorted unique Question IDs present only on the right.
    pub right_only_question_ids: Vec<QuestionId>,
}

/// Cannot retain all side-local handles or resolve trusted exact Pool members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintComparisonError {
    /// A module reference repeats within one side.
    DuplicateModuleId,
    /// An Assessment reference repeats within one side.
    DuplicateAssessmentId,
    /// A referenced Pool has no supplied membership.
    MissingPoolMembership,
    /// Pool membership is empty, repeats a lineage, or cannot satisfy selection.
    InvalidPoolMembership,
}
impl std::fmt::Display for BlueprintComparisonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DuplicateModuleId => "Blueprint comparison repeats a module ID",
            Self::DuplicateAssessmentId => "Blueprint comparison repeats an Assessment ID",
            Self::MissingPoolMembership => "Blueprint comparison lacks exact Pool membership",
            Self::InvalidPoolMembership => "Blueprint comparison has invalid Pool membership",
        })
    }
}
impl std::error::Error for BlueprintComparisonError {}

/// Compares two complete trees without requiring an origin baseline.
///
/// Only shared Question IDs establish relationships. Names, internal handles,
/// Revisions and fuzzy similarity never establish a match. Pools contribute all
/// members of exactly the supplied current Pool, not sampled/latest members.
/// Authorized reads, lineage eligibility, names and provenance belong to the
/// caller. Inputs remain unchanged; no Question bodies or answers are read.
///
/// # Errors
///
/// Rejects repeated side-local handles and missing or invalid referenced Pool
/// memberships instead of returning a partial or misleading comparison.
pub fn compare_blueprint_courses(
    left: &BlueprintCourseContent,
    right: &BlueprintCourseContent,
    pool_memberships: &BTreeMap<(QuestionId, QuestionPoolEditNumber), Vec<QuestionRevisionTuple>>,
) -> Result<BlueprintComparison, BlueprintComparisonError> {
    let left = inventory(left, pool_memberships)?;
    let right = inventory(right, pool_memberships)?;
    let left_ids = inventory_question_ids(&left);
    let right_ids = inventory_question_ids(&right);
    let mut relationships = Vec::new();
    for left_assessment in &left.assessments {
        let left_assessment_ids: BTreeSet<_> =
            left_assessment.question_ids.iter().cloned().collect();
        for right_assessment in &right.assessments {
            let shared_question_ids: Vec<_> = right_assessment
                .question_ids
                .iter()
                .filter(|id| left_assessment_ids.contains(*id))
                .cloned()
                .collect();
            if !shared_question_ids.is_empty() {
                relationships.push(BlueprintAssessmentRelationship {
                    left_assessment_id: left_assessment.blueprint_assessment_id,
                    right_assessment_id: right_assessment.blueprint_assessment_id,
                    shared_question_ids,
                });
            }
        }
    }
    Ok(BlueprintComparison {
        shared_question_ids: left_ids.intersection(&right_ids).cloned().collect(),
        left_only_question_ids: left_ids.difference(&right_ids).cloned().collect(),
        right_only_question_ids: right_ids.difference(&left_ids).cloned().collect(),
        left,
        right,
        relationships,
    })
}

fn inventory(
    content: &BlueprintCourseContent,
    pool_memberships: &BTreeMap<(QuestionId, QuestionPoolEditNumber), Vec<QuestionRevisionTuple>>,
) -> Result<BlueprintComparisonInventory, BlueprintComparisonError> {
    let mut inventory = BlueprintComparisonInventory {
        modules: Vec::new(),
        assessments: Vec::new(),
    };
    let mut module_ids = BTreeSet::new();
    let mut assessment_ids = BTreeSet::new();
    for (position, module) in content.modules().iter().enumerate() {
        let module_id = module.blueprint_module_id();
        if !module_ids.insert(module_id) {
            return Err(BlueprintComparisonError::DuplicateModuleId);
        }
        inventory.modules.push(BlueprintComparisonModule {
            blueprint_module_id: module_id,
            position,
            label: module.label().to_owned(),
        });
        for (position, assessment) in module.assessments().iter().enumerate() {
            let assessment_id = assessment.blueprint_assessment_id();
            if !assessment_ids.insert(assessment_id) {
                return Err(BlueprintComparisonError::DuplicateAssessmentId);
            }
            let mut question_ids = BTreeSet::new();
            for entry in assessment.entries() {
                match entry {
                    BlueprintAssessmentEntryContent::Fixed { question_revision, .. } => {
                        question_ids.insert(question_revision.question_id.clone());
                    }
                    BlueprintAssessmentEntryContent::Pool(pool) => {
                        let members = pool_memberships
                            .get(&(
                                pool.question_pool_id().clone(),
                                pool.question_pool_edit_number(),
                            ))
                            .ok_or(BlueprintComparisonError::MissingPoolMembership)?;
                        let member_ids: BTreeSet<_> = members
                            .iter()
                            .map(|member| member.question_id.clone())
                            .collect();
                        if members.is_empty()
                            || member_ids.len() != members.len()
                            || members.len() < pool.selection_count().get() as usize
                        {
                            return Err(BlueprintComparisonError::InvalidPoolMembership);
                        }
                        question_ids.extend(member_ids);
                    }
                }
            }
            inventory.assessments.push(BlueprintComparisonAssessment {
                blueprint_assessment_id: assessment_id,
                blueprint_module_id: module_id,
                position,
                content: CanonicalBlueprintAssessment::from(assessment),
                question_ids: question_ids.into_iter().collect(),
            });
        }
    }
    Ok(inventory)
}

fn inventory_question_ids(inventory: &BlueprintComparisonInventory) -> BTreeSet<QuestionId> {
    inventory
        .assessments
        .iter()
        .flat_map(|assessment| assessment.question_ids.iter().cloned())
        .collect()
}
