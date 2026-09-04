// assignment_editor_repository.ts - Questions discovery and source-selection browser adapter.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { ApiClient } from "../api/client";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import type {
  QuestionLibraryBrowsePage,
  QuestionLibraryBrowseQuery,
  QuestionLibraryBrowseRow,
} from "./library_page_model";
import type {
  QuestionPickerSearchRequest,
  QuestionPickerSource,
  QuestionPickerSourceRepository,
} from "../features/question_picker";
import {
  blueprintCourseQuestionPickerRepository,
  questionLibraryPickerRepository,
} from "../features/question_picker/question_picker_model";
import type { AssignmentQuestionRow } from "./assignment_editor_model";

export interface AssignmentEditorRepository {
  readonly resolvePublished: (questionId: string) => Promise<AssignmentQuestionRow>;
  /** Sources and answer-free rows for the shared D2 picker. */
  readonly listQuestionPickerSources: (
    course: CourseId,
    exclude?: AssignmentId,
  ) => Promise<ReadonlyArray<QuestionPickerSource>>;
  readonly questionPickerRepository: QuestionPickerSourceRepository;
  readonly listRetainedAssignmentQuestionSources: (
    course: CourseId,
    exclude?: AssignmentId,
  ) => Promise<ReadonlyArray<RetainedAssignmentQuestionSource>>;
}

/** Answer-free Questions projected from one retained Course Instance Assignment. */
export interface RetainedAssignmentQuestionSource {
  readonly assignmentId: AssignmentId;
  readonly title: string;
  readonly questions: ReadonlyArray<AssignmentQuestionRow>;
}

function retainedQueryMatches(
  row: QuestionLibraryBrowseRow,
  query: QuestionLibraryBrowseQuery,
): boolean {
  const search = query.search.trim().toLocaleLowerCase();
  if (search !== "") {
    const haystack = [row.questionTitle, row.displayId, row.summary, ...row.authorNames]
      .join(" ")
      .toLocaleLowerCase();
    if (!haystack.includes(search)) return false;
  }
  if (query.authorName !== null && !row.authorNames.includes(query.authorName)) return false;
  if (query.backend !== null || query.questionType !== null || query.tag !== null) return false;
  if (query.capability !== null && !row.capabilities.includes(query.capability)) return false;
  if (query.questionLicense !== null && row.questionLicense !== query.questionLicense) return false;
  if (query.usedInMyCourses === "used") return false;
  return true;
}

function page(
  rows: ReadonlyArray<QuestionLibraryBrowseRow>,
  nextCursor: string | null,
): QuestionLibraryBrowsePage {
  return { items: rows, nextCursor, aggregates: [] };
}

function questionRow(item: {
  readonly questionId: string;
  readonly metadata: { readonly questionTitle: string };
  readonly backend: AssignmentQuestionRow["backend"];
}): AssignmentQuestionRow {
  return {
    questionId: item.questionId,
    questionTitle: item.metadata.questionTitle,
    backend: item.backend,
  };
}

/** Questions reads published metadata and reusable sources through this adapter. */
export function createAssignmentEditorRepository(client: ApiClient): AssignmentEditorRepository {
  const questionLibrary = createQuestionLibraryRepository(client);
  const myQuestions = createQuestionLibraryRepository(client, "authoredByCurrentAccount");
  const questionLibraryPicker = questionLibraryPickerRepository(questionLibrary, myQuestions);
  const blueprintCourse = blueprintCourseQuestionPickerRepository(client);
  const questionPickerRepository: QuestionPickerSourceRepository = {
    async search(request: QuestionPickerSearchRequest): Promise<unknown> {
      if (request.source.kind === "blueprintCourseAssignment") {
        return await blueprintCourse.search(request);
      }
      if (request.source.kind !== "retainedAssignment")
        return await questionLibraryPicker.search(request);
      const assignment = await client.getAssignmentWorkspace(
        request.source.retainedAssignment.course,
        request.source.retainedAssignment.assignment,
      );
      if (request.cursor !== null) return page([], null);
      const fixedRows = assignment.entries
        .filter(
          (entry): entry is Extract<typeof entry, { readonly kind: "fixedQuestion" }> =>
            entry.kind === "fixedQuestion" && entry.availability === "available",
        )
        .map((entry) => ({
          displayId: entry.questionId,
          questionTitle: entry.questionTitle,
          summary: "Active fixed question retained in this assignment.",
          authorNames: [],
          capabilities: entry.capabilities,
          questionLicense: null,
          evidence: { state: "unavailable" as const },
        }));
      const poolRows = assignment.entries.flatMap((entry) =>
        entry.kind === "questionPool"
          ? entry.items.map((item) => ({
              displayId: item.questionId,
              questionTitle: item.questionTitle,
              summary: "Question retained in this assignment pool.",
              authorNames: [],
              capabilities: [],
              questionLicense: null,
              evidence: { state: "unavailable" as const },
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
    resolvePublished: async (questionId) => questionRow(await client.resolveQuestion(questionId)),
    questionPickerRepository,
    listQuestionPickerSources: async (
      course,
      exclude,
    ): Promise<ReadonlyArray<QuestionPickerSource>> => {
      const retainedAssignments = await listRetainedAssignmentQuestionSources(
        client,
        course,
        exclude,
      );
      const blueprintCourses = await listAllBlueprintCourses(client);
      const blueprintAssignments = await listBlueprintAssignmentSources(client, blueprintCourses);
      return [
        { kind: "library", label: "Question Library" },
        { kind: "mine", label: "My Questions" },
        ...retainedAssignments.map((assignment) => ({
          kind: "retainedAssignment" as const,
          label: `Assignment: ${assignment.title}`,
          retainedAssignment: { course, assignment: assignment.assignmentId },
        })),
        ...blueprintAssignments,
      ];
    },
    listRetainedAssignmentQuestionSources: async (
      course,
      exclude,
    ): Promise<ReadonlyArray<RetainedAssignmentQuestionSource>> =>
      await listRetainedAssignmentQuestionSources(client, course, exclude),
  };
}

async function listRetainedAssignmentQuestionSources(
  client: ApiClient,
  course: CourseId,
  exclude: AssignmentId | undefined,
): Promise<ReadonlyArray<RetainedAssignmentQuestionSource>> {
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
          entry.kind === "fixedQuestion" && entry.availability === "available",
      )
      .map((entry) => ({
        questionId: entry.questionId,
        questionTitle: entry.questionTitle,
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
      module.assignments.map((content) => ({
        kind: "blueprintCourseAssignment" as const,
        source: {
          reference: blueprintCourse.reference,
          revision: blueprintCourse.revision,
          blueprint_assignment_reference: content.blueprint_assignment_reference,
        },
        label: `Blueprint Course: ${blueprintCourse.title} - ${module.label} - ${content.content.title}`,
      })),
    ),
  );
}
