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
    revision?: string,
  ): Promise<PleQuestionJsonSave>;
  publish(
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
    revision: string,
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
  synchronizeRevision(draftQuestion: DraftQuestionRouteId, revision: string): void;
}

/** A stale save keeps the caller's private source available for a deliberate merge or reload. */
export class PleQuestionJsonStaleConflictError extends PleQuestionJsonConflictError {
  public readonly source: PleQuestionJsonDocument;

  public constructor(cause: PleQuestionJsonConflictError, source: PleQuestionJsonDocument) {
    super(cause.status, cause.path);
    this.source = source;
  }
}

/** Owns only the server revision; editor state remains with the calling UI. */
export function createPleQuestionJsonRepository(
  client: PleQuestionJsonAuthoringClient,
): PleQuestionJsonRepository {
  const revisions = new Map<DraftQuestionRouteId, string>();
  const operationGenerations = new Map<DraftQuestionRouteId, number>();

  function startOperation(draftQuestion: DraftQuestionRouteId): number {
    const generation = (operationGenerations.get(draftQuestion) ?? 0) + 1;
    operationGenerations.set(draftQuestion, generation);
    return generation;
  }

  function setRevisionIfCurrent(
    draftQuestion: DraftQuestionRouteId,
    generation: number,
    revision: string,
  ): void {
    if (operationGenerations.get(draftQuestion) === generation)
      revisions.set(draftQuestion, revision);
  }

  async function load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead> {
    const generation = startOperation(draftQuestion);
    const result = await client.load(draftQuestion);
    setRevisionIfCurrent(draftQuestion, generation, result.revision);
    return result;
  }

  async function save(
    draftQuestion: DraftQuestionRouteId,
    source: PleQuestionJsonDocument,
  ): Promise<PleQuestionJsonSave> {
    const generation = startOperation(draftQuestion);
    const revision = revisions.get(draftQuestion);
    try {
      const result = await client.save(draftQuestion, source, revision);
      setRevisionIfCurrent(draftQuestion, generation, result.revision);
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
    const revision = revisions.get(draftQuestion);
    if (revision === undefined) {
      throw new Error("Load the saved Question before publishing it.");
    }
    return await client.publish(draftQuestion, request, revision);
  }

  function synchronizeRevision(draftQuestion: DraftQuestionRouteId, revision: string): void {
    revisions.set(draftQuestion, revision);
  }

  return { load, save, reload, publish, synchronizeRevision };
}
