import type { CatalogQuestionSummary } from "../../../generated/api/CatalogQuestionSummary";
import type { PublicByline } from "../../../generated/api/PublicByline";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import {
  FlatQuestionConflictError,
  type FlatQuestionRead,
  type FlatQuestionSave,
} from "./flat_question_client";
import type { FlatQuestionSourceV2 } from "./flat_question_source";

export interface FlatQuestionAuthoringClient {
  load(workspace: WorkspaceId): Promise<FlatQuestionRead>;
  save(
    workspace: WorkspaceId,
    source: FlatQuestionSourceV2,
    revision?: string,
  ): Promise<FlatQuestionSave>;
  publish(
    workspace: WorkspaceId,
    request: { readonly byline: PublicByline },
    revision: string,
  ): Promise<CatalogQuestionSummary>;
}

export interface FlatQuestionRepository {
  load(workspace: WorkspaceId): Promise<FlatQuestionRead>;
  save(workspace: WorkspaceId, source: FlatQuestionSourceV2): Promise<FlatQuestionSave>;
  reload(workspace: WorkspaceId): Promise<FlatQuestionRead>;
  publish(
    workspace: WorkspaceId,
    request: { readonly byline: PublicByline },
  ): Promise<CatalogQuestionSummary>;
}

/** A stale save keeps the caller's private source available for a deliberate merge or reload. */
export class FlatQuestionStaleConflictError extends FlatQuestionConflictError {
  public readonly source: FlatQuestionSourceV2;

  public constructor(cause: FlatQuestionConflictError, source: FlatQuestionSourceV2) {
    super(cause.status, cause.path);
    this.source = source;
  }
}

/** Owns only the server revision; editor state remains with the calling UI. */
export function createFlatQuestionRepository(
  client: FlatQuestionAuthoringClient,
): FlatQuestionRepository {
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

  async function load(workspace: WorkspaceId): Promise<FlatQuestionRead> {
    const generation = startOperation(workspace);
    const result = await client.load(workspace);
    setRevisionIfCurrent(workspace, generation, result.revision);
    return result;
  }

  async function save(
    workspace: WorkspaceId,
    source: FlatQuestionSourceV2,
  ): Promise<FlatQuestionSave> {
    const generation = startOperation(workspace);
    const revision = revisions.get(workspace);
    try {
      const result = await client.save(workspace, source, revision);
      setRevisionIfCurrent(workspace, generation, result.revision);
      return result;
    } catch (error: unknown) {
      if (error instanceof FlatQuestionConflictError) {
        throw new FlatQuestionStaleConflictError(error, source);
      }
      throw error;
    }
  }

  async function reload(workspace: WorkspaceId): Promise<FlatQuestionRead> {
    return await load(workspace);
  }

  async function publish(
    workspace: WorkspaceId,
    request: { readonly byline: PublicByline },
  ): Promise<CatalogQuestionSummary> {
    const revision = revisions.get(workspace);
    if (revision === undefined) {
      throw new Error("Load the saved flat question before publishing it.");
    }
    return await client.publish(workspace, request, revision);
  }

  return { load, save, reload, publish };
}
