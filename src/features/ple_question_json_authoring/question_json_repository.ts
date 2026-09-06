import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionAuthorship } from "../../../generated/api/QuestionAuthorship";
import type { DraftQuestionReference } from "../../../generated/api/DraftQuestionReference";
import {
  PleQuestionJsonConflictError,
  type PleQuestionJsonRead,
  type PleQuestionJsonSave,
} from "./question_json_client";
import type { PleQuestionJsonDocument } from "./question_json_source";

export interface PleQuestionJsonAuthoringClient {
  load(draftQuestion: DraftQuestionReference): Promise<PleQuestionJsonRead>;
  save(
    draftQuestion: DraftQuestionReference,
    source: PleQuestionJsonDocument,
    revision?: string,
  ): Promise<PleQuestionJsonSave>;
  publish(
    draftQuestion: DraftQuestionReference,
    request: { readonly authorship: QuestionAuthorship },
    revision: string,
  ): Promise<QuestionSummary>;
}

export interface PleQuestionJsonRepository {
  load(draftQuestion: DraftQuestionReference): Promise<PleQuestionJsonRead>;
  save(
    draftQuestion: DraftQuestionReference,
    source: PleQuestionJsonDocument,
  ): Promise<PleQuestionJsonSave>;
  reload(draftQuestion: DraftQuestionReference): Promise<PleQuestionJsonRead>;
  publish(
    draftQuestion: DraftQuestionReference,
    request: { readonly authorship: QuestionAuthorship },
  ): Promise<QuestionSummary>;
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
  const revisions = new Map<DraftQuestionReference, string>();
  const operationGenerations = new Map<DraftQuestionReference, number>();

  function startOperation(draftQuestion: DraftQuestionReference): number {
    const generation = (operationGenerations.get(draftQuestion) ?? 0) + 1;
    operationGenerations.set(draftQuestion, generation);
    return generation;
  }

  function setRevisionIfCurrent(
    draftQuestion: DraftQuestionReference,
    generation: number,
    revision: string,
  ): void {
    if (operationGenerations.get(draftQuestion) === generation)
      revisions.set(draftQuestion, revision);
  }

  async function load(draftQuestion: DraftQuestionReference): Promise<PleQuestionJsonRead> {
    const generation = startOperation(draftQuestion);
    const result = await client.load(draftQuestion);
    setRevisionIfCurrent(draftQuestion, generation, result.revision);
    return result;
  }

  async function save(
    draftQuestion: DraftQuestionReference,
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

  async function reload(draftQuestion: DraftQuestionReference): Promise<PleQuestionJsonRead> {
    return await load(draftQuestion);
  }

  async function publish(
    draftQuestion: DraftQuestionReference,
    request: { readonly authorship: QuestionAuthorship },
  ): Promise<QuestionSummary> {
    const revision = revisions.get(draftQuestion);
    if (revision === undefined) {
      throw new Error("Load the saved Question before publishing it.");
    }
    return await client.publish(draftQuestion, request, revision);
  }

  return { load, save, reload, publish };
}
