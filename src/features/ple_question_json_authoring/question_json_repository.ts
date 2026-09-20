import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionAuthorship } from "../../../generated/api/QuestionAuthorship";
import type { DraftQuestionRouteId } from "../../navigation/public_route";
import {
  PleQuestionJsonConflictError,
  type PleQuestionJsonRead,
  type PleQuestionJsonSave,
} from "./question_json_client";
import type { PleQuestionJsonDocument } from "./question_json_source";

export type PleQuestionJsonPublicationRequest = {
  readonly authorship: QuestionAuthorship;
  readonly disciplineUuid: string;
  readonly subjectUuid: string;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
};

export interface PleQuestionJsonAuthoringClient {
  load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead>;
  save(
    draftQuestion: DraftQuestionRouteId,
    source: PleQuestionJsonDocument,
    expectedDraftQuestionEditNumber?: string,
  ): Promise<PleQuestionJsonSave>;
  publish(
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
    expectedDraftQuestionEditNumber: string,
  ): Promise<QuestionSummary>;
}

export interface PleQuestionJsonRepository {
  load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead>;
  save(
    draftQuestion: DraftQuestionRouteId,
    source: PleQuestionJsonDocument,
  ): Promise<PleQuestionJsonSave>;
  reload(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead>;
  publish(
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
  ): Promise<QuestionSummary>;
  /** A separately saved Draft metadata field advances the same server edit number. */
  synchronizeDraftQuestionEditNumber(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
  ): void;
}

/** A stale save keeps the caller's private source available for a deliberate merge or reload. */
export class PleQuestionJsonStaleConflictError extends PleQuestionJsonConflictError {
  public readonly source: PleQuestionJsonDocument;

  public constructor(cause: PleQuestionJsonConflictError, source: PleQuestionJsonDocument) {
    super(cause.status, cause.path);
    this.source = source;
  }
}

/** Owns only the Draft Question Edit Number; editor state remains with the calling UI. */
export function createPleQuestionJsonRepository(
  client: PleQuestionJsonAuthoringClient,
): PleQuestionJsonRepository {
  const editNumbers = new Map<DraftQuestionRouteId, string>();
  const operationGenerations = new Map<DraftQuestionRouteId, number>();

  function startOperation(draftQuestion: DraftQuestionRouteId): number {
    const generation = (operationGenerations.get(draftQuestion) ?? 0) + 1;
    operationGenerations.set(draftQuestion, generation);
    return generation;
  }

  function setEditNumberIfCurrent(
    draftQuestion: DraftQuestionRouteId,
    generation: number,
    draftQuestionEditNumber: string,
  ): void {
    if (operationGenerations.get(draftQuestion) === generation)
      editNumbers.set(draftQuestion, draftQuestionEditNumber);
  }

  async function load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead> {
    const generation = startOperation(draftQuestion);
    const result = await client.load(draftQuestion);
    setEditNumberIfCurrent(draftQuestion, generation, result.draftQuestionEditNumber);
    return result;
  }

  async function save(
    draftQuestion: DraftQuestionRouteId,
    source: PleQuestionJsonDocument,
  ): Promise<PleQuestionJsonSave> {
    const generation = startOperation(draftQuestion);
    const expectedDraftQuestionEditNumber = editNumbers.get(draftQuestion);
    try {
      const result = await client.save(draftQuestion, source, expectedDraftQuestionEditNumber);
      setEditNumberIfCurrent(draftQuestion, generation, result.draftQuestionEditNumber);
      return result;
    } catch (error: unknown) {
      if (error instanceof PleQuestionJsonConflictError) {
        throw new PleQuestionJsonStaleConflictError(error, source);
      }
      throw error;
    }
  }

  async function reload(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead> {
    return await load(draftQuestion);
  }

  async function publish(
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
  ): Promise<QuestionSummary> {
    const expectedDraftQuestionEditNumber = editNumbers.get(draftQuestion);
    if (expectedDraftQuestionEditNumber === undefined) {
      throw new Error("Load the saved Question before publishing it.");
    }
    return await client.publish(draftQuestion, request, expectedDraftQuestionEditNumber);
  }

  function synchronizeDraftQuestionEditNumber(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
  ): void {
    editNumbers.set(draftQuestion, draftQuestionEditNumber);
  }

  return { load, save, reload, publish, synchronizeDraftQuestionEditNumber };
}
