// assignment_editor_repository.ts - Questions discovery and reuse browser adapter.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { AlphaCourseSummaryView } from "../../generated/api/AlphaCourseSummaryView";
import type { BlueprintSummaryView } from "../../generated/api/BlueprintSummaryView";
import type { ApiClient } from "../api/client";
import { createCatalogRepository } from "../api/catalog_repository";
import type { CatalogBrowsePage, CatalogBrowseQuery, CatalogBrowseRow } from "./library_page_model";
import type {
  ProblemPickerSearchRequest,
  ProblemPickerSource,
  ProblemPickerSourceRepository,
} from "../features/problem_picker";
import { reusableCurriculumProblemPickerRepository } from "../features/problem_picker/problem_picker_model";
import { createProblemCurationRepository } from "../features/problem_curation/problem_curation_repository";
import type { AssignmentCatalogRow } from "./assignment_editor_model";

export interface AssignmentEditorRepository {
  readonly resolvePublished: (questionId: string) => Promise<AssignmentCatalogRow>;
  /** Sources and answer-free rows for the shared D2 picker. */
  readonly listProblemPickerSources: (
    course: CourseId,
    exclude?: AssignmentId,
  ) => Promise<ReadonlyArray<ProblemPickerSource>>;
  readonly problemPickerRepository: ProblemPickerSourceRepository;
  readonly listReusableAssignments: (
    course: CourseId,
    exclude?: AssignmentId,
  ) => Promise<ReadonlyArray<ReusableAssignment>>;
}

export interface ReusableAssignment {
  readonly assignmentId: AssignmentId;
  readonly title: string;
  readonly questions: ReadonlyArray<AssignmentCatalogRow>;
}

function retainedQueryMatches(row: CatalogBrowseRow, query: CatalogBrowseQuery): boolean {
  const search = query.search.trim().toLocaleLowerCase();
  if (search !== "") {
    const haystack = [row.title, row.displayId, row.summary, ...row.taxonomy, ...row.byline]
      .join(" ")
      .toLocaleLowerCase();
    if (!haystack.includes(search)) return false;
  }
  if (query.byline !== null && !row.byline.includes(query.byline)) return false;
  if (query.backend !== null || query.responseFamily !== null || query.tag !== null) return false;
  if (query.taxonomy !== null && !row.taxonomy.includes(query.taxonomy)) return false;
  if (query.capability !== null && !row.capabilities.includes(query.capability)) return false;
  if (query.license !== null && row.license !== query.license) return false;
  if (query.evidence === "available" || query.usedInMyCourses === "used") return false;
  return true;
}

function page(rows: ReadonlyArray<CatalogBrowseRow>, nextCursor: string | null): CatalogBrowsePage {
  return { items: rows, nextCursor, aggregates: [] };
}

function catalogRow(item: {
  readonly questionId: string;
  readonly metadata: { readonly title: string };
  readonly backend: AssignmentCatalogRow["backend"];
}): AssignmentCatalogRow {
  return { questionId: item.questionId, title: item.metadata.title, backend: item.backend };
}

/** Questions reads published metadata and reusable sources through this adapter. */
export function createAssignmentEditorRepository(client: ApiClient): AssignmentEditorRepository {
  const catalog = createCatalogRepository(client);
  const curation = createProblemCurationRepository(client, catalog);
  const reusableCurriculum = reusableCurriculumProblemPickerRepository(client);
  const problemPickerRepository: ProblemPickerSourceRepository = {
    async search(request: ProblemPickerSearchRequest): Promise<unknown> {
      if (
        request.source.kind === "personalBlueprint" ||
        request.source.kind === "alphaCurriculum"
      ) {
        return await reusableCurriculum.search(request);
      }
      if (request.source.kind !== "retainedAssignment")
        return await curation.picker.search(request);
      const assignment = await client.getAssignmentWorkspace(
        request.source.retainedAssignment.course,
        request.source.retainedAssignment.assignment,
      );
      if (request.cursor !== null) return page([], null);
      const fixedRows = assignment.items
        .filter((item) => item.deliveryState === "active")
        .map((item) => ({
          displayId: item.questionId,
          title: item.title,
          summary: "Active fixed question retained in this assignment.",
          byline: [],
          taxonomy: [],
          capabilities: item.capabilities,
          license: "allRightsReserved",
          evidence: { state: "insufficientEvidence" as const },
        }));
      const poolRows = assignment.selectionGroups.flatMap((group) =>
        group.candidates.map((candidate) => ({
          displayId: candidate.questionId,
          title: candidate.title,
          summary: "Question retained in this assignment pool.",
          byline: [],
          taxonomy: [],
          capabilities: [],
          license: "allRightsReserved",
          evidence: { state: "insufficientEvidence" as const },
        })),
      );
      const rows = [...fixedRows, ...poolRows].filter((row) =>
        retainedQueryMatches(row, request.query),
      );
      return page(rows, null);
    },
  };
  return {
    resolvePublished: async (questionId) =>
      catalogRow(await client.resolveCatalogProblem(questionId)),
    problemPickerRepository,
    listProblemPickerSources: async (
      course,
      exclude,
    ): Promise<ReadonlyArray<ProblemPickerSource>> => {
      const collections = await client.listProblemCollections();
      const reusable = await listReusableAssignments(client, course, exclude);
      const blueprints = await listAllBlueprints(client);
      const alphaCourses = await listAllAlphaCourses(client);
      const alphaDefinitions = await listAlphaDefinitionSources(client, alphaCourses);
      return [
        { kind: "catalog", label: "Library" },
        { kind: "mine", label: "My published questions" },
        { kind: "favorites", label: "Favorites" },
        ...collections.items
          .filter((collection) => collection.kind === "named")
          .map((collection) => ({
            kind: "collection" as const,
            label: collection.title,
            collection: collection.reference,
          })),
        ...reusable.map((assignment) => ({
          kind: "retainedAssignment" as const,
          label: `Assignment: ${assignment.title}`,
          retainedAssignment: { course, assignment: assignment.assignmentId },
        })),
        ...blueprints.map((blueprint) => ({
          kind: "personalBlueprint" as const,
          label: `Blueprint: ${blueprint.title}`,
          blueprint: blueprint.reference,
        })),
        ...alphaDefinitions,
      ];
    },
    listReusableAssignments: async (course, exclude): Promise<ReadonlyArray<ReusableAssignment>> =>
      await listReusableAssignments(client, course, exclude),
  };
}

async function listReusableAssignments(
  client: ApiClient,
  course: CourseId,
  exclude: AssignmentId | undefined,
): Promise<ReadonlyArray<ReusableAssignment>> {
  const assignments = [];
  let cursor: string | undefined;
  const seenCursors = new Set<string>();
  while (true) {
    const page = await client.listAssignments(course, cursor);
    assignments.push(...page.items);
    if (page.nextCursor === null) break;
    if (seenCursors.has(page.nextCursor)) {
      throw new Error("Assignment pagination repeated a cursor.");
    }
    seenCursors.add(page.nextCursor);
    cursor = page.nextCursor;
  }
  const details = await Promise.all(
    assignments
      .filter((assignment) => assignment.id !== exclude)
      .map(async (assignment) => await client.getAssignmentWorkspace(course, assignment.id)),
  );
  return details.map((assignment) => ({
    assignmentId: assignment.id,
    title: assignment.title,
    questions: assignment.items
      .filter((item) => item.deliveryState === "active")
      .map((item) => ({
        questionId: item.questionId,
        title: item.title,
        backend: item.backend,
      })),
  }));
}

async function listAllBlueprints(client: ApiClient): Promise<ReadonlyArray<BlueprintSummaryView>> {
  const items: BlueprintSummaryView[] = [];
  let cursor: string | undefined;
  const seenCursors = new Set<string>();
  while (true) {
    const page = await client.listBlueprints(cursor);
    items.push(...page.items);
    if (page.nextCursor === null) return items;
    if (seenCursors.has(page.nextCursor))
      throw new Error("Blueprint pagination repeated a cursor.");
    seenCursors.add(page.nextCursor);
    cursor = page.nextCursor;
  }
}

async function listAllAlphaCourses(
  client: ApiClient,
): Promise<ReadonlyArray<AlphaCourseSummaryView>> {
  const items: AlphaCourseSummaryView[] = [];
  let cursor: string | undefined;
  const seenCursors = new Set<string>();
  while (true) {
    const page = await client.listAlphaCourses(cursor);
    items.push(...page.items);
    if (page.nextCursor === null) return items;
    if (seenCursors.has(page.nextCursor))
      throw new Error("Alpha curriculum pagination repeated a cursor.");
    seenCursors.add(page.nextCursor);
    cursor = page.nextCursor;
  }
}

async function listAlphaDefinitionSources(
  client: ApiClient,
  alphaCourses: Awaited<ReturnType<ApiClient["listAlphaCourses"]>>["items"],
): Promise<ReadonlyArray<Extract<ProblemPickerSource, { readonly kind: "alphaCurriculum" }>>> {
  const currentCourses = await Promise.all(
    alphaCourses.map(async (alpha) => await client.getAlphaCourse(alpha.reference)),
  );
  return currentCourses.flatMap(({ alpha }) =>
    alpha.modules.flatMap((module, moduleIndex) =>
      module.definitions.map((definition, assignmentIndex) => ({
        kind: "alphaCurriculum" as const,
        alpha: alpha.reference,
        modulePosition: moduleIndex + 1,
        assignmentPosition: assignmentIndex + 1,
        label: `Alpha: ${alpha.title} - ${module.label} - ${definition.title}`,
      })),
    ),
  );
}
