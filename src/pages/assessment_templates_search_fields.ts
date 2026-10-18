import type { AssessmentTemplate } from "../../generated/api/AssessmentTemplate";
import type { RecordField } from "../components/data_sheet/data_sheet_fields";
import { assessmentTypePresentation } from "../assessment_type_presentation";

export type TemplateSortKey = "name" | "assessmentType";
export type TemplateFilterKey = "assessmentType";

export const assessmentTemplateSearchFields: ReadonlyArray<
  RecordField<AssessmentTemplate, TemplateSortKey, TemplateFilterKey>
> = [
  {
    id: "name",
    label: "Template",
    role: "identity",
    sortKey: "name",
    value: (template) => template.name,
    defaultColumn: { priority: 0, width: "17rem" },
  },
  {
    id: "assessmentType",
    label: "Assessment Type",
    sortKey: "assessmentType",
    filterKey: "assessmentType",
    value: (template) => assessmentTypePresentation(template.assessmentType).label,
    defaultColumn: { priority: 1, width: "11rem" },
  },
];
