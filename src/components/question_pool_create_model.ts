// Stable source binding for creating one Question Pool from a Published Question.

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
import type { QuestionBulkMetadataClient } from "../api/question_bulk_metadata";
import type { QuestionPickerEligibility } from "../features/question_picker/question_picker_model";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import type { QuestionLibraryBrowseRepository } from "../pages/library_page_model";
import { validateCanonicalQuestionIdSyntax } from "../question_id";
import type {
  QuestionPickerSelection,
  QuestionPickerSourceRepository,
} from "../features/question_picker/question_picker_model";

export interface QuestionPoolStartingQuestion {
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly questionTitle: string;
  readonly disciplineName: string;
  readonly subjectName: string;
  readonly questionType: QuestionDetails["summary"]["questionType"];
  readonly backend: QuestionDetails["summary"]["backend"];
}

function canonicalQuestionId(value: string): PublishedQuestionId {
  const questionId = validateCanonicalQuestionIdSyntax(value);
  if (questionId === null || questionId !== value) {
    throw new Error("The selected Question is no longer a canonical Published Question.");
  }
  return questionId;
}

/** Resolves one selected Question into the authoritative eligibility anchor for a new Pool. */
export async function questionPoolEligibilityForQuestion(
  questionId: PublishedQuestionId,
  getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>,
  getCurrentQuestionBulkMetadata: QuestionBulkMetadataClient["getCurrentQuestionBulkMetadata"],
): Promise<QuestionPickerEligibility> {
  const [details, metadata] = await Promise.all([
    getQuestionDetails(questionId),
    getCurrentQuestionBulkMetadata([questionId]),
  ]);
  const current = metadata[0];
  if (
    current === undefined ||
    current.questionId !== questionId ||
    details.summary.publishedQuestionRevisionTuple.publishedQuestionId !== questionId ||
    details.summary.metadata.questionLicense === null
  ) {
    throw new Error("The selected Question cannot establish Pool eligibility.");
  }
  return {
    disciplineUuid: current.disciplineUuid,
    subjectUuid: current.subjectUuid,
    questionType: details.summary.questionType,
    backend: details.summary.backend,
    excludedQuestionId: questionId,
  };
}

/** Reads current classification UUIDs while retaining the source Question's immutable type and backend. */
export async function questionPoolStartingEligibility(
  startingQuestion: QuestionPoolStartingQuestion,
  getCurrentQuestionBulkMetadata: QuestionBulkMetadataClient["getCurrentQuestionBulkMetadata"],
): Promise<QuestionPickerEligibility> {
  const questionId = canonicalQuestionId(
    startingQuestion.publishedQuestionRevisionTuple.publishedQuestionId,
  );
  const metadata = await getCurrentQuestionBulkMetadata([questionId]);
  const current = metadata[0];
  if (current === undefined || current.questionId !== questionId) {
    throw new Error("The starting Question's current classification could not be loaded.");
  }
  return {
    disciplineUuid: current.disciplineUuid,
    subjectUuid: current.subjectUuid,
    questionType: startingQuestion.questionType,
    backend: startingQuestion.backend,
    excludedQuestionId: questionId,
  };
}

function sameEligibility(
  metadata: PublishedQuestionSharedMetadata,
  details: QuestionDetails,
  eligibility: QuestionPickerEligibility,
): boolean {
  return (
    metadata.disciplineUuid === eligibility.disciplineUuid &&
    metadata.subjectUuid === eligibility.subjectUuid &&
    details.summary.questionType === eligibility.questionType &&
    details.summary.backend === eligibility.backend &&
    details.summary.metadata.questionLicense !== null
  );
}

function exactStartingRevision(
  startingQuestion: QuestionPoolStartingQuestion,
): PublishedQuestionRevisionTuple {
  const questionId = canonicalQuestionId(
    startingQuestion.publishedQuestionRevisionTuple.publishedQuestionId,
  );
  const revisionNumber = startingQuestion.publishedQuestionRevisionTuple.revisionNumber;
  if (!Number.isSafeInteger(revisionNumber) || revisionNumber < 1) {
    throw new Error("The starting Question Revision is no longer valid.");
  }
  return { publishedQuestionId: questionId, revisionNumber };
}

async function latestSelectedRevisions(
  selection: QuestionPickerSelection,
  getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>,
  getCurrentQuestionBulkMetadata: QuestionBulkMetadataClient["getCurrentQuestionBulkMetadata"],
  eligibility: QuestionPickerEligibility,
): Promise<ReadonlyArray<PublishedQuestionRevisionTuple>> {
  return await Promise.all(
    selection.questions.map(async (selected) => {
      const questionId = canonicalQuestionId(selected.questionId);
      const [detail, metadata] = await Promise.all([
        getQuestionDetails(questionId),
        getCurrentQuestionBulkMetadata([questionId]),
      ]);
      const current = metadata[0];
      if (
        current === undefined ||
        current.questionId !== questionId ||
        detail.summary.publishedQuestionRevisionTuple.publishedQuestionId !== questionId ||
        !sameEligibility(current, detail, eligibility)
      ) {
        throw new Error(
          "Every Pool Question must share the Pool's Discipline, Subject, Type, and Backend.",
        );
      }
      return detail.summary.publishedQuestionRevisionTuple;
    }),
  );
}

/** Pins an optional exact starting Revision first and rechecks every added Question's eligibility. */
export async function questionPoolMemberTuples(
  selection: QuestionPickerSelection,
  getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>,
  getCurrentQuestionBulkMetadata: QuestionBulkMetadataClient["getCurrentQuestionBulkMetadata"],
  eligibility: QuestionPickerEligibility,
  startingQuestion?: QuestionPoolStartingQuestion,
): Promise<ReadonlyArray<PublishedQuestionRevisionTuple>> {
  const additional = await latestSelectedRevisions(
    selection,
    getQuestionDetails,
    getCurrentQuestionBulkMetadata,
    eligibility,
  );
  return startingQuestion === undefined
    ? additional
    : [exactStartingRevision(startingQuestion), ...additional];
}

/** Source-bound Pool discovery has the same authoritative eligibility on every request. */
export function questionPoolSourcePickerRepository(
  library: QuestionLibraryBrowseRepository,
  eligibility: QuestionPickerEligibility,
): QuestionPickerSourceRepository {
  return {
    async search(request): Promise<unknown> {
      if (request.source.kind !== "library" && request.source.kind !== "sharedLibrary") {
        throw new Error("Choose the Question Library source for this Pool.");
      }
      return await library.search(
        {
          ...request.query,
          kind: "questions",
          membership: "all",
          discipline_uuid: eligibility.disciplineUuid,
          subject_uuid: eligibility.subjectUuid,
          questionType: eligibility.questionType,
          backend: eligibility.backend,
        },
        request.cursor,
        request.pageSize,
      );
    },
  };
}
