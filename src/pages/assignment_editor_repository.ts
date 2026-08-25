// assignment_editor_repository.ts - QID-only assignment editor browser adapter.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseReference } from "../../generated/api/CourseReference";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type {
  AssignmentCreateInput,
  AssignmentEditorDetail,
  AssignmentEditorInput,
  AddAssignmentItemInput,
  ReplaceAssignmentItemQuestionInput,
  PoolDrawPreview,
} from "../api/contracts";
import type { ApiClient } from "../api/client";
import type { AssignmentCatalogRow } from "./assignment_editor_model";

export interface AssignmentEditorRepository {
  readonly load: (assignment: AssignmentId) => Promise<AssignmentEditorDetail>;
  readonly create: (
    course: CourseId,
    input: AssignmentCreateInput,
  ) => Promise<AssignmentEditorDetail>;
  readonly save: (
    course: CourseId,
    assignment: AssignmentId,
    input: AssignmentEditorInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly saveTeachingSettings: (
    course: CourseId,
    assignment: AssignmentId,
    settings: InstructorAssignmentTeachingSettingsLocal,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly add: (
    course: CourseId,
    assignment: AssignmentId,
    input: AddAssignmentItemInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly remove: (
    course: CourseId,
    assignment: AssignmentId,
    itemId: string,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly replace: (
    course: CourseId,
    assignment: AssignmentId,
    itemId: string,
    input: ReplaceAssignmentItemQuestionInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly previewPoolDraw: (
    course: CourseReference,
    assignment: AssignmentReference,
    revision: string,
    groupPosition: number,
  ) => Promise<PoolDrawPreview>;
  readonly searchPublished: (text: string) => Promise<ReadonlyArray<AssignmentCatalogRow>>;
  readonly resolvePublished: (questionId: string) => Promise<AssignmentCatalogRow>;
  readonly listReusableAssignments: (
    course: CourseId,
    exclude?: AssignmentId,
  ) => Promise<ReadonlyArray<ReusableAssignment>>;
}

export interface ReusableAssignment {
  readonly title: string;
  readonly questions: ReadonlyArray<AssignmentCatalogRow>;
}

function catalogRow(item: {
  readonly questionId: string;
  readonly metadata: { readonly title: string };
  readonly backend: AssignmentCatalogRow["backend"];
}): AssignmentCatalogRow {
  return { questionId: item.questionId, title: item.metadata.title, backend: item.backend };
}

function previewRevision(assignmentRevision: string): string {
  const match = /^"([1-9][0-9]*)"$/u.exec(assignmentRevision);
  const revision = match?.[1];
  if (revision === undefined || BigInt(revision) > 9_223_372_036_854_775_807n)
    throw new Error(
      "A saved assignment needs one positive strong revision before previewing a pool.",
    );
  return revision;
}

export function createAssignmentEditorRepository(client: ApiClient): AssignmentEditorRepository {
  return {
    load: async (assignment) => await client.getAssignmentEditor(assignment),
    create: async (course, input) => await client.createAssignment(course, input),
    save: async (course, assignment, input, revision) =>
      await client.saveAssignment(course, assignment, input, revision),
    saveTeachingSettings: async (course, assignment, settings, revision) =>
      await client.saveAssignmentTeachingSettings(course, assignment, settings, revision),
    add: async (course, assignment, input, revision) =>
      await client.addAssignmentItem(course, assignment, input, revision),
    remove: async (course, assignment, itemId, revision) =>
      await client.removeAssignmentItem(course, assignment, itemId, revision),
    replace: async (course, assignment, itemId, input, revision) =>
      await client.replaceAssignmentItemQuestion(course, assignment, itemId, input, revision),
    previewPoolDraw: async (course, assignment, revision, groupPosition) =>
      await client.previewPoolDraw(course, assignment, previewRevision(revision), groupPosition),
    searchPublished: async (text) =>
      (
        await client.searchCatalog({
          text: text.trim() || null,
          taxonomy: [],
          capabilities: [],
          licenses: [],
          statistics: "any",
          cursor: null,
          pageSize: 20,
        })
      ).items.map(catalogRow),
    resolvePublished: async (questionId) =>
      catalogRow(await client.resolveCatalogProblem(questionId)),
    listReusableAssignments: async (
      course,
      exclude,
    ): Promise<ReadonlyArray<ReusableAssignment>> => {
      const assignments = [];
      let cursor: string | undefined;
      for (let pageNumber = 0; pageNumber < 5; pageNumber += 1) {
        const page = await client.listAssignments(course, cursor);
        assignments.push(...page.items);
        if (page.nextCursor === null || assignments.length >= 100) break;
        cursor = page.nextCursor;
      }
      const details = await Promise.all(
        assignments
          .filter((assignment) => assignment.id !== exclude)
          .slice(0, 100)
          .map(async (assignment) => await client.getAssignmentEditor(assignment.id)),
      );
      return details.map((assignment) => ({
        title: assignment.title,
        questions: assignment.items
          .filter((item) => item.deliveryState === "active")
          .map((item) => ({
            questionId: item.questionId,
            title: item.title,
            backend: item.backend,
          })),
      }));
    },
  };
}
