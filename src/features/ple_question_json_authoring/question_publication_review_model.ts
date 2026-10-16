import type { QuestionType } from "../../../generated/api/QuestionType";

export type PublicationReviewSnapshot = {
  readonly authorshipText: string;
  readonly selfAuthorPosition: number | null;
  readonly disciplineUuid: string | null;
  readonly subjectUuid: string | null;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
};

/** Review values are local until publication; saving the Draft does not save them. */
export function isPublicationReviewDirty(
  current: PublicationReviewSnapshot,
  baseline: PublicationReviewSnapshot,
): boolean {
  return (
    current.authorshipText !== baseline.authorshipText ||
    current.selfAuthorPosition !== baseline.selfAuthorPosition ||
    current.disciplineUuid !== baseline.disciplineUuid ||
    current.subjectUuid !== baseline.subjectUuid ||
    current.topicUuid !== baseline.topicUuid ||
    current.subtopicUuid !== baseline.subtopicUuid
  );
}

export type DraftQuestionMutationQueue = <T>(operation: () => Promise<T>) => Promise<T>;

/** Serializes source and metadata CAS writes so each uses the preceding acknowledged Edit Number. */
export function createDraftQuestionMutationQueue(): DraftQuestionMutationQueue {
  let mutationTail: Promise<void> = Promise.resolve();
  return <T>(operation: () => Promise<T>): Promise<T> => {
    const result = mutationTail.then(operation);
    mutationTail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  };
}

/** Returns true only when source and metadata snapshots have both been acknowledged. */
export function isDraftSnapshotSaved(
  sourceAcknowledged: boolean,
  metadataDirty: boolean,
  mutationInFlight: boolean,
): boolean {
  return sourceAcknowledged && !metadataDirty && !mutationInFlight;
}

/** WebWork publication requires a saved manually selected Question Type. */
export function canPublishWebworkDraft(
  isSaved: boolean,
  questionType: QuestionType | null,
): boolean {
  return isSaved && questionType !== null;
}
