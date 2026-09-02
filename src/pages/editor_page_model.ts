// editor_page_model.ts - browser-safe contracts for the workspace editor surface.

import type { Capability } from "../../generated/api/Capability";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { DraftQuestionBackendLocator } from "../../generated/api/DraftQuestionBackendLocator";
import type { QuestionVariationRule } from "../../generated/api/QuestionVariationRule";
import type { QuestionResponseFormat } from "../../generated/api/QuestionResponseFormat";
import type { QuestionSeed } from "../../generated/api/QuestionSeed";
import type { QuestionAttemptTimeLimit } from "../../generated/api/QuestionAttemptTimeLimit";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { DraftQuestionSummary } from "../../generated/api/DraftQuestionSummary";
import type { QuestionAttemptLimit } from "../../generated/api/QuestionAttemptLimit";
import type { QuestionAuthorship } from "../../generated/api/QuestionAuthorship";
import type { InstructorPreviewResult } from "./editor_instructor_preview";

export type { DraftQuestionSummary };

/**
 * The editor's deliberate browser-safe Editor Draft.
 *
 * It retains workspace ownership, but has neither a durable published identity nor any
 * server-only evaluation material. The live workspace API will construct this Editor Draft.
 */
export interface EditorDraft {
  readonly workspace: WorkspaceId;
  readonly title: string;
  readonly backendLocator: DraftQuestionBackendLocator;
  readonly prompt: ReadonlyArray<QuestionContentBlock>;
  readonly response: QuestionResponseFormat;
  readonly questionAttemptLimit: QuestionAttemptLimit;
  readonly questionAttemptTimeLimit: QuestionAttemptTimeLimit;
  readonly questionVariationRule: QuestionVariationRule;
}

export interface EditorDraftDisplayState {
  readonly revision: string;
  readonly dirty: boolean;
}

export interface DraftQuestionPage {
  readonly items: ReadonlyArray<DraftQuestionSummary>;
  readonly nextCursor: string | null;
}

/** A key-free, unversioned offline preview result. */
export interface EditorPreview {
  readonly workspace: WorkspaceId;
  readonly seed: QuestionSeed;
  readonly title: string;
  readonly prompt: ReadonlyArray<QuestionContentBlock>;
  readonly response: QuestionResponseFormat;
}

export interface DraftCapabilityViolation {
  readonly workspace: WorkspaceId;
  readonly title: string;
  readonly capability: Capability;
}

export interface QuestionPublicationReviewSection {
  readonly label: string;
  readonly before: string | null;
  readonly after: string;
}

/** A server-computed, browser-safe review of one saved Draft Question Revision. */
export interface QuestionPublicationReview {
  /** Exact strong ETag returned with the server-computed review. */
  readonly revision: string;
  /** Every review proposes a distinct immutable Question ID. */
  readonly baseQuestion: "newQuestion";
  readonly proposedTitle: string;
  readonly sections: ReadonlyArray<QuestionPublicationReviewSection>;
}

/**
 * InstructorPreviewProvider supplies Instructor Preview while preserving the repository's private
 * strong revision ownership.
 */
export interface InstructorPreviewProvider {
  readonly requestPresentation: (
    draft: EditorDraft,
    seed: QuestionSeed,
  ) => Promise<InstructorPreviewResult>;
}

export type PublishOutcome =
  | { readonly kind: "published"; readonly questionId: string }
  | {
      readonly kind: "validationFailed";
      readonly violations: ReadonlyArray<DraftCapabilityViolation>;
    }
  | { readonly kind: "error"; readonly message: string };

/** Injected workspace boundary supplied by the active runtime composition. */
export interface EditorRepository {
  readonly listDrafts: (cursor?: string) => Promise<DraftQuestionPage>;
  readonly getDraft: (workspace: WorkspaceId) => Promise<EditorDraft>;
  readonly saveDraft: (draft: EditorDraft) => Promise<EditorDraft>;
  readonly validateCapabilities: (
    draft: EditorDraft,
    required: ReadonlyArray<Capability>,
  ) => Promise<ReadonlyArray<DraftCapabilityViolation>>;
  readonly getQuestionPublicationReview: (draft: EditorDraft) => Promise<QuestionPublicationReview>;
  /** Publishes only the exact revision represented by a previously reviewed server result. */
  readonly publish: (
    draft: EditorDraft,
    request: { readonly authorship: QuestionAuthorship },
    reviewedRevision: string,
  ) => Promise<PublishOutcome>;
  /** Only completed server contracts are enabled by a live repository. */
  readonly capabilities?: {
    readonly assignmentValidation: boolean;
    readonly publication: boolean;
    readonly instructorPreview: boolean;
  };
  readonly deleteDraft?: (workspace: WorkspaceId) => Promise<void>;
  readonly reloadDraft?: (workspace: WorkspaceId) => Promise<EditorDraft>;
  /** Strong revision retained for the draft currently displayed by an editor. */
  readonly displayedRevision?: (workspace: WorkspaceId) => string | null;
  readonly instructorPreview?: InstructorPreviewProvider;
}

/** Injected WASM-shaped key-free boundary. It must not call a network service. */
export interface PreviewFacade {
  readonly preview: (draft: EditorDraft, seed: QuestionSeed) => Promise<EditorPreview>;
}

export function serializeEditorState(draft: EditorDraft, preview: EditorPreview | null): string {
  return JSON.stringify({ draft, preview });
}

export function capabilityLabel(capability: Capability): string {
  return capability.replace(/([A-Z])/g, " $1").toLowerCase();
}
