// Stable source binding for creating one Pool from selected Published Questions.

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
import type { QuestionBulkMetadataClient } from "../api/question_bulk_metadata";
import type { QuestionAvailabilityClient } from "../api/question_availability";
import type { ContentClassificationClient } from "../api/content_classification";
import type { QuestionPickerEligibility } from "../features/question_picker/question_picker_model";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import { validateCanonicalQuestionIdSyntax } from "../question_id";
import type { QuestionPickerSelection } from "../features/question_picker/question_picker_model";

export interface QuestionPoolStartingQuestion {
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly questionTitle: string;
  readonly disciplineName: string;
  readonly subjectName: string;
  readonly questionType: QuestionDetails["summary"]["questionType"];
  readonly backend: QuestionDetails["summary"]["backend"];
}

export interface QuestionPoolStartingContext {
  readonly eligibility: QuestionPickerEligibility;
  readonly disciplineName: string;
  readonly subjectName: string;
}

/** Safe, specific conflict copy for the Instructor's selected Questions. */
export class QuestionPoolSelectionError extends Error {}

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
  getCurrentQuestionSharedMetadata: QuestionBulkMetadataClient["getCurrentQuestionSharedMetadata"],
): Promise<QuestionPickerEligibility> {
  const [details, metadata] = await Promise.all([
    getQuestionDetails(questionId),
    getCurrentQuestionSharedMetadata([questionId]),
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
    excludedQuestionIds: [questionId],
  };
}

/** Reads current classification UUIDs while retaining each exact selected Revision tuple. */
export async function questionPoolStartingEligibility(
  startingQuestions: ReadonlyArray<QuestionPoolStartingQuestion>,
  getCurrentQuestionSharedMetadata: QuestionBulkMetadataClient["getCurrentQuestionSharedMetadata"],
  classificationClient: Pick<
    ContentClassificationClient,
    "listDisciplinesIncludingRetired" | "listSubjects"
  >,
): Promise<QuestionPoolStartingContext> {
  const first = startingQuestions[0];
  if (first === undefined)
    throw new QuestionPoolSelectionError("Choose at least one starting Question.");
  const ids = startingQuestions.map((question) =>
    canonicalQuestionId(question.publishedQuestionRevisionTuple.publishedQuestionId),
  );
  if (new Set(ids).size !== ids.length) {
    throw new QuestionPoolSelectionError("Choose one Revision per starting Published Question.");
  }
  // ASVS 2.2.1/2.2.3: compare every member against the Pool's shared constraints.
  // The create command repeats these checks at its trusted service/database boundary.
  const metadata = await getCurrentQuestionSharedMetadata([...ids].sort());
  if (metadata.length !== ids.length) {
    throw new Error("The starting Questions' current classification could not be loaded.");
  }
  const byId = new Map(metadata.map((current) => [current.questionId, current]));
  const firstMetadata = byId.get(ids[0]!);
  if (firstMetadata === undefined) {
    throw new Error("The starting Questions' current classification could not be loaded.");
  }
  for (const question of startingQuestions) {
    const id = question.publishedQuestionRevisionTuple.publishedQuestionId;
    const current = byId.get(id);
    if (current === undefined) {
      throw new Error("The starting Questions' current classification could not be loaded.");
    }
    if (
      current.disciplineUuid !== firstMetadata.disciplineUuid ||
      current.subjectUuid !== firstMetadata.subjectUuid
    ) {
      throw new QuestionPoolSelectionError(
        "Selected Questions have different Disciplines or Subjects. Select Questions that share both.",
      );
    }
    if (question.questionType !== first.questionType || question.backend !== first.backend) {
      throw new QuestionPoolSelectionError(
        "Selected Questions have different Types or Backends. Select interchangeable Questions.",
      );
    }
  }
  const [disciplines, subjects] = await Promise.all([
    classificationClient.listDisciplinesIncludingRetired(),
    classificationClient.listSubjects(firstMetadata.disciplineUuid),
  ]);
  const discipline = disciplines.find((item) => item.uuid === firstMetadata.disciplineUuid);
  const subject = subjects.find((item) => item.uuid === firstMetadata.subjectUuid);
  if (discipline === undefined || subject === undefined) {
    throw new Error("The starting Questions' current classification could not be loaded.");
  }
  return {
    disciplineName: discipline.name,
    subjectName: subject.name,
    eligibility: {
      disciplineUuid: firstMetadata.disciplineUuid,
      subjectUuid: firstMetadata.subjectUuid,
      questionType: first.questionType,
      backend: first.backend,
      excludedQuestionIds: ids,
    },
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

async function exactSelectedRevisions(
  selection: QuestionPickerSelection,
  getQuestionRevision: QuestionAvailabilityClient["getQuestionRevision"],
  getCurrentQuestionSharedMetadata: QuestionBulkMetadataClient["getCurrentQuestionSharedMetadata"],
  eligibility: QuestionPickerEligibility,
): Promise<ReadonlyArray<PublishedQuestionRevisionTuple>> {
  return await Promise.all(
    selection.questions.map(async (selected) => {
      const questionId = canonicalQuestionId(selected.questionId);
      const tuple = selected.row.publishedQuestionRevisionTuple;
      if (
        tuple.publishedQuestionId !== questionId ||
        !Number.isSafeInteger(tuple.revisionNumber) ||
        tuple.revisionNumber < 1
      ) {
        throw new Error("The selected Question Revision is no longer valid.");
      }
      const [detail, metadata] = await Promise.all([
        getQuestionRevision(tuple),
        getCurrentQuestionSharedMetadata([questionId]),
      ]);
      const current = metadata[0];
      if (
        current === undefined ||
        current.questionId !== questionId ||
        detail.summary.publishedQuestionRevisionTuple.publishedQuestionId !== questionId ||
        detail.summary.publishedQuestionRevisionTuple.revisionNumber !== tuple.revisionNumber ||
        !sameEligibility(current, detail, eligibility)
      ) {
        throw new Error(
          "Every Pool Question must share the Pool's Discipline, Subject, Type, and Backend.",
        );
      }
      return tuple;
    }),
  );
}

/** Pins every exact starting Revision first and rechecks every added Question's eligibility. */
export async function questionPoolMemberTuples(
  selection: QuestionPickerSelection,
  getQuestionRevision: QuestionAvailabilityClient["getQuestionRevision"],
  getCurrentQuestionSharedMetadata: QuestionBulkMetadataClient["getCurrentQuestionSharedMetadata"],
  eligibility: QuestionPickerEligibility,
  startingQuestions: ReadonlyArray<QuestionPoolStartingQuestion> = [],
): Promise<ReadonlyArray<PublishedQuestionRevisionTuple>> {
  const additional = await exactSelectedRevisions(
    selection,
    getQuestionRevision,
    getCurrentQuestionSharedMetadata,
    eligibility,
  );
  const fixed = startingQuestions.map(exactStartingRevision);
  const allIds = [...fixed, ...additional].map((member) => member.publishedQuestionId);
  if (new Set(allIds).size !== allIds.length) {
    throw new QuestionPoolSelectionError("A Question may appear only once in this Pool.");
  }
  return [...fixed, ...additional];
}
