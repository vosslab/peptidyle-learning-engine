// Browser capability for frozen Blueprint Change Proposal creation, review, and one decision.

import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { BlueprintChangeProposalAcceptanceRequest } from "../../generated/api/BlueprintChangeProposalAcceptanceRequest";
import type { BlueprintChangeProposalAcceptedView } from "../../generated/api/BlueprintChangeProposalAcceptedView";
import type { BlueprintChangeProposalCreateRequest } from "../../generated/api/BlueprintChangeProposalCreateRequest";
import type { BlueprintChangeProposalDetailView } from "../../generated/api/BlueprintChangeProposalDetailView";
import type { BlueprintChangeProposalPageView } from "../../generated/api/BlueprintChangeProposalPageView";

/** Participant-only browser operations for immutable Change Proposal evidence. */
export interface BlueprintChangeProposalClient {
  readonly createBlueprintChangeProposal: (
    target: BlueprintCourseReference,
    request: BlueprintChangeProposalCreateRequest,
  ) => Promise<BlueprintChangeProposalDetailView>;
  readonly listBlueprintChangeProposalsForTarget: (
    target: BlueprintCourseReference,
    cursor?: string,
    pageSize?: number,
  ) => Promise<BlueprintChangeProposalPageView>;
  readonly listMyBlueprintChangeProposals: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<BlueprintChangeProposalPageView>;
  readonly getBlueprintChangeProposal: (
    proposalId: string,
  ) => Promise<BlueprintChangeProposalDetailView>;
  readonly acceptBlueprintChangeProposal: (
    proposalId: string,
    request: BlueprintChangeProposalAcceptanceRequest,
  ) => Promise<BlueprintChangeProposalAcceptedView>;
}
