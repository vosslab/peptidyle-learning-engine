// assignment_editor_repository.ts - QID-only assignment editor browser adapter.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type {
  AssignmentCreateInput,
  AssignmentEditorDetail,
  AssignmentEditorInput,
  AddAssignmentItemInput,
  ReplaceAssignmentItemQuestionInput,
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

export function createAssignmentEditorRepository(client: ApiClient): AssignmentEditorRepository {
  return {
    load: async (assignment) => await client.getAssignmentEditor(assignment),
    create: async (course, input) => await client.createAssignment(course, input),
    save: async (course, assignment, input, revision) =>
      await client.saveAssignment(course, assignment, input, revision),
    add: async (course, assignment, input, revision) =>
      await client.addAssignmentItem(course, assignment, input, revision),
    remove: async (course, assignment, itemId, revision) =>
      await client.removeAssignmentItem(course, assignment, itemId, revision),
    replace: async (course, assignment, itemId, input, revision) =>
      await client.replaceAssignmentItemQuestion(course, assignment, itemId, input, revision),
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
      return assignments
        .filter((assignment) => assignment.id !== exclude)
        .slice(0, 100)
        .map((assignment) => ({
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
