import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionAuthorship } from "../../../generated/api/QuestionAuthorship";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import {
  PleQuestionJsonConflictError,
  type PleQuestionJsonRead,
  type PleQuestionJsonSave,
} from "./question_json_client";
import type { PleQuestionJsonDocument } from "./question_json_source";

export interface PleQuestionJsonAuthoringClient {
  load(workspace: WorkspaceId): Promise<PleQuestionJsonRead>;
  save(
    workspace: WorkspaceId,
    source: PleQuestionJsonDocument,
    revision?: string,
  ): Promise<PleQuestionJsonSave>;
  publish(
    workspace: WorkspaceId,
    request: { readonly authorship: QuestionAuthorship },
    revision: string,
  ): Promise<QuestionSummary>;
}

export interface PleQuestionJsonRepository {
  load(workspace: WorkspaceId): Promise<PleQuestionJsonRead>;
  save(workspace: WorkspaceId, source: PleQuestionJsonDocument): Promise<PleQuestionJsonSave>;
  reload(workspace: WorkspaceId): Promise<PleQuestionJsonRead>;
  publish(
    workspace: WorkspaceId,
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
  const revisions = new Map<WorkspaceId, string>();
  const operationGenerations = new Map<WorkspaceId, number>();

  function startOperation(workspace: WorkspaceId): number {
    const generation = (operationGenerations.get(workspace) ?? 0) + 1;
    operationGenerations.set(workspace, generation);
    return generation;
  }

  function setRevisionIfCurrent(
    workspace: WorkspaceId,
    generation: number,
    revision: string,
  ): void {
    if (operationGenerations.get(workspace) === generation) revisions.set(workspace, revision);
  }

  async function load(workspace: WorkspaceId): Promise<PleQuestionJsonRead> {
    const generation = startOperation(workspace);
    const result = await client.load(workspace);
    setRevisionIfCurrent(workspace, generation, result.revision);
    return result;
  }

  async function save(
    workspace: WorkspaceId,
    source: PleQuestionJsonDocument,
  ): Promise<PleQuestionJsonSave> {
    const generation = startOperation(workspace);
    const revision = revisions.get(workspace);
    try {
      const result = await client.save(workspace, source, revision);
      setRevisionIfCurrent(workspace, generation, result.revision);
      return result;
    } catch (error: unknown) {
      if (error instanceof PleQuestionJsonConflictError) {
        throw new PleQuestionJsonStaleConflictError(error, source);
      }
      throw error;
    }
  }

  async function reload(workspace: WorkspaceId): Promise<PleQuestionJsonRead> {
    return await load(workspace);
  }

  async function publish(
    workspace: WorkspaceId,
    request: { readonly authorship: QuestionAuthorship },
  ): Promise<QuestionSummary> {
    const revision = revisions.get(workspace);
    if (revision === undefined) {
      throw new Error("Load the saved Question before publishing it.");
    }
    return await client.publish(workspace, request, revision);
  }

  return { load, save, reload, publish };
}
