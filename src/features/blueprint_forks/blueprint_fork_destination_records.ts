// Complete destination projections retain authored identity independently of display queries.
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import type { BlueprintComparisonSide } from "../../../generated/api/BlueprintComparisonSide";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintForkApplySelection } from "../../../generated/api/BlueprintForkApplySelection";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import type { RecordAction, RecordContent } from "../../components/data_sheet/record_content";
import type { RecordField } from "../../components/data_sheet/data_sheet_fields";
import { reorderedRecordListRows } from "../../components/data_sheet/record_reorder";
import { destinationKey, type Layout } from "./blueprint_fork_apply_model";

type Labels = BlueprintForkApplySelection["sourceModuleLabels"];
type Contents = BlueprintForkApplySelection["sourceAssessments"];
type AssessmentDestination = Layout["assessments"][number];

export type DestinationModuleRecord = {
  readonly id: string;
  readonly moduleIndex: number;
  readonly label: string;
  readonly context: string;
  readonly assessmentCount: number;
  readonly sourceContext: string;
};

export type DestinationAssessmentRecord = {
  readonly id: string;
  readonly entry: AssessmentDestination;
  readonly moduleId: string;
  readonly moduleIndex: number;
  readonly assessmentIndex: number;
  readonly moduleContext: string;
  readonly title: string;
  readonly assessmentType: AssessmentType | undefined;
  readonly entryCount: number;
  readonly sourceContext: string;
};

export function destinationModuleLabel(
  review: BlueprintComparisonView,
  labels: Labels,
  module: Layout["module"],
): string {
  const sourceId =
    module.kind === "newFromSource"
      ? module.sourceModuleId
      : labels.find((copy) => copy.targetModuleId === module.targetModuleId)?.sourceModuleId;
  const label =
    sourceId !== undefined
      ? review.left.modules.find((item) => item.blueprintModuleId === sourceId)?.label
      : module.kind === "existing"
        ? review.right.modules.find((item) => item.blueprintModuleId === module.targetModuleId)
            ?.label
        : undefined;
  return label ?? "Module";
}

function assessmentSource(
  review: BlueprintComparisonView,
  contents: Contents,
  entry: AssessmentDestination,
): { readonly side: BlueprintComparisonSide; readonly id: string; readonly label: string } {
  const sourceId =
    entry.kind === "newFromSource"
      ? entry.sourceAssessmentId
      : contents.find((copy) => copy.targetAssessmentId === entry.targetAssessmentId)
          ?.sourceAssessmentId;
  return sourceId !== undefined
    ? { side: review.left, id: sourceId, label: "Source" }
    : {
        side: review.right,
        id: entry.kind === "existing" ? entry.targetAssessmentId : entry.sourceAssessmentId,
        label: "Receiving Blueprint",
      };
}

export function destinationAssessmentTitle(
  review: BlueprintComparisonView,
  contents: Contents,
  entry: AssessmentDestination,
): string {
  const source = assessmentSource(review, contents, entry);
  return (
    source.side.assessments.find((item) => item.blueprintAssessmentId === source.id)?.content
      .title ?? "Assessment"
  );
}

function revisionContext(side: BlueprintComparisonSide, label: string): string {
  const tuple = side.currentRevisionTuple;
  return `${label}: ${side.names.shortName}; Blueprint ID ${tuple.blueprintCourseId}; Revision ${tuple.revisionNumber}`;
}

/** Module and Assessment positions describe the complete current destination, before paging. */
export function destinationRecords(
  review: BlueprintComparisonView,
  layout: ReadonlyArray<Layout>,
  labels: Labels,
  contents: Contents,
): {
  readonly modules: ReadonlyArray<DestinationModuleRecord>;
  readonly assessments: ReadonlyArray<DestinationAssessmentRecord>;
} {
  const modules = layout.map((row, moduleIndex): DestinationModuleRecord => {
    const sourceId =
      row.module.kind === "newFromSource"
        ? row.module.sourceModuleId
        : labels.find(
            (copy) =>
              row.module.kind === "existing" && copy.targetModuleId === row.module.targetModuleId,
          )?.sourceModuleId;
    const side = sourceId === undefined ? review.right : review.left;
    const id = sourceId ?? (row.module.kind === "existing" ? row.module.targetModuleId : "");
    const sourceModule = side.modules.find((item) => item.blueprintModuleId === id);
    const label = destinationModuleLabel(review, labels, row.module);
    return {
      id: destinationKey(row.module),
      moduleIndex,
      label,
      context: `${moduleIndex + 1}. ${label}`,
      assessmentCount: row.assessments.length,
      sourceContext: `${revisionContext(side, sourceId === undefined ? "Receiving Blueprint" : "Source")}; Module ID ${id}; Module position ${sourceModule === undefined ? "unavailable" : sourceModule.position + 1}`,
    };
  });
  const assessments = layout.flatMap((row, moduleIndex) =>
    row.assessments.map((entry, assessmentIndex): DestinationAssessmentRecord => {
      const source = assessmentSource(review, contents, entry);
      const snapshot = source.side.assessments.find(
        (item) => item.blueprintAssessmentId === source.id,
      );
      const sourceModule = source.side.modules.find(
        (item) => item.blueprintModuleId === snapshot?.blueprintModuleId,
      );
      return {
        id: destinationKey(entry),
        entry,
        moduleId: destinationKey(row.module),
        moduleIndex,
        assessmentIndex,
        moduleContext: modules[moduleIndex]!.context,
        title: snapshot?.content.title ?? "Assessment",
        assessmentType: snapshot?.content.assessment_type,
        entryCount: snapshot?.content.entries.length ?? 0,
        sourceContext: `${revisionContext(source.side, source.label)}; Assessment ID ${source.id}; Module ${sourceModule?.label ?? "unavailable"}; Module position ${sourceModule === undefined ? "unavailable" : sourceModule.position + 1}; Assessment position ${snapshot === undefined ? "unavailable" : snapshot.position + 1}`,
      };
    }),
  );
  return { modules, assessments };
}

/** Resolve identities against the full authored layout, never a filtered or paged collection. */
export function moveDestinationModule(layout: Layout[], id: string, offset: number): Layout[] {
  const index = layout.findIndex((row) => destinationKey(row.module) === id);
  const target = index + offset;
  if (index < 0 || target < 0 || target >= layout.length || offset === 0) return layout;
  return reorderedRecordListRows(layout, index, target);
}

/** Assessment movement is confined to its own complete sibling order. */
export function moveDestinationAssessment(
  layout: Layout[],
  moduleId: string,
  id: string,
  offset: number,
): Layout[] {
  const moduleIndex = layout.findIndex((row) => destinationKey(row.module) === moduleId);
  const row = layout[moduleIndex];
  if (row === undefined) return layout;
  const index = row.assessments.findIndex((entry) => destinationKey(entry) === id);
  const target = index + offset;
  if (index < 0 || target < 0 || target >= row.assessments.length || offset === 0) return layout;
  return layout.map((candidate, position) =>
    position === moduleIndex
      ? { ...candidate, assessments: reorderedRecordListRows(row.assessments, index, target) }
      : candidate,
  );
}

export type DestinationModuleSort = "label" | "position" | "count";
export type DestinationAssessmentSort = "title" | "module" | "position" | "type" | "count";
export type DestinationAssessmentFilter = "module" | "type";

export const destinationModuleFields: ReadonlyArray<
  RecordField<DestinationModuleRecord, DestinationModuleSort, "label">
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
    label: "Destination position",
    sortKey: "position",
    value: (row) => row.moduleIndex + 1,
    defaultColumn: { priority: 1, width: "9rem" },
  },
  {
    id: "assessment_count",
    label: "Assessments",
    sortKey: "count",
    value: (row) => row.assessmentCount,
    defaultColumn: { priority: 2, width: "8rem" },
  },
  {
    id: "source",
    label: "Label source",
    value: (row) => row.sourceContext,
    defaultColumn: { priority: 3, width: "20rem" },
  },
];

export const destinationAssessmentFields: ReadonlyArray<
  RecordField<DestinationAssessmentRecord, DestinationAssessmentSort, DestinationAssessmentFilter>
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
    label: "Destination Module",
    sortKey: "module",
    filterKey: "module",
    value: (row) => row.moduleContext,
    defaultColumn: { priority: 1, width: "14rem" },
  },
  {
    id: "position",
    label: "Destination Assessment position",
    sortKey: "position",
    value: (row) => row.assessmentIndex + 1,
    defaultColumn: { priority: 2, width: "10rem" },
  },
  {
    id: "type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (row) =>
      row.assessmentType === undefined ? "" : assessmentTypePresentation(row.assessmentType).label,
    defaultColumn: { priority: 3, width: "12rem" },
  },
  {
    id: "entry_count",
    label: "Library Objects",
    sortKey: "count",
    value: (row) => row.entryCount,
    defaultColumn: { priority: 4, width: "9rem" },
  },
  {
    id: "source",
    label: "Content source",
    value: (row) => row.sourceContext,
    defaultColumn: { priority: 5, width: "22rem" },
  },
];

export function destinationModuleContent(
  row: DestinationModuleRecord,
  actions: ReadonlyArray<RecordAction>,
): RecordContent {
  return {
    title: row.label,
    details: [
      { kind: "text", label: "Destination position", value: String(row.moduleIndex + 1) },
      { kind: "text", label: "Assessments", value: String(row.assessmentCount) },
      { kind: "text", label: "Label source", value: row.sourceContext },
    ],
    actions,
  };
}

export function destinationAssessmentContent(
  row: DestinationAssessmentRecord,
  actions: ReadonlyArray<RecordAction>,
): RecordContent {
  return {
    title: row.title,
    details: [
      { kind: "text", label: "Destination Module", value: row.moduleContext },
      {
        kind: "text",
        label: "Destination Assessment position",
        value: String(row.assessmentIndex + 1),
      },
      ...(row.assessmentType === undefined
        ? []
        : [{ kind: "assessmentType" as const, value: row.assessmentType }]),
      { kind: "text", label: "Library Objects", value: String(row.entryCount) },
      { kind: "text", label: "Content source", value: row.sourceContext },
    ],
    actions,
  };
}
