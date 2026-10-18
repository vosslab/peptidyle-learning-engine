// Complete local record projections for the saved Blueprint hierarchy.
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import type { RecordAction, RecordContent } from "../../components/data_sheet/record_content";
import type { RecordField } from "../../components/data_sheet/record_fields";

type BlueprintModule = ReplaceBlueprintCourseContentInput["modules"][number];

export type BlueprintStructureModule = {
  readonly id: string;
  readonly moduleIndex: number;
  readonly label: string;
  readonly context: string;
  readonly assessmentCount: number;
};

export type BlueprintStructureAssessment = {
  readonly id: string;
  readonly moduleIndex: number;
  readonly assessmentIndex: number;
  readonly moduleContext: string;
  readonly title: string;
  readonly assessmentType: AssessmentType;
  readonly entryCount: number;
};

export type BlueprintModuleSort = "label" | "position" | "assessmentCount";
export type BlueprintModuleFilter = "label";
export type BlueprintAssessmentSort = "title" | "module" | "position" | "type" | "entryCount";
export type BlueprintAssessmentFilter = "module" | "type";

/** Retain saved identities and source positions independently of display order. */
export function blueprintStructureModules(
  modules: ReadonlyArray<BlueprintModule>,
): ReadonlyArray<BlueprintStructureModule> {
  return modules.map((module, moduleIndex) => ({
    id:
      module.choice.kind === "retained"
        ? module.choice.blueprint_module_id
        : `new-module-${moduleIndex}`,
    moduleIndex,
    label: module.label,
    context: `${moduleIndex + 1}. ${module.label}`,
    assessmentCount: module.assessments.length,
  }));
}

/** Flatten only the presentation; source Module and Assessment positions stay unchanged. */
export function blueprintStructureAssessments(
  modules: ReadonlyArray<BlueprintModule>,
): ReadonlyArray<BlueprintStructureAssessment> {
  return modules.flatMap((module, moduleIndex) =>
    module.assessments.map((assessment, assessmentIndex) => ({
      id:
        assessment.choice.kind === "retained"
          ? assessment.choice.blueprint_assessment_id
          : `new-assessment-${moduleIndex}-${assessmentIndex}`,
      moduleIndex,
      assessmentIndex,
      moduleContext: `${moduleIndex + 1}. ${module.label}`,
      title: assessment.content.title,
      assessmentType: assessment.content.assessment_type,
      entryCount: assessment.content.entries.length,
    })),
  );
}

export const blueprintStructureModuleFields: ReadonlyArray<
  RecordField<BlueprintStructureModule, BlueprintModuleSort, BlueprintModuleFilter>
> = [
  {
    id: "module",
    role: "identity",
    label: "Module",
    sortKey: "label",
    filterKey: "label",
    value: (row) => row.label,
    defaultColumn: { priority: 0, width: "14rem" },
  },
  {
    id: "position",
    label: "Saved position",
    sortKey: "position",
    value: (row) => row.moduleIndex + 1,
    defaultColumn: { priority: 2, width: "8rem" },
  },
  {
    id: "assessment_count",
    label: "Assessments",
    sortKey: "assessmentCount",
    value: (row) => row.assessmentCount,
    defaultColumn: { priority: 1, width: "8rem" },
  },
];

export const blueprintStructureAssessmentFields: ReadonlyArray<
  RecordField<BlueprintStructureAssessment, BlueprintAssessmentSort, BlueprintAssessmentFilter>
> = [
  {
    id: "assessment",
    role: "identity",
    label: "Assessment",
    sortKey: "title",
    value: (row) => row.title,
    defaultColumn: { priority: 0, width: "18rem" },
  },
  {
    id: "module",
    label: "Module",
    sortKey: "module",
    filterKey: "module",
    value: (row) => row.moduleContext,
    defaultColumn: { priority: 1, width: "14rem" },
  },
  {
    id: "position",
    label: "Saved Assessment position",
    sortKey: "position",
    value: (row) => row.assessmentIndex + 1,
    defaultColumn: { priority: 3, width: "10rem" },
  },
  {
    id: "assessment_type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (row) => assessmentTypePresentation(row.assessmentType).label,
    defaultColumn: { priority: 2, width: "12rem" },
  },
  {
    id: "entry_count",
    label: "Library Objects",
    sortKey: "entryCount",
    value: (row) => row.entryCount,
    defaultColumn: { priority: 4, width: "9rem" },
  },
];

export function blueprintStructureModuleContent(
  row: BlueprintStructureModule,
  action: RecordAction,
): RecordContent {
  return {
    title: row.label,
    details: [
      { kind: "text", label: "Saved position", value: String(row.moduleIndex + 1) },
      { kind: "text", label: "Assessments", value: String(row.assessmentCount) },
    ],
    actions: [action],
  };
}

export function blueprintStructureAssessmentContent(
  row: BlueprintStructureAssessment,
  action: RecordAction,
): RecordContent {
  return {
    title: row.title,
    details: [
      { kind: "text", label: "Module", value: row.moduleContext },
      {
        kind: "text",
        label: "Saved Assessment position",
        value: String(row.assessmentIndex + 1),
      },
      { kind: "assessmentType", value: row.assessmentType },
      { kind: "text", label: "Library Objects", value: String(row.entryCount) },
    ],
    actions: [action],
  };
}
