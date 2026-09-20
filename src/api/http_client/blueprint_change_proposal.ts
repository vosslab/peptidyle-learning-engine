// Strict same-origin transport for frozen Blueprint Change Proposal evidence.

import type { ApiClient } from "../client";
import type { BlueprintChangeProposalClient } from "../blueprint_change_proposal";
import type { BlueprintChangeProposalDetailView } from "../../../generated/api/BlueprintChangeProposalDetailView";
import { decodeBlueprintCourseId } from "../decoders/blueprint_course";
import { decodeCursor } from "../decoders/shared";
import {
  decodeBlueprintChangeProposalAcceptanceRequest,
  decodeBlueprintChangeProposalAcceptedView,
  decodeBlueprintChangeProposalCreateRequest,
  decodeBlueprintChangeProposalDetailView,
  decodeBlueprintChangeProposalPageView,
} from "../decoders/blueprint_change_proposal";
import { ApiProtocolError, ApiRequestError, BlueprintCourseConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_RESPONSE_CHARACTERS = 16 * 1_024 * 1_024;

function proposalId(value: string): string {
  if (!/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/iu.test(value))
    throw new ApiProtocolError("Blueprint Change Proposal ID must be an opaque UUID");
  return value;
}

function targetPath(target: string): string {
  return `/api/course-blueprints/${encodeURIComponent(decodeBlueprintCourseId(target, "target"))}/change-proposals`;
}

function pagePath(path: string, cursor?: string, pageSize?: number): string {
  if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 100))
    throw new ApiProtocolError(
      "Blueprint Change Proposal page size must be an integer from 1 through 100",
    );
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("cursor", decodeCursor(cursor, "cursor"));
  if (pageSize !== undefined) query.set("pageSize", String(pageSize));
  return query.size === 0 ? path : `${path}${path.includes("?") ? "&" : "?"}${query.toString()}`;
}

async function proposalJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  method: "GET" | "POST" = "GET",
  body?: unknown,
  expectedStatus: 200 | 201 = 200,
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, { method, body });
  requireNoStore(response, path);
  if (response.status === 412) throw new BlueprintCourseConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== expectedStatus)
    throw new ApiProtocolError(`API response ${path} must use status ${expectedStatus}`);
  return decoder(await boundedResponseJson(response, path, MAX_RESPONSE_CHARACTERS), "response");
}

/** Creates only the proposal API capability; no fork-update or Save route is reachable here. */
export function createBlueprintChangeProposalClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof BlueprintChangeProposalClient> {
  return {
    createBlueprintChangeProposal: async (
      target,
      request,
    ): Promise<BlueprintChangeProposalDetailView> => {
      const body = decodeBlueprintChangeProposalCreateRequest(request);
      if (body.targetRevisionTuple.blueprintCourseId !== target)
        throw new ApiProtocolError(
          "Blueprint Change Proposal body target must match its path target",
        );
      return proposalJson(
        fetchImplementation,
        basePath,
        targetPath(target),
        decodeBlueprintChangeProposalDetailView,
        "POST",
        body,
        201,
      );
    },
    listBlueprintChangeProposalsForTarget: (target, cursor, pageSize) =>
      proposalJson(
        fetchImplementation,
        basePath,
        pagePath(targetPath(target), cursor, pageSize),
        decodeBlueprintChangeProposalPageView,
      ),
    listMyBlueprintChangeProposals: (cursor, pageSize) =>
      proposalJson(
        fetchImplementation,
        basePath,
        pagePath("/api/blueprint-change-proposals?scope=mine", cursor, pageSize),
        decodeBlueprintChangeProposalPageView,
      ),
    getBlueprintChangeProposal: (id) =>
      proposalJson(
        fetchImplementation,
        basePath,
        `/api/blueprint-change-proposals/${encodeURIComponent(proposalId(id))}`,
        decodeBlueprintChangeProposalDetailView,
      ),
    acceptBlueprintChangeProposal: (id, request) =>
      proposalJson(
        fetchImplementation,
        basePath,
        `/api/blueprint-change-proposals/${encodeURIComponent(proposalId(id))}/acceptance`,
        decodeBlueprintChangeProposalAcceptedView,
        "POST",
        decodeBlueprintChangeProposalAcceptanceRequest(request),
      ),
  };
}
