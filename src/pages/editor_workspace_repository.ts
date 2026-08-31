// editor_workspace_repository.ts - live CRUD adapter for private, unversioned workspace drafts.

import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { ApiClient } from "../api/client";
import { PublicationValidationError, WorkspaceConflictError } from "../api/http_client";
import type {
  EditorDraft,
  EditorRepository,
  DraftCapabilityViolation,
  PublishOutcome,
  PublishVersionDiff,
  WorkspaceDraftPage,
} from "./editor_page_model";
import type {
  InstructorPreviewBoundary,
  InstructorPreviewResult,
} from "./editor_instructor_preview";

interface LoadedDraft {
  readonly definition: DraftQuestionDefinition;
  readonly revision: string;
}

function editorDraft(definition: DraftQuestionDefinition): EditorDraft {
  return {
    workspace: definition.workspace,
    title: definition.metadata.title,
    source: definition.source,
    prompt: definition.prompt,
    response: definition.response,
    questionAttemptLimit: definition.questionAttemptLimit,
    questionAttemptTimeLimit: definition.questionAttemptTimeLimit,
    randomization: definition.randomization,
  };
}

function updateDefinition(
  existing: DraftQuestionDefinition,
  draft: EditorDraft,
): DraftQuestionDefinition {
  if (existing.workspace !== draft.workspace) {
    throw new Error("Workspace identity cannot change while editing a draft");
  }
  return {
    ...existing,
    source: draft.source,
    prompt: [...draft.prompt],
    response: draft.response,
    questionAttemptLimit: draft.questionAttemptLimit,
    questionAttemptTimeLimit: draft.questionAttemptTimeLimit,
    randomization: draft.randomization,
    metadata: { ...existing.metadata, title: draft.title },
  };
}

/**
 * Retains the transport-only revision and complete draft record privately inside this adapter.
 * Editor state receives only the identity-safe projection, while saving faithfully round-trips
 * fields this compact UI does not edit.
 */
export function createWorkspaceEditorRepository(
  client: ApiClient,
  instructorPreview?: InstructorPreviewBoundary,
): EditorRepository {
  const loaded = new Map<WorkspaceId, LoadedDraft>();

  async function loadedDraft(workspace: WorkspaceId): Promise<LoadedDraft> {
    const current = loaded.get(workspace);
    if (current !== undefined) return current;
    const detail = await client.getWorkspaceDraft(workspace);
    const retrieved = { definition: detail.draft, revision: detail.revision };
    loaded.set(workspace, retrieved);
    return retrieved;
  }

  async function get(workspace: WorkspaceId): Promise<EditorDraft> {
    const detail = await client.getWorkspaceDraft(workspace);
    if (detail.draft.workspace !== workspace) {
      throw new Error("Workspace detail identity does not match the requested workspace");
    }
    loaded.set(workspace, { definition: detail.draft, revision: detail.revision });
    return editorDraft(detail.draft);
  }

  return {
    listDrafts: async (cursor?: string): Promise<WorkspaceDraftPage> =>
      await client.listWorkspaceDrafts(cursor),
    getDraft: get,
    reloadDraft: get,
    displayedRevision: (workspace): string | null => loaded.get(workspace)?.revision ?? null,
    saveDraft: async (draft: EditorDraft): Promise<EditorDraft> => {
      const current = await loadedDraft(draft.workspace);
      const definition = updateDefinition(current.definition, draft);
      try {
        const saved = await client.saveWorkspaceDraft(
          draft.workspace,
          definition,
          current.revision,
        );
        loaded.set(draft.workspace, { definition: saved.draft, revision: saved.revision });
        return editorDraft(saved.draft);
      } catch (error: unknown) {
        if (error instanceof WorkspaceConflictError) throw error;
        throw error;
      }
    },
    deleteDraft: async (workspace): Promise<void> => {
      const current = await loadedDraft(workspace);
      await client.deleteWorkspaceDraft(workspace, current.revision);
      loaded.delete(workspace);
    },
    validateCapabilities: async (
      draft,
      _required,
    ): Promise<ReadonlyArray<DraftCapabilityViolation>> => {
      const validation = await client.validateWorkspacePublication(draft.workspace);
      if (validation.kind === "readinessFailure") {
        throw new Error(validation.message);
      }
      return validation.violations;
    },
    getPublishDiff: async (draft): Promise<PublishVersionDiff> => {
      const diff = await client.getWorkspacePublicationDiff(draft.workspace);
      return {
        revision: diff.revision,
        baseline: diff.baseline,
        proposedTitle: diff.current.title,
        sections: diff.changed.map((label) => ({
          label,
          before: null,
          after: "Current saved draft",
        })),
      };
    },
    publish: async (draft, request, reviewedRevision): Promise<PublishOutcome> => {
      try {
        const current = loaded.get(draft.workspace);
        if (current === undefined) {
          throw new Error("Review the saved workspace draft before publishing.");
        }
        if (current.revision !== reviewedRevision) {
          throw new WorkspaceConflictError(409, `/api/problems/${draft.workspace}/publish`);
        }
        const result = await client.publishWorkspace(draft.workspace, request, reviewedRevision);
        return { kind: "published", questionId: result.summary.questionId };
      } catch (error: unknown) {
        if (error instanceof PublicationValidationError) {
          return { kind: "validationFailed", violations: error.violations };
        }
        throw error;
      }
    },
    capabilities: {
      assignmentValidation: true,
      publication: true,
      instructorPreview: instructorPreview !== undefined,
    },
    ...(instructorPreview === undefined
      ? {}
      : {
          instructorPreview: {
            requestPresentation: async (draft, seed): Promise<InstructorPreviewResult> => {
              const current = await loadedDraft(draft.workspace);
              const result = await instructorPreview.requestPresentation(
                draft.workspace,
                seed,
                current.revision,
              );
              if (result.revision !== current.revision) {
                throw new Error(
                  "Instructor preview revision does not match the saved workspace draft",
                );
              }
              return result;
            },
          },
        }),
  };
}
