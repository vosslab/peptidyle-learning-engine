// Strict same-origin transport for private Instructor-owned Assessment Templates.

import type { AssessmentTemplate } from "../../../generated/api/AssessmentTemplate";
import type { AssessmentTemplateId } from "../../../generated/api/AssessmentTemplateId";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type {
  AssessmentTemplateClient,
  AssessmentTemplateResponse,
  CreateAssessmentFromTemplateInput,
} from "../assessment_template";
import type { LiveAssessmentWorkspaceResponse } from "../assessment_release";
import {
  decodeAssessmentTemplate,
  decodeAssessmentTemplateList,
  decodeCreateAssessmentTemplateInput,
  decodeSaveAssessmentTemplateInput,
} from "../decoders/assessment_template";
import { decodeRecord, decodeUuid } from "../decoder";
import { decodeLiveAssessmentWorkspace } from "../decoders/assessment_release";
import { decodeAssessmentTitle, field, requireOnlyFields } from "../decoders/shared";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseCourseInstanceReference } from "../../navigation/public_route";

function templatePath(id?: AssessmentTemplateId): string {
  if (id === undefined) return "/api/assessment-templates";
  if (!/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/iu.test(id)) {
    throw new ApiProtocolError("Assessment Template ID must be a UUID");
  }
  return `/api/assessment-templates/${encodeURIComponent(id)}`;
}

function createFromTemplatePath(course: CourseInstanceReference): string {
  if (parseCourseInstanceReference(course) === null) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/assessments/from-template`;
}

function decodeCreateAssessmentFromTemplateInput(
  value: unknown,
  path = "request",
): CreateAssessmentFromTemplateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["templateId", "title"]);
  return {
    templateId: decodeUuid(field(record, "templateId", path), `${path}.templateId`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
  };
}

function quotedStrongEtag(etag: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(etag) || BigInt(etag.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(
      `API ${path} If-Match must be one quoted strong Assessment Template Edit Number`,
    );
  }
  return etag;
}

function requireMatchingEtag(
  response: Response,
  template: AssessmentTemplate,
  path: string,
): string {
  const etag = response.headers.get("etag");
  if (etag !== `"${template.editNumber}"`) {
    throw new ApiProtocolError(
      `API response ${path} ETag must match its Assessment Template Edit Number`,
    );
  }
  return etag;
}

function requireWorkspaceEtag(response: Response, editNumber: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || etag !== `"${editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its Assessment Edit Number`);
  }
  return etag;
}

function requireRequestedTemplate(
  template: AssessmentTemplate,
  expectedId: AssessmentTemplateId,
  path: string,
): void {
  if (template.id !== expectedId) {
    throw new ApiProtocolError(
      `API response ${path} must retain the requested Assessment Template`,
    );
  }
}

async function templateJson(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: string;
    readonly status?: 200 | 201;
  } = {},
): Promise<{ readonly template: AssessmentTemplate; readonly response: Response }> {
  const headers: Record<string, string> =
    options.etag === undefined ? {} : { "if-match": quotedStrongEtag(options.etag, path) };
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.status !== undefined && response.status !== options.status) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.status}`);
  }
  const template = decodeAssessmentTemplate(await boundedResponseJson(response, path), "response");
  return { template, response };
}

function templateResponse(
  result: { readonly template: AssessmentTemplate; readonly response: Response },
  path: string,
): AssessmentTemplateResponse {
  return {
    template: result.template,
    etag: requireMatchingEtag(result.response, result.template, path),
  };
}

/** Composes the private Template capability without widening shared HTTP transport behavior. */
export function createAssessmentTemplateClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof AssessmentTemplateClient> {
  return {
    listAssessmentTemplates: async (): Promise<ReadonlyArray<AssessmentTemplate>> => {
      const path = templatePath();
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      return decodeAssessmentTemplateList(await boundedResponseJson(response, path), "response");
    },
    createAssessmentTemplate: async (input): Promise<AssessmentTemplateResponse> => {
      const path = templatePath();
      const result = await templateJson(fetchImplementation, basePath, path, {
        method: "POST",
        body: decodeCreateAssessmentTemplateInput(input),
        status: 201,
      });
      return templateResponse(result, path);
    },
    getAssessmentTemplate: async (id): Promise<AssessmentTemplateResponse> => {
      const path = templatePath(id);
      const result = await templateJson(fetchImplementation, basePath, path, { status: 200 });
      requireRequestedTemplate(result.template, id, path);
      return templateResponse(result, path);
    },
    saveAssessmentTemplate: async (id, input, etag): Promise<AssessmentTemplateResponse> => {
      const path = templatePath(id);
      const result = await templateJson(fetchImplementation, basePath, path, {
        method: "PUT",
        body: decodeSaveAssessmentTemplateInput(input),
        etag,
        status: 200,
      });
      requireRequestedTemplate(result.template, id, path);
      return templateResponse(result, path);
    },
    createAssessmentFromTemplate: async (
      course,
      input,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = createFromTemplatePath(course);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        body: decodeCreateAssessmentFromTemplateInput(input),
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 201) {
        throw new ApiProtocolError(`API response ${path} must use status 201`);
      }
      const workspace = decodeLiveAssessmentWorkspace(
        await boundedResponseJson(response, path),
        "response",
      );
      return {
        workspace,
        etag: requireWorkspaceEtag(response, workspace.editNumber, path),
      };
    },
  };
}
