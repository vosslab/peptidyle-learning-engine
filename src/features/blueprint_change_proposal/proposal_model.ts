import type { BlueprintChangeProposalComparisonView } from "../../../generated/api/BlueprintChangeProposalComparisonView";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";

/** Adapter only for pure inventory/layout controls; never loads current lineage evidence. */
export function proposalInventory(
  value: BlueprintChangeProposalComparisonView,
): BlueprintComparisonView {
  return {
    left: { ...value.source, currentRevisionTuple: value.source.blueprintRevisionTuple },
    right: { ...value.target, currentRevisionTuple: value.target.blueprintRevisionTuple },
    assessmentRelationships: value.assessmentRelationships,
    sharedQuestionIds: value.sharedQuestionIds,
    leftOnlyQuestionIds: value.sourceOnlyQuestionIds,
    rightOnlyQuestionIds: value.targetOnlyQuestionIds,
  };
}
