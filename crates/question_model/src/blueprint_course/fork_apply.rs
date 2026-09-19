//! Explicit selective adoption from trusted current source and fork content.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BlueprintAssessmentContent, BlueprintAssessmentId, BlueprintCourseContent,
    BlueprintCourseModuleContent, BlueprintCourseValidationError, BlueprintModuleId,
};

/// Independent source-content choices and optional complete destination layout.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplySelection {
    /// Module labels to copy exactly from the current source.
    pub source_module_labels: Vec<BlueprintForkApplyModuleLabelCopy>,
    /// Complete Assessments to copy, including defaults, Question Revision pins, and Pool ID plus Edit Number.
    pub source_assessments: Vec<BlueprintForkApplyAssessmentCopy>,
    /// Complete authored membership, parentage, and order; omission preserves fork layout.
    pub layout: Option<Vec<BlueprintForkApplyModuleLayout>>,
}

/// One module in an explicitly supplied complete destination layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplyModuleLayout {
    /// Explicit target-local destination or fresh copy from source.
    pub module: BlueprintForkApplyModuleDestination,
    /// Explicit Assessment destinations in authored order.
    pub assessments: Vec<BlueprintForkApplyAssessmentDestination>,
}

/// Copies a source label onto an existing target module, or a new module when null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplyModuleLabelCopy {
    /// Module in the readable current source.
    pub source_module_id: BlueprintModuleId,
    /// Existing target-local module; null explicitly requests a new copy.
    #[serde(deserialize_with = "deserialize_explicit_target")]
    pub target_module_id: Option<BlueprintModuleId>,
}

/// Copies complete source Assessment content without copying its local identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplyAssessmentCopy {
    /// Assessment in the readable current source.
    pub source_assessment_id: BlueprintAssessmentId,
    /// Existing target-local Assessment; null explicitly requests a new copy.
    #[serde(deserialize_with = "deserialize_explicit_target")]
    pub target_assessment_id: Option<BlueprintAssessmentId>,
}

// ASVS 1.5.2: a missing target is not the explicit null new-copy choice.
fn deserialize_explicit_target<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// A module's explicit destination in the complete target layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BlueprintForkApplyModuleDestination {
    /// Retains a target-local module ID.
    Existing {
        /// Module already in the target.
        target_module_id: BlueprintModuleId,
    },
    /// Allocates a fresh target-local ID for a selected source label.
    NewFromSource {
        /// Source module whose label is selected for copying.
        source_module_id: BlueprintModuleId,
    },
}

/// An Assessment's explicit destination in the complete target layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BlueprintForkApplyAssessmentDestination {
    /// Retains a target-local Assessment ID.
    Existing {
        /// Assessment already in the target.
        target_assessment_id: BlueprintAssessmentId,
    },
    /// Allocates a fresh target-local ID for selected complete source content.
    NewFromSource {
        /// Source Assessment selected for copying.
        source_assessment_id: BlueprintAssessmentId,
    },
}

/// Selection or resulting-tree validation failure, before any persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintForkApplyError {
    /// A supplied tree, selection, or layout repeats a module identity.
    DuplicateModuleId,
    /// A supplied tree, selection, or layout repeats an Assessment identity.
    DuplicateAssessmentId,
    /// A selected source label or destination module does not exist on the required side.
    UnknownModuleId,
    /// A selected source Assessment or destination Assessment does not exist on the required side.
    UnknownAssessmentId,
    /// A destination source-only module needs an explicit source-label choice.
    MissingSourceModuleLabelSelection,
    /// A destination source-only Assessment needs an explicit complete source-content choice.
    MissingSourceAssessmentSelection,
    /// A selected source label or Assessment is absent from the destination layout.
    SelectionOutsideLayout,
    /// Trusted allocations must have exactly the new-copy keys and fresh unique IDs.
    InvalidAllocation,
    /// Existing domain constructors refuse the complete destination tree.
    InvalidContent(BlueprintCourseValidationError),
}

impl std::fmt::Display for BlueprintForkApplyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateModuleId => formatter.write_str("Blueprint apply repeats a module ID"),
            Self::DuplicateAssessmentId => {
                formatter.write_str("Blueprint apply repeats an Assessment ID")
            }
            Self::UnknownModuleId => {
                formatter.write_str("Blueprint apply has an unknown module ID")
            }
            Self::UnknownAssessmentId => {
                formatter.write_str("Blueprint apply has an unknown Assessment ID")
            }
            Self::MissingSourceModuleLabelSelection => formatter.write_str(
                "Blueprint apply requires an explicit source label for a source-only module",
            ),
            Self::MissingSourceAssessmentSelection => formatter.write_str(
                "Blueprint apply requires explicit source content for a source-only Assessment",
            ),
            Self::SelectionOutsideLayout => formatter.write_str(
                "Blueprint apply selects source content absent from its destination layout",
            ),
            Self::InvalidContent(error) => error.fmt(formatter),
            Self::InvalidAllocation => {
                formatter.write_str("Blueprint apply has invalid fresh ID allocations")
            }
        }
    }
}

impl std::error::Error for BlueprintForkApplyError {}

struct ContentIndex<'a> {
    modules: BTreeMap<BlueprintModuleId, &'a BlueprintCourseModuleContent>,
    assessments: BTreeMap<BlueprintAssessmentId, &'a BlueprintAssessmentContent>,
}

impl<'a> ContentIndex<'a> {
    fn new(content: &'a BlueprintCourseContent) -> Result<Self, BlueprintForkApplyError> {
        let mut index = Self {
            modules: BTreeMap::new(),
            assessments: BTreeMap::new(),
        };
        for module in content.modules() {
            if index
                .modules
                .insert(module.blueprint_module_id(), module)
                .is_some()
            {
                return Err(BlueprintForkApplyError::DuplicateModuleId);
            }
            for assessment in module.assessments() {
                if index
                    .assessments
                    .insert(assessment.blueprint_assessment_id(), assessment)
                    .is_some()
                {
                    return Err(BlueprintForkApplyError::DuplicateAssessmentId);
                }
            }
        }
        Ok(index)
    }
}

/// Builds one coherent owned fork tree without mutating either borrowed input.
///
/// Unselected labels and complete Assessment content come from the target. Selected
/// content comes directly from the source without resolving publication heads.
/// Existing destinations retain target-local IDs; new-copy destinations use only
/// exact fresh allocations supplied by trusted server code. Only an explicit
/// complete layout adds/removes/moves nodes; source positions never infer target
/// positions. Every new node requires its matching explicit null-target copy.
/// Name metadata, source
/// read authorization, fork ownership, expected-current CAS, and persistence are
/// caller responsibilities, not part of this pure function.
///
/// # Errors
///
/// Rejects duplicate or unknown identities, incomplete source-only selections,
/// selections absent from the destination, and layouts refused by the existing
/// nonempty bounded module/course constructors.
pub fn apply_blueprint_fork(
    source: &BlueprintCourseContent,
    target: &BlueprintCourseContent,
    selection: &BlueprintForkApplySelection,
    new_modules: &BTreeMap<BlueprintModuleId, BlueprintModuleId>,
    new_assessments: &BTreeMap<BlueprintAssessmentId, BlueprintAssessmentId>,
) -> Result<BlueprintCourseContent, BlueprintForkApplyError> {
    // An explicit fork layout remains an ordinary authored replacement tree.
    // Trusted C419 persistence may represent an empty Course, but fork apply
    // cannot use that widening to erase a destination Blueprint.
    if selection.layout.as_ref().is_some_and(Vec::is_empty) {
        return Err(BlueprintForkApplyError::InvalidContent(
            BlueprintCourseValidationError::InvalidModuleCount,
        ));
    }
    let source_index = ContentIndex::new(source)?;
    let target_index = ContentIndex::new(target)?;
    let mut module_sources = BTreeSet::new();
    let mut assessment_sources = BTreeSet::new();
    let mut module_copies = BTreeMap::new();
    let mut assessment_copies = BTreeMap::new();
    let mut new_module_sources = BTreeSet::new();
    let mut new_assessment_sources = BTreeSet::new();
    // ASVS 2.2.1, 2.2.3: source choices and existing destinations are unique and known.
    for copy in &selection.source_module_labels {
        if !module_sources.insert(copy.source_module_id) {
            return Err(BlueprintForkApplyError::DuplicateModuleId);
        }
        if !source_index.modules.contains_key(&copy.source_module_id) {
            return Err(BlueprintForkApplyError::UnknownModuleId);
        }
        if let Some(target_module_id) = copy.target_module_id {
            if !target_index.modules.contains_key(&target_module_id) {
                return Err(BlueprintForkApplyError::UnknownModuleId);
            }
            if module_copies
                .insert(target_module_id, copy.source_module_id)
                .is_some()
            {
                return Err(BlueprintForkApplyError::DuplicateModuleId);
            }
        } else {
            new_module_sources.insert(copy.source_module_id);
        }
    }
    for copy in &selection.source_assessments {
        if !assessment_sources.insert(copy.source_assessment_id) {
            return Err(BlueprintForkApplyError::DuplicateAssessmentId);
        }
        if !source_index
            .assessments
            .contains_key(&copy.source_assessment_id)
        {
            return Err(BlueprintForkApplyError::UnknownAssessmentId);
        }
        if let Some(target_assessment_id) = copy.target_assessment_id {
            if !target_index.assessments.contains_key(&target_assessment_id) {
                return Err(BlueprintForkApplyError::UnknownAssessmentId);
            }
            if assessment_copies
                .insert(target_assessment_id, copy.source_assessment_id)
                .is_some()
            {
                return Err(BlueprintForkApplyError::DuplicateAssessmentId);
            }
        } else {
            new_assessment_sources.insert(copy.source_assessment_id);
        }
    }
    if selection.layout.is_none()
        && (!new_module_sources.is_empty() || !new_assessment_sources.is_empty())
    {
        return Err(BlueprintForkApplyError::SelectionOutsideLayout);
    }
    // ASVS 2.2.2, 2.2.3: only trusted, exact, fresh allocation sets are accepted.
    if new_modules.keys().copied().collect::<BTreeSet<_>>() != new_module_sources
        || new_assessments.keys().copied().collect::<BTreeSet<_>>() != new_assessment_sources
        || new_modules.values().copied().collect::<BTreeSet<_>>().len() != new_modules.len()
        || new_assessments
            .values()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != new_assessments.len()
        || new_modules.values().any(|id| {
            source_index.modules.contains_key(id) || target_index.modules.contains_key(id)
        })
        || new_assessments.values().any(|id| {
            source_index.assessments.contains_key(id) || target_index.assessments.contains_key(id)
        })
    {
        return Err(BlueprintForkApplyError::InvalidAllocation);
    }
    let preserved_layout;
    let layout = if let Some(layout) = &selection.layout {
        layout
    } else {
        preserved_layout = target
            .modules()
            .iter()
            .map(|module| BlueprintForkApplyModuleLayout {
                module: BlueprintForkApplyModuleDestination::Existing {
                    target_module_id: module.blueprint_module_id(),
                },
                assessments: module
                    .assessments()
                    .iter()
                    .map(
                        |assessment| BlueprintForkApplyAssessmentDestination::Existing {
                            target_assessment_id: assessment.blueprint_assessment_id(),
                        },
                    )
                    .collect(),
            })
            .collect::<Vec<_>>();
        &preserved_layout
    };
    let mut destination_modules = BTreeSet::new();
    let mut destination_assessments = BTreeSet::new();
    let mut modules = Vec::new();
    for row in layout {
        let (destination_module_id, label_module) = match row.module {
            BlueprintForkApplyModuleDestination::Existing { target_module_id } => {
                let original = target_index
                    .modules
                    .get(&target_module_id)
                    .ok_or(BlueprintForkApplyError::UnknownModuleId)?;
                let module = module_copies
                    .get(&target_module_id)
                    .map_or(*original, |source| source_index.modules[source]);
                (target_module_id, module)
            }
            BlueprintForkApplyModuleDestination::NewFromSource { source_module_id } => {
                let destination_module_id = *new_modules
                    .get(&source_module_id)
                    .ok_or(BlueprintForkApplyError::MissingSourceModuleLabelSelection)?;
                (
                    destination_module_id,
                    source_index.modules[&source_module_id],
                )
            }
        };
        if !destination_modules.insert(destination_module_id) {
            return Err(BlueprintForkApplyError::DuplicateModuleId);
        }
        let mut assessments = Vec::new();
        for destination in &row.assessments {
            let (destination_assessment_id, content) = match *destination {
                BlueprintForkApplyAssessmentDestination::Existing {
                    target_assessment_id,
                } => {
                    let original = target_index
                        .assessments
                        .get(&target_assessment_id)
                        .ok_or(BlueprintForkApplyError::UnknownAssessmentId)?;
                    let content = assessment_copies
                        .get(&target_assessment_id)
                        .map_or(*original, |source| source_index.assessments[source]);
                    (target_assessment_id, content)
                }
                BlueprintForkApplyAssessmentDestination::NewFromSource {
                    source_assessment_id,
                } => {
                    let destination_assessment_id = *new_assessments
                        .get(&source_assessment_id)
                        .ok_or(BlueprintForkApplyError::MissingSourceAssessmentSelection)?;
                    (
                        destination_assessment_id,
                        source_index.assessments[&source_assessment_id],
                    )
                }
            };
            if !destination_assessments.insert(destination_assessment_id) {
                return Err(BlueprintForkApplyError::DuplicateAssessmentId);
            }
            assessments.push(
                BlueprintAssessmentContent::new(
                    destination_assessment_id,
                    content.assessment_type(),
                    crate::AssessmentTitle::try_new(content.title().to_owned()).map_err(|_| {
                        BlueprintForkApplyError::InvalidContent(
                            BlueprintCourseValidationError::InvalidContentTitle,
                        )
                    })?,
                    content.instructions().clone(),
                    content.entries().to_vec(),
                    content.defaults().clone(),
                )
                .map_err(BlueprintForkApplyError::InvalidContent)?,
            );
        }
        modules.push(
            BlueprintCourseModuleContent::new(
                destination_module_id,
                label_module.label().to_owned(),
                assessments,
            )
            .map_err(BlueprintForkApplyError::InvalidContent)?,
        );
    }
    if module_copies
        .keys()
        .chain(new_modules.values())
        .any(|id| !destination_modules.contains(id))
        || assessment_copies
            .keys()
            .chain(new_assessments.values())
            .any(|id| !destination_assessments.contains(id))
    {
        return Err(BlueprintForkApplyError::SelectionOutsideLayout);
    }
    BlueprintCourseContent::new(modules).map_err(BlueprintForkApplyError::InvalidContent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_empty_layout_cannot_erase_fork_content() {
        let content = BlueprintCourseContent::new(Vec::new()).expect("trusted empty content");
        let error = apply_blueprint_fork(
            &content,
            &content,
            &BlueprintForkApplySelection {
                layout: Some(Vec::new()),
                ..BlueprintForkApplySelection::default()
            },
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect_err("explicit fork layout remains nonempty");

        assert_eq!(
            error,
            BlueprintForkApplyError::InvalidContent(
                BlueprintCourseValidationError::InvalidModuleCount
            )
        );
    }
}
