//! Canonical three-way comparison for deliberate Blueprint Course fork updates.
//!
//! This module accepts only the four selectable Blueprint units.  It therefore
//! cannot compare ownership, visibility, Stars, Watches, adoption, Course
//! state, Student state, or any other operational state.  Store and server
//! layers own source authorization, candidate digests, persistence, and
//! selected application; this pure model layer only classifies a supplied
//! immutable base, source, and fork snapshot.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    BlueprintAssessmentReference, BlueprintModuleReference,
    canonical_exchange::CanonicalBlueprintAssessment,
};

/// One module's Assessment ordering, identified only by stable child References.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintForkAssessmentOrder {
    module_reference: BlueprintModuleReference,
    assessment_references: Vec<BlueprintAssessmentReference>,
}

impl BlueprintForkAssessmentOrder {
    /// Builds one authored module order from stable References.
    pub fn new(
        module_reference: BlueprintModuleReference,
        assessment_references: Vec<BlueprintAssessmentReference>,
    ) -> Self {
        Self {
            module_reference,
            assessment_references,
        }
    }

    /// Returns the retained module identity.
    pub fn module_reference(&self) -> BlueprintModuleReference {
        self.module_reference
    }

    /// Returns Assessment identities in authored order.
    pub fn assessment_references(&self) -> &[BlueprintAssessmentReference] {
        &self.assessment_references
    }
}

/// The only values eligible for Blueprint fork synchronization comparison.
///
/// This intentionally omits every operational or identity field.  Each map
/// value is the canonical complete reusable Assessment projection, so a whole
/// Assessment comparison includes its settings, Published Question Revision
/// pins, and published Question Pool content.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintForkSyncSnapshot {
    short_name: String,
    long_name: String,
    assessments: BTreeMap<BlueprintAssessmentReference, CanonicalBlueprintAssessment>,
    assessment_order: Vec<BlueprintForkAssessmentOrder>,
}

impl BlueprintForkSyncSnapshot {
    /// Constructs the constrained snapshot used by the canonical comparator.
    pub fn new(
        short_name: impl Into<String>,
        long_name: impl Into<String>,
        assessments: BTreeMap<BlueprintAssessmentReference, CanonicalBlueprintAssessment>,
        assessment_order: Vec<BlueprintForkAssessmentOrder>,
    ) -> Self {
        Self {
            short_name: short_name.into(),
            long_name: long_name.into(),
            assessments,
            assessment_order,
        }
    }
}

/// One independently selectable Blueprint fork update unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlueprintForkSyncUnit {
    /// Compact reusable Blueprint name.
    ShortName,
    /// Descriptive reusable Blueprint name.
    LongName,
    /// One complete reusable Blueprint Assessment by stable Reference.
    Assessment(BlueprintAssessmentReference),
    /// The complete authored module and Assessment order.
    OrderedAssessmentList,
}

/// The only allowed three-way result for one selectable update unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintForkSyncClassification {
    /// The source changed while the fork still has the immutable base value.
    Safe,
    /// The fork already has the source value.
    AlreadyApplied,
    /// Both source and fork differ from their common immutable base.
    Conflict,
    /// The source did not change this unit from its immutable base.
    NotApplicable,
}

/// Classification attached to its exact selectable unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintForkSyncComparison {
    unit: BlueprintForkSyncUnit,
    classification: BlueprintForkSyncClassification,
}

impl BlueprintForkSyncComparison {
    /// Returns the specific unit that was compared.
    pub fn unit(&self) -> BlueprintForkSyncUnit {
        self.unit
    }

    /// Returns the resulting three-way classification.
    pub fn classification(&self) -> BlueprintForkSyncClassification {
        self.classification
    }
}

/// Classifies one allowed unit from its base, newer source, and current fork values.
///
/// Equality is deliberately the complete canonical value, not a partial field
/// comparison.  A removed Assessment is represented by `None`, so a source
/// deletion remains a normal safe/conflict/already-applied candidate rather
/// than being silently ignored.
pub fn compare_blueprint_fork_sync_unit(
    unit: BlueprintForkSyncUnit,
    base: &BlueprintForkSyncSnapshot,
    source: &BlueprintForkSyncSnapshot,
    fork: &BlueprintForkSyncSnapshot,
) -> BlueprintForkSyncComparison {
    let classification = match unit {
        BlueprintForkSyncUnit::ShortName => classify(
            Some(&base.short_name),
            Some(&source.short_name),
            Some(&fork.short_name),
        ),
        BlueprintForkSyncUnit::LongName => classify(
            Some(&base.long_name),
            Some(&source.long_name),
            Some(&fork.long_name),
        ),
        BlueprintForkSyncUnit::Assessment(reference) => classify(
            base.assessments.get(&reference),
            source.assessments.get(&reference),
            fork.assessments.get(&reference),
        ),
        BlueprintForkSyncUnit::OrderedAssessmentList => classify(
            Some(&base.assessment_order),
            Some(&source.assessment_order),
            Some(&fork.assessment_order),
        ),
    };
    BlueprintForkSyncComparison {
        unit,
        classification,
    }
}

/// Compares every allowed unit exactly once in deterministic stable-Reference order.
pub fn compare_blueprint_fork_sync(
    base: &BlueprintForkSyncSnapshot,
    source: &BlueprintForkSyncSnapshot,
    fork: &BlueprintForkSyncSnapshot,
) -> Vec<BlueprintForkSyncComparison> {
    let assessment_references = base
        .assessments
        .keys()
        .chain(source.assessments.keys())
        .chain(fork.assessments.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut comparisons = vec![
        compare_blueprint_fork_sync_unit(BlueprintForkSyncUnit::ShortName, base, source, fork),
        compare_blueprint_fork_sync_unit(BlueprintForkSyncUnit::LongName, base, source, fork),
    ];
    comparisons.extend(assessment_references.into_iter().map(|reference| {
        compare_blueprint_fork_sync_unit(
            BlueprintForkSyncUnit::Assessment(reference),
            base,
            source,
            fork,
        )
    }));
    comparisons.push(compare_blueprint_fork_sync_unit(
        BlueprintForkSyncUnit::OrderedAssessmentList,
        base,
        source,
        fork,
    ));
    comparisons
}

fn classify<T: PartialEq>(
    base: Option<&T>,
    source: Option<&T>,
    fork: Option<&T>,
) -> BlueprintForkSyncClassification {
    if source == base {
        BlueprintForkSyncClassification::NotApplicable
    } else if fork == base {
        BlueprintForkSyncClassification::Safe
    } else if fork == source {
        BlueprintForkSyncClassification::AlreadyApplied
    } else {
        BlueprintForkSyncClassification::Conflict
    }
}
