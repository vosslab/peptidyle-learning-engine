//! Explicit selective adoption from trusted current source and fork content.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BlueprintAssessmentContent, BlueprintAssessmentReference, BlueprintCourseContent,
    BlueprintCourseModuleContent, BlueprintCourseValidationError, BlueprintModuleReference,
};

/// Independent source-content choices and optional complete destination layout.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplySelection {
    /// Module labels to copy exactly from the current source.
    pub source_module_labels: Vec<BlueprintForkApplyModuleLabelCopy>,
    /// Complete Assessments to copy, including defaults and exact Question/Pool pins.
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
    pub source_module_reference: BlueprintModuleReference,
    /// Existing target-local module; null explicitly requests a new copy.
    #[serde(deserialize_with = "deserialize_explicit_target")]
    pub target_module_reference: Option<BlueprintModuleReference>,
}

/// Copies complete source Assessment content without copying its local identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintForkApplyAssessmentCopy {
    /// Assessment in the readable current source.
    pub source_assessment_reference: BlueprintAssessmentReference,
    /// Existing target-local Assessment; null explicitly requests a new copy.
    #[serde(deserialize_with = "deserialize_explicit_target")]
    pub target_assessment_reference: Option<BlueprintAssessmentReference>,
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
        target_module_reference: BlueprintModuleReference,
    },
    /// Allocates a fresh target-local ID for a selected source label.
    NewFromSource {
        /// Source module whose label is selected for copying.
        source_module_reference: BlueprintModuleReference,
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
        target_assessment_reference: BlueprintAssessmentReference,
    },
    /// Allocates a fresh target-local ID for selected complete source content.
    NewFromSource {
        /// Source Assessment selected for copying.
        source_assessment_reference: BlueprintAssessmentReference,
    },
}

/// Selection or resulting-tree validation failure, before any persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintForkApplyError {
    /// A supplied tree, selection, or layout repeats a module identity.
    DuplicateModuleReference,
    /// A supplied tree, selection, or layout repeats an Assessment identity.
    DuplicateAssessmentReference,
    /// A selected source label or destination module does not exist on the required side.
    UnknownModuleReference,
    /// A selected source Assessment or destination Assessment does not exist on the required side.
    UnknownAssessmentReference,
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
            Self::DuplicateModuleReference => {
                formatter.write_str("Blueprint apply repeats a module reference")
            }
            Self::DuplicateAssessmentReference => {
                formatter.write_str("Blueprint apply repeats an Assessment reference")
            }
            Self::UnknownModuleReference => {
                formatter.write_str("Blueprint apply has an unknown module reference")
            }
            Self::UnknownAssessmentReference => {
                formatter.write_str("Blueprint apply has an unknown Assessment reference")
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
    modules: BTreeMap<BlueprintModuleReference, &'a BlueprintCourseModuleContent>,
    assessments: BTreeMap<BlueprintAssessmentReference, &'a BlueprintAssessmentContent>,
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
                .insert(module.blueprint_module_reference(), module)
                .is_some()
            {
                return Err(BlueprintForkApplyError::DuplicateModuleReference);
            }
            for assessment in module.assessments() {
                if index
                    .assessments
                    .insert(assessment.blueprint_assessment_reference(), assessment)
                    .is_some()
                {
                    return Err(BlueprintForkApplyError::DuplicateAssessmentReference);
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
    new_modules: &BTreeMap<BlueprintModuleReference, BlueprintModuleReference>,
    new_assessments: &BTreeMap<BlueprintAssessmentReference, BlueprintAssessmentReference>,
) -> Result<BlueprintCourseContent, BlueprintForkApplyError> {
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
        if !module_sources.insert(copy.source_module_reference) {
            return Err(BlueprintForkApplyError::DuplicateModuleReference);
        }
        if !source_index
            .modules
            .contains_key(&copy.source_module_reference)
        {
            return Err(BlueprintForkApplyError::UnknownModuleReference);
        }
        if let Some(reference) = copy.target_module_reference {
            if !target_index.modules.contains_key(&reference) {
                return Err(BlueprintForkApplyError::UnknownModuleReference);
            }
            if module_copies
                .insert(reference, copy.source_module_reference)
                .is_some()
            {
                return Err(BlueprintForkApplyError::DuplicateModuleReference);
            }
        } else {
            new_module_sources.insert(copy.source_module_reference);
        }
    }
    for copy in &selection.source_assessments {
        if !assessment_sources.insert(copy.source_assessment_reference) {
            return Err(BlueprintForkApplyError::DuplicateAssessmentReference);
        }
        if !source_index
            .assessments
            .contains_key(&copy.source_assessment_reference)
        {
            return Err(BlueprintForkApplyError::UnknownAssessmentReference);
        }
        if let Some(reference) = copy.target_assessment_reference {
            if !target_index.assessments.contains_key(&reference) {
                return Err(BlueprintForkApplyError::UnknownAssessmentReference);
            }
            if assessment_copies
                .insert(reference, copy.source_assessment_reference)
                .is_some()
            {
                return Err(BlueprintForkApplyError::DuplicateAssessmentReference);
            }
        } else {
            new_assessment_sources.insert(copy.source_assessment_reference);
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
                    target_module_reference: module.blueprint_module_reference(),
                },
                assessments: module
                    .assessments()
                    .iter()
                    .map(
                        |assessment| BlueprintForkApplyAssessmentDestination::Existing {
                            target_assessment_reference: assessment
                                .blueprint_assessment_reference(),
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
        let (reference, label_module) = match row.module {
            BlueprintForkApplyModuleDestination::Existing {
                target_module_reference,
            } => {
                let original = target_index
                    .modules
                    .get(&target_module_reference)
                    .ok_or(BlueprintForkApplyError::UnknownModuleReference)?;
                let module = module_copies
                    .get(&target_module_reference)
                    .map_or(*original, |source| source_index.modules[source]);
                (target_module_reference, module)
            }
            BlueprintForkApplyModuleDestination::NewFromSource {
                source_module_reference,
            } => {
                let reference = *new_modules
                    .get(&source_module_reference)
                    .ok_or(BlueprintForkApplyError::MissingSourceModuleLabelSelection)?;
                (reference, source_index.modules[&source_module_reference])
            }
        };
        if !destination_modules.insert(reference) {
            return Err(BlueprintForkApplyError::DuplicateModuleReference);
        }
        let mut assessments = Vec::new();
        for destination in &row.assessments {
            let (reference, content) = match *destination {
                BlueprintForkApplyAssessmentDestination::Existing {
                    target_assessment_reference,
                } => {
                    let original = target_index
                        .assessments
                        .get(&target_assessment_reference)
                        .ok_or(BlueprintForkApplyError::UnknownAssessmentReference)?;
                    let content = assessment_copies
                        .get(&target_assessment_reference)
                        .map_or(*original, |source| source_index.assessments[source]);
                    (target_assessment_reference, content)
                }
                BlueprintForkApplyAssessmentDestination::NewFromSource {
                    source_assessment_reference,
                } => {
                    let reference = *new_assessments
                        .get(&source_assessment_reference)
                        .ok_or(BlueprintForkApplyError::MissingSourceAssessmentSelection)?;
                    (
                        reference,
                        source_index.assessments[&source_assessment_reference],
                    )
                }
            };
            if !destination_assessments.insert(reference) {
                return Err(BlueprintForkApplyError::DuplicateAssessmentReference);
            }
            assessments.push(
                BlueprintAssessmentContent::new(
                    reference,
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
                reference,
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
