// assessment_editor_repository.ts - Questions discovery and source-selection browser adapter.

import type { AssessmentId } from "../../generated/api/AssessmentId";
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
import type { AssessmentQuestionRow } from "./assessment_editor_model";

export interface AssessmentEditorRepository {
  readonly resolvePublished: (questionId: string) => Promise<AssessmentQuestionRow>;
  /** Sources and answer-free rows for the shared D2 picker. */
  readonly listQuestionPickerSources: (
    course: CourseId,
    exclude?: AssessmentId,
  ) => Promise<ReadonlyArray<QuestionPickerSource>>;
  readonly questionPickerRepository: QuestionPickerSourceRepository;
  readonly listRetainedAssessmentQuestionSources: (
    course: CourseId,
    exclude?: AssessmentId,
  ) => Promise<ReadonlyArray<RetainedAssessmentQuestionSource>>;
}

/** Answer-free Questions projected from one retained Course Instance Assessment. */
export interface RetainedAssessmentQuestionSource {
  readonly assessmentId: AssessmentId;
  readonly title: string;
  readonly questions: ReadonlyArray<AssessmentQuestionRow>;
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
  readonly backend: AssessmentQuestionRow["backend"];
}): AssessmentQuestionRow {
  return {
    questionId: item.questionId,
    questionTitle: item.metadata.questionTitle,
    backend: item.backend,
  };
}

/** Questions reads published metadata and reusable sources through this adapter. */
export function createAssessmentEditorRepository(client: ApiClient): AssessmentEditorRepository {
  const questionLibrary = createQuestionLibraryRepository(client);
  const myQuestions = createQuestionLibraryRepository(client, "authoredByCurrentAccount");
  const questionLibraryPicker = questionLibraryPickerRepository(questionLibrary, myQuestions);
  const blueprintCourse = blueprintCourseQuestionPickerRepository(client);
  const questionPickerRepository: QuestionPickerSourceRepository = {
    async search(request: QuestionPickerSearchRequest): Promise<unknown> {
      if (request.source.kind === "blueprintCourseAssessment") {
        return await blueprintCourse.search(request);
      }
      if (request.source.kind !== "retainedAssessment")
        return await questionLibraryPicker.search(request);
      const assessment = await client.getAssessmentWorkspace(
        request.source.retainedAssessment.course,
        request.source.retainedAssessment.assessment,
      );
      if (request.cursor !== null) return page([], null);
      const fixedRows = assessment.entries
        .filter(
          (entry): entry is Extract<typeof entry, { readonly kind: "fixedQuestion" }> =>
            entry.kind === "fixedQuestion" && entry.availability === "available",
        )
        .map((entry) => ({
          displayId: entry.questionId,
          questionTitle: entry.questionTitle,
          summary: "Active fixed question retained in this assessment.",
          questionFormat: null,
          authorNames: [],
          capabilities: entry.capabilities,
          questionLicense: null,
          evidence: { state: "unavailable" as const },
        }));
      const poolRows = assessment.entries.flatMap((entry) =>
        entry.kind === "questionPool"
          ? entry.items.map((item) => ({
              displayId: item.questionId,
              questionTitle: item.questionTitle,
              summary: "Question retained in this assessment pool.",
              questionFormat: null,
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
      const retainedAssessments = await listRetainedAssessmentQuestionSources(
        client,
        course,
        exclude,
      );
      const blueprintCourses = await listAllBlueprintCourses(client);
      const blueprintAssessments = await listBlueprintAssessmentSources(client, blueprintCourses);
      return [
        { kind: "library", label: "Question Library" },
        { kind: "mine", label: "My Questions" },
        ...retainedAssessments.map((assessment) => ({
          kind: "retainedAssessment" as const,
          label: `Assessment: ${assessment.title}`,
          retainedAssessment: { course, assessment: assessment.assessmentId },
        })),
        ...blueprintAssessments,
      ];
    },
    listRetainedAssessmentQuestionSources: async (
      course,
      exclude,
    ): Promise<ReadonlyArray<RetainedAssessmentQuestionSource>> =>
      await listRetainedAssessmentQuestionSources(client, course, exclude),
  };
}

async function listRetainedAssessmentQuestionSources(
  client: ApiClient,
  course: CourseId,
  exclude: AssessmentId | undefined,
): Promise<ReadonlyArray<RetainedAssessmentQuestionSource>> {
  const assessments = [];
  let cursor: string | undefined;
  const seenCursors = new Set<string>();
  while (true) {
    const page = await client.listAssessments(course, cursor);
    assessments.push(...page.items);
    if (page.nextCursor === null) break;
    if (seenCursors.has(page.nextCursor)) {
      throw new Error("Assessment pagination repeated a cursor.");
    }
    seenCursors.add(page.nextCursor);
    cursor = page.nextCursor;
  }
  const details = await Promise.all(
    assessments
      .filter((assessment) => assessment.id !== exclude)
      .map(async (assessment) => await client.getAssessmentWorkspace(course, assessment.id)),
  );
  return details.map((assessment) => ({
    assessmentId: assessment.id,
    title: assessment.title,
    questions: assessment.entries
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

async function listBlueprintAssessmentSources(
  client: ApiClient,
  courses: Awaited<ReturnType<ApiClient["listBlueprintCourses"]>>["items"],
): Promise<
  ReadonlyArray<Extract<QuestionPickerSource, { readonly kind: "blueprintCourseAssessment" }>>
> {
  const currentRevisions = await Promise.all(
    courses
      .filter((course): course is BlueprintCourseSummaryView => course.availability === "public")
      .map(async (course) => {
        const revision = course.current_revision;
        return await client.getBlueprintRevision(revision.reference, revision.revision);
      }),
  );
  return currentRevisions.flatMap((revision) =>
    revision.modules.flatMap((module) =>
      module.assessments.map((content) => ({
        kind: "blueprintCourseAssessment" as const,
        source: {
          blueprint_revision: revision.blueprintRevision,
          blueprint_assessment_reference: content.blueprint_assessment_reference,
        },
        label: `Blueprint Course Revision ${revision.blueprintRevision.revision}: ${module.label} - ${content.content.title}`,
      })),
    ),
  );
}
