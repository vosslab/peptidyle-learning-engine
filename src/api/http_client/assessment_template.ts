// Strict same-origin transport for private Instructor-owned Assessment Templates.

import type { AssessmentTemplate } from "../../../generated/api/AssessmentTemplate";
import type { AssessmentTemplateId } from "../../../generated/api/AssessmentTemplateId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
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
import {
  assertResponseMatchesPositiveNumber,
  ifMatchHeaderForPositiveNumber,
} from "./conditional_request";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseCourseInstanceId } from "../../navigation/public_route";

function templatePath(id?: AssessmentTemplateId): string {
  if (id === undefined) return "/api/assessment-templates";
  if (!/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/iu.test(id)) {
    throw new ApiProtocolError("Assessment Template ID must be a UUID");
  }
  return `/api/assessment-templates/${encodeURIComponent(id)}`;
}

function createFromTemplatePath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(courseInstanceId)}/assessments/from-template`;
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

function requireMatchingTemplateEditNumber(
  response: Response,
  template: AssessmentTemplate,
  path: string,
): void {
  assertResponseMatchesPositiveNumber(
    response,
    template.editNumber,
    path,
    "Assessment Template Edit Number",
  );
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
    readonly expectedAssessmentTemplateEditNumber?: string;
    readonly status?: 200 | 201;
  } = {},
): Promise<{ readonly template: AssessmentTemplate; readonly response: Response }> {
  const headers: Record<string, string> =
    options.expectedAssessmentTemplateEditNumber === undefined
      ? {}
      : {
          "if-match": ifMatchHeaderForPositiveNumber(
            options.expectedAssessmentTemplateEditNumber,
            path,
            "Assessment Template Edit Number",
          ),
        };
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
  requireMatchingTemplateEditNumber(result.response, result.template, path);
  return { template: result.template };
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
    saveAssessmentTemplate: async (
      id,
      input,
      expectedAssessmentTemplateEditNumber,
    ): Promise<AssessmentTemplateResponse> => {
      const path = templatePath(id);
      const result = await templateJson(fetchImplementation, basePath, path, {
        method: "PUT",
        body: decodeSaveAssessmentTemplateInput(input),
        expectedAssessmentTemplateEditNumber,
        status: 200,
      });
      requireRequestedTemplate(result.template, id, path);
      return templateResponse(result, path);
    },
    createAssessmentFromTemplate: async (
      courseInstanceId,
      input,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = createFromTemplatePath(courseInstanceId);
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
      assertResponseMatchesPositiveNumber(
        response,
        workspace.assessmentEditNumber,
        path,
        "Assessment Edit Number",
      );
      return { workspace };
    },
  };
}
