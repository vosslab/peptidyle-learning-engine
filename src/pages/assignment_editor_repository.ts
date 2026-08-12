// assignment_editor_repository.ts - narrow browser adapter for immutable assignment references.

import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { CourseId } from "../../generated/api/CourseId";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { AssignmentEditorDetail, AssignmentEditorInput } from "../api/contracts";
import type { ApiClient } from "../api/client";
import { assignmentProblemLabel, type AssignmentCatalogRow } from "./assignment_editor_model";

export interface AssignmentEditorRepository {
  readonly load: (assignment: AssignmentId) => Promise<AssignmentEditorDetail>;
  readonly create: (
    course: CourseId,
    input: AssignmentEditorInput,
  ) => Promise<AssignmentEditorDetail>;
  readonly save: (
    course: CourseId,
    assignment: AssignmentId,
    input: AssignmentEditorInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly searchPublished: (text: string) => Promise<ReadonlyArray<AssignmentCatalogRow>>;
  /** Resolves one exact, copyable P-n-vn identity to an immutable assignment tuple. */
  readonly resolvePublished: (displayReference: string) => Promise<AssignmentCatalogRow>;
  /** Resolves safe display metadata for immutable references already on an assignment. */
  readonly describePublished: (
    references: ReadonlyArray<ProblemVersionRef>,
  ) => Promise<ReadonlyArray<AssignmentCatalogRow>>;
}

function catalogRow(item: CatalogProblemSummary): AssignmentCatalogRow {
  return {
    reference: { problem: item.problem, version: item.version },
    publicId: item.publicId,
    versionNumber: item.versionNumber,
    title: item.metadata.title,
    backend: item.backend,
  };
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
    create: async (course, input) => await client.createAssignment(course, input),
    save: async (course, assignment, input, revision) =>
      await client.saveAssignment(course, assignment, input, revision),
    searchPublished: async (text): Promise<ReadonlyArray<AssignmentCatalogRow>> => {
      const page = await client.searchCatalog(catalogQuery(text));
      return page.items.map(catalogRow);
    },
    resolvePublished: async (displayReference): Promise<AssignmentCatalogRow> => {
      const row = catalogRow(await client.resolveCatalogProblem(displayReference));
      if (assignmentProblemLabel(row) !== displayReference) {
        throw new Error("The catalog resolved an unrelated question identity.");
      }
      return row;
    },
    describePublished: async (references) =>
      await Promise.all(
        references.map(async (reference) => {
          const detail = await client.getCatalogProblemDetail(reference.problem, reference.version);
          return catalogRow(detail.summary);
        }),
      ),
  };
}
