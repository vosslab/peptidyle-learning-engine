// Strict same-origin transport for frozen Blueprint Change Proposal evidence.

import type { ApiClient } from "../client";
import type {
  BlueprintChangeProposalClient,
  BlueprintProposalListQuery,
} from "../blueprint_change_proposal";
import type { BlueprintChangeProposalDetailView } from "../../../generated/api/BlueprintChangeProposalDetailView";
import type { RecordFilterChoices } from "../../components/data_sheet/data_sheet_fields";
import { decodeNonnegativeInteger, decodeRecord } from "../decoder";
import { field, requireOnlyFields } from "../decoders/shared";
import { decodeBlueprintCourseId } from "../decoders/blueprint_course";
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

function pagePath(
  path: string,
  pageNumber: number,
  pageSize: number,
  options: BlueprintProposalListQuery,
): string {
  if (!Number.isSafeInteger(pageNumber) || pageNumber < 1)
    throw new ApiProtocolError("Blueprint Change Proposal page number must be a positive integer");
  if (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 100)
    throw new ApiProtocolError(
      "Blueprint Change Proposal page size must be an integer from 1 through 100",
    );
  const query = new URLSearchParams();
  query.set("pageNumber", String(pageNumber));
  query.set("pageSize", String(pageSize));
  if (options.text.length > 160 || /[%_\\]/u.test(options.text))
    throw new ApiProtocolError("Blueprint Proposal search text is invalid");
  query.set("text", options.text.trim());
  query.set("sort", options.sort);
  query.set("status", [...new Set(options.status)].sort().join(","));
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
    listBlueprintProposalStatusChoices: async (
      target,
      text,
      selected,
    ): Promise<RecordFilterChoices> => {
      if (text.length > 160 || /[%_\\]/u.test(text))
        throw new ApiProtocolError("Blueprint Proposal search text is invalid");
      const path =
        target === undefined
          ? "/api/blueprint-change-proposals/facet-choices/state?scope=mine"
          : `${targetPath(target)}/facet-choices/state`;
      const counts = await proposalJson(
        fetchImplementation,
        basePath,
        `${path}${path.includes("?") ? "&" : "?"}text=${encodeURIComponent(text.trim())}`,
        (value, location = "response") => {
          const record = decodeRecord(value, location);
          requireOnlyFields(record, location, ["proposed", "stale", "accepted"]);
          return {
            proposed: decodeNonnegativeInteger(
              field(record, "proposed", location),
              `${location}.proposed`,
            ),
            stale: decodeNonnegativeInteger(field(record, "stale", location), `${location}.stale`),
            accepted: decodeNonnegativeInteger(
              field(record, "accepted", location),
              `${location}.accepted`,
            ),
          };
        },
      );
      const choices = (["proposed", "stale", "accepted"] as const).map((key) => ({
        key,
        label: key === "stale" ? "Older target basis" : key[0]!.toUpperCase() + key.slice(1),
        count: counts[key],
      }));
      return {
        choices,
        selected: choices.filter((choice) => selected.includes(choice.key)),
        nextCursor: null,
      };
    },
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
    listBlueprintChangeProposalsForTarget: (target, query, pageNumber, pageSize) =>
      proposalJson(
        fetchImplementation,
        basePath,
        pagePath(targetPath(target), pageNumber, pageSize, query),
        decodeBlueprintChangeProposalPageView,
      ),
    listMyBlueprintChangeProposals: (query, pageNumber, pageSize) =>
      proposalJson(
        fetchImplementation,
        basePath,
        pagePath("/api/blueprint-change-proposals?scope=mine", pageNumber, pageSize, query),
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
