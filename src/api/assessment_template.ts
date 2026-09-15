// Browser contract for private Instructor-owned Assessment Templates.

import type { AssessmentTemplate } from "../../generated/api/AssessmentTemplate";
import type { AssessmentTemplateId } from "../../generated/api/AssessmentTemplateId";
import type { AssessmentTemplateName } from "../../generated/api/AssessmentTemplateName";
import type { AssessmentTemplateSettings } from "../../generated/api/AssessmentTemplateSettings";
import type { AssessmentType } from "../../generated/api/AssessmentType";

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
}
