// assignment_editor_repository.ts - narrow browser adapter for immutable assignment references.

import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { CourseId } from "../../generated/api/CourseId";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentEditorDetail, AssignmentEditorInput } from "../api/contracts";
import type { ApiClient } from "../api/client";
import type { AssignmentCatalogRow } from "./assignment_editor_model";

export interface AssignmentEditorRepository {
  readonly load: (assignment: AssignmentId) => Promise<AssignmentEditorDetail>;
  readonly save: (
    course: CourseId,
    assignment: AssignmentId,
    input: AssignmentEditorInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly searchPublished: (text: string) => Promise<ReadonlyArray<AssignmentCatalogRow>>;
}

function catalogQuery(text: string): CatalogSearchQuery {
  return {
    text: text.trim() === "" ? null : text.trim(),
    taxonomy: [],
    capabilities: [],
    licenses: [],
    statistics: "any",
    cursor: null,
    pageSize: 20,
  };
}

/** Adapts only immutable catalog tuples; question payloads never enter editor state. */
export function createAssignmentEditorRepository(client: ApiClient): AssignmentEditorRepository {
  return {
    load: async (assignment) => await client.getAssignmentEditor(assignment),
    save: async (course, assignment, input, revision) =>
      await client.saveAssignment(course, assignment, input, revision),
    searchPublished: async (text): Promise<ReadonlyArray<AssignmentCatalogRow>> => {
      const page = await client.searchCatalog(catalogQuery(text));
      return page.items.map((item) => ({
        reference: { problem: item.problem, version: item.version },
        title: item.metadata.title,
      }));
    },
  };
}
