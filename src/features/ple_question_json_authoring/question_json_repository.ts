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
    etag?: string,
  ): Promise<PleQuestionJsonSave>;
  publish(
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
    etag: string,
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
  synchronizeEtag(draftQuestion: DraftQuestionRouteId, etag: string): void;
}

/** A stale save keeps the caller's private source available for a deliberate merge or reload. */
export class PleQuestionJsonStaleConflictError extends PleQuestionJsonConflictError {
  public readonly source: PleQuestionJsonDocument;

  public constructor(cause: PleQuestionJsonConflictError, source: PleQuestionJsonDocument) {
    super(cause.status, cause.path);
    this.source = source;
  }
}

/** Owns only the server ETag; editor state remains with the calling UI. */
export function createPleQuestionJsonRepository(
  client: PleQuestionJsonAuthoringClient,
): PleQuestionJsonRepository {
  const etags = new Map<DraftQuestionRouteId, string>();
  const operationGenerations = new Map<DraftQuestionRouteId, number>();

  function startOperation(draftQuestion: DraftQuestionRouteId): number {
    const generation = (operationGenerations.get(draftQuestion) ?? 0) + 1;
    operationGenerations.set(draftQuestion, generation);
    return generation;
  }

  function setEtagIfCurrent(
    draftQuestion: DraftQuestionRouteId,
    generation: number,
    etag: string,
  ): void {
    if (operationGenerations.get(draftQuestion) === generation) etags.set(draftQuestion, etag);
  }

  async function load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead> {
    const generation = startOperation(draftQuestion);
    const result = await client.load(draftQuestion);
    setEtagIfCurrent(draftQuestion, generation, result.etag);
    return result;
  }

  async function save(
    draftQuestion: DraftQuestionRouteId,
    source: PleQuestionJsonDocument,
  ): Promise<PleQuestionJsonSave> {
    const generation = startOperation(draftQuestion);
    const etag = etags.get(draftQuestion);
    try {
      const result = await client.save(draftQuestion, source, etag);
      setEtagIfCurrent(draftQuestion, generation, result.etag);
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
    const etag = etags.get(draftQuestion);
    if (etag === undefined) {
      throw new Error("Load the saved Question before publishing it.");
    }
    return await client.publish(draftQuestion, request, etag);
  }

  function synchronizeEtag(draftQuestion: DraftQuestionRouteId, etag: string): void {
    etags.set(draftQuestion, etag);
  }

  return { load, save, reload, publish, synchronizeEtag };
}
