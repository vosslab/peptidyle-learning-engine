// Browser contract for private Instructor-owned Assessment Templates.

import type { AssessmentTemplate } from "../../generated/api/AssessmentTemplate";
import type { AssessmentTemplateId } from "../../generated/api/AssessmentTemplateId";
import type { AssessmentTemplateName } from "../../generated/api/AssessmentTemplateName";
import type { AssessmentTemplateSettings } from "../../generated/api/AssessmentTemplateSettings";
import type { AssessmentType } from "../../generated/api/AssessmentType";
import type { AssessmentTitle } from "../../generated/api/AssessmentTitle";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { LiveAssessmentWorkspaceResponse } from "./assessment_release";

/** Closed creation input; server-owned settings begin from the model defaults. */
export interface CreateAssessmentTemplateInput {
  readonly name: AssessmentTemplateName;
  readonly assessmentType: AssessmentType;
}

/** Full current-value replacement for one private Assessment Template. */
export interface SaveAssessmentTemplateInput {
  readonly name: AssessmentTemplateName;
  readonly assessmentType: AssessmentType;
  readonly settings: AssessmentTemplateSettings;
}

/** Closed command that copies one private Template into an authorized Course Instance. */
export interface CreateAssessmentFromTemplateInput {
  readonly templateId: AssessmentTemplateId;
  readonly title: AssessmentTitle;
}

/** One complete Template together with the exact strong ETag for its next replacement. */
export interface AssessmentTemplateResponse {
  readonly template: AssessmentTemplate;
  readonly etag: string;
}

/** Same-origin private Template collection and compare-and-swap boundary. */
export interface AssessmentTemplateClient {
  readonly listAssessmentTemplates: () => Promise<ReadonlyArray<AssessmentTemplate>>;
  readonly createAssessmentTemplate: (
    input: CreateAssessmentTemplateInput,
  ) => Promise<AssessmentTemplateResponse>;
  readonly getAssessmentTemplate: (id: AssessmentTemplateId) => Promise<AssessmentTemplateResponse>;
  readonly saveAssessmentTemplate: (
    id: AssessmentTemplateId,
    input: SaveAssessmentTemplateInput,
    etag: string,
  ) => Promise<AssessmentTemplateResponse>;
  /** Creates an Unreleased live Assessment from one private Template and returns its strong ETag. */
  readonly createAssessmentFromTemplate: (
    course: CourseInstanceReference,
    input: CreateAssessmentFromTemplateInput,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
}
