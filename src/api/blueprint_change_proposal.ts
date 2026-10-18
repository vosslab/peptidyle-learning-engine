// Browser capability for frozen Blueprint Change Proposal creation, review, and one decision.

import type { BlueprintCourseId } from "../../generated/api/BlueprintCourseId";
import type { BlueprintChangeProposalAcceptanceRequest } from "../../generated/api/BlueprintChangeProposalAcceptanceRequest";
import type { BlueprintChangeProposalAcceptedView } from "../../generated/api/BlueprintChangeProposalAcceptedView";
import type { BlueprintChangeProposalCreateRequest } from "../../generated/api/BlueprintChangeProposalCreateRequest";
import type { BlueprintChangeProposalDetailView } from "../../generated/api/BlueprintChangeProposalDetailView";
import type { BlueprintChangeProposalPageView } from "../../generated/api/BlueprintChangeProposalPageView";
import type { RecordFilterChoices } from "../components/data_sheet/data_sheet_fields";

export type BlueprintProposalListQuery = {
  readonly text: string;
  readonly sort: "createdDescending" | "createdAscending";
  readonly status: ReadonlyArray<"proposed" | "stale" | "accepted">;
};

/** Participant-only browser operations for immutable Change Proposal evidence. */
export interface BlueprintChangeProposalClient {
  readonly listBlueprintProposalStatusChoices: (
    targetBlueprintCourseId: BlueprintCourseId | undefined,
    text: string,
    selected: BlueprintProposalListQuery["status"],
  ) => Promise<RecordFilterChoices>;
  readonly createBlueprintChangeProposal: (
    targetBlueprintCourseId: BlueprintCourseId,
    request: BlueprintChangeProposalCreateRequest,
  ) => Promise<BlueprintChangeProposalDetailView>;
  readonly listBlueprintChangeProposalsForTarget: (
    targetBlueprintCourseId: BlueprintCourseId,
    query: BlueprintProposalListQuery,
    pageNumber: number,
    pageSize: number,
  ) => Promise<BlueprintChangeProposalPageView>;
  readonly listMyBlueprintChangeProposals: (
    query: BlueprintProposalListQuery,
    pageNumber: number,
    pageSize: number,
  ) => Promise<BlueprintChangeProposalPageView>;
  readonly getBlueprintChangeProposal: (
    proposalId: string,
  ) => Promise<BlueprintChangeProposalDetailView>;
  readonly acceptBlueprintChangeProposal: (
    proposalId: string,
    request: BlueprintChangeProposalAcceptanceRequest,
  ) => Promise<BlueprintChangeProposalAcceptedView>;
}
