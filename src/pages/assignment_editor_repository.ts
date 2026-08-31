// assignment_editor_repository.ts - Questions discovery and reuse browser adapter.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { ApiClient } from "../api/client";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import type { QuestionSearchPage, QuestionSearchQuery, QuestionSearchResult } from "./library_page_model";
import type {
  QuestionPickerSearchRequest,
  QuestionPickerSource,
  QuestionPickerSourceRepository,
} from "../features/question_picker";
import { reusableCurriculumQuestionPickerRepository } from "../features/question_picker/question_picker_model";
import { createQuestionCurationRepository } from "../features/question_curation/question_curation_repository";
import type { AssignmentCatalogRow } from "./assignment_editor_model";

export interface AssignmentEditorRepository {
  readonly resolvePublished: (questionId: string) => Promise<AssignmentCatalogRow>;
  /** Sources and answer-free rows for the shared D2 picker. */
  readonly listQuestionPickerSources: (
    course: CourseId,
    exclude?: AssignmentId,
  ) => Promise<ReadonlyArray<QuestionPickerSource>>;
  readonly questionPickerRepository: QuestionPickerSourceRepository;
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

function retainedQueryMatches(row: QuestionSearchResult, query: QuestionSearchQuery): boolean {
  const search = query.search.trim().toLocaleLowerCase();
  if (search !== "") {
    const haystack = [row.title, row.displayId, row.summary, ...row.taxonomy, ...row.byline]
      .join(" ")
      .toLocaleLowerCase();
    if (!haystack.includes(search)) return false;
  }
  if (query.byline !== null && !row.byline.includes(query.byline)) return false;
  if (query.backend !== null || query.questionType !== null || query.tag !== null) return false;
  if (query.taxonomy !== null && !row.taxonomy.includes(query.taxonomy)) return false;
  if (query.capability !== null && !row.capabilities.includes(query.capability)) return false;
  if (query.license !== null && row.license !== query.license) return false;
  if (query.evidence === "available" || query.usedInMyCourses === "used") return false;
  return true;
}

function page(rows: ReadonlyArray<QuestionSearchResult>, nextCursor: string | null): QuestionSearchPage {
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
  const catalog = createQuestionLibraryRepository(client);
  const curation = createQuestionCurationRepository(client, catalog);
  const reusableCurriculum = reusableCurriculumQuestionPickerRepository(client);
  const questionPickerRepository: QuestionPickerSourceRepository = {
    async search(request: QuestionPickerSearchRequest): Promise<unknown> {
      if (request.source.kind === "blueprintCourseAssignment") {
        return await reusableCurriculum.search(request);
      }
      if (request.source.kind !== "retainedAssignment")
        return await curation.picker.search(request);
      const assignment = await client.getAssignmentWorkspace(
        request.source.retainedAssignment.course,
        request.source.retainedAssignment.assignment,
      );
      if (request.cursor !== null) return page([], null);
      const fixedRows = assignment.entries
        .filter(
          (entry): entry is Extract<typeof entry, { readonly kind: "fixedQuestion" }> =>
            entry.kind === "fixedQuestion" && entry.deliveryState === "active",
        )
        .map((entry) => ({
          displayId: entry.questionId,
          title: entry.title,
          summary: "Active fixed question retained in this assignment.",
          byline: [],
          taxonomy: [],
          capabilities: entry.capabilities,
          license: "allRightsReserved",
          evidence: { state: "insufficientEvidence" as const },
        }));
      const poolRows = assignment.entries.flatMap((entry) =>
        entry.kind === "questionPool"
          ? entry.candidates.map((candidate) => ({
          displayId: candidate.questionId,
          title: candidate.title,
          summary: "Question retained in this assignment pool.",
          byline: [],
          taxonomy: [],
          capabilities: [],
          license: "allRightsReserved",
          evidence: { state: "insufficientEvidence" as const },
            }))
          : [],
      );
      const rows = [...fixedRows, ...poolRows].filter((row) =>
        retainedQueryMatches(row, request.query),
      );
      return page(rows, null);
    },
  };
  return {
    resolvePublished: async (questionId) =>
      catalogRow(await client.resolveQuestion(questionId)),
    questionPickerRepository,
    listQuestionPickerSources: async (
      course,
      exclude,
    ): Promise<ReadonlyArray<QuestionPickerSource>> => {
      const folders = await client.listQuestionFolders();
      const reusable = await listReusableAssignments(client, course, exclude);
      const blueprintCourses = await listAllBlueprintCourses(client);
      const blueprintAssignments = await listBlueprintAssignmentSources(client, blueprintCourses);
      return [
        { kind: "library", label: "Library" },
        { kind: "mine", label: "My Questions" },
        ...folders.items
          .map((folder) => ({
            kind: "folder" as const,
            label: folder.title,
            folder: folder.reference,
          })),
        ...reusable.map((assignment) => ({
          kind: "retainedAssignment" as const,
          label: `Assignment: ${assignment.title}`,
          retainedAssignment: { course, assignment: assignment.assignmentId },
        })),
        ...blueprintAssignments,
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
    questions: assignment.entries
      .filter(
        (entry): entry is Extract<typeof entry, { readonly kind: "fixedQuestion" }> =>
          entry.kind === "fixedQuestion" && entry.deliveryState === "active",
      )
      .map((entry) => ({
        questionId: entry.questionId,
        title: entry.title,
        backend: entry.backend,
      })),
  }));
}

async function listAllBlueprintCourses(
  client: ApiClient,
): Promise<ReadonlyArray<BlueprintCourseSummaryView>> {
  const items: BlueprintCourseSummaryView[] = [];
  let cursor: string | undefined;
  const seenCursors = new Set<string>();
  while (true) {
    const page = await client.listBlueprintCourses(cursor);
    items.push(...page.items);
    if (page.nextCursor === null) return items;
    if (seenCursors.has(page.nextCursor))
      throw new Error("Blueprint Course pagination repeated a cursor.");
    seenCursors.add(page.nextCursor);
    cursor = page.nextCursor;
  }
}

async function listBlueprintAssignmentSources(
  client: ApiClient,
  courses: Awaited<ReturnType<ApiClient["listBlueprintCourses"]>>["items"],
): Promise<
  ReadonlyArray<Extract<QuestionPickerSource, { readonly kind: "blueprintCourseAssignment" }>>
> {
  const currentCourses = await Promise.all(
    courses.map(async (course) => await client.getBlueprintCourse(course.reference)),
  );
  return currentCourses.flatMap(({ blueprintCourse }) =>
    blueprintCourse.modules.flatMap((module) =>
      module.definitions.map((definition) => ({
        kind: "blueprintCourseAssignment" as const,
        source: {
          reference: blueprintCourse.reference,
          revision: blueprintCourse.revision,
          assignment_id: definition.assignment_id,
        },
        label: `Blueprint Course: ${blueprintCourse.title} - ${module.label} - ${definition.definition.title}`,
      })),
    ),
  );
}
