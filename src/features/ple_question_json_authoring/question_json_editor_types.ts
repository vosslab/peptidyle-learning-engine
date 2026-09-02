// Public composition contract for the ple-question-json editor surface.

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { ApiClient } from "../../api/client";
import type { WasmFacade } from "../../wasm/index";
import type { PleQuestionJsonRead } from "./question_json_client";
import type { PleQuestionJsonRepository } from "./question_json_repository";
import type {
  PleQuestionJsonMatchingChoice,
  PleQuestionJsonMatchingPrompt,
  PleQuestionJsonOrderingItem,
} from "./question_json_source";

type Assert<T extends true> = T;

/** Private source roles share JSON members but stay non-assignable while authoring. */
type PleQuestionJsonResponseMemberRolesAreDistinct = Assert<
  [PleQuestionJsonMatchingPrompt] extends [PleQuestionJsonMatchingChoice]
    ? false
    : [PleQuestionJsonMatchingChoice] extends [PleQuestionJsonMatchingPrompt]
      ? false
      : [PleQuestionJsonMatchingPrompt] extends [PleQuestionJsonOrderingItem]
        ? false
        : [PleQuestionJsonOrderingItem] extends [PleQuestionJsonMatchingPrompt]
          ? false
          : [PleQuestionJsonMatchingChoice] extends [PleQuestionJsonOrderingItem]
            ? false
            : [PleQuestionJsonOrderingItem] extends [PleQuestionJsonMatchingChoice]
              ? false
              : true
>;

void (undefined as unknown as PleQuestionJsonResponseMemberRolesAreDistinct);

export interface PleQuestionJsonEditorPageProps {
  readonly workspace: WorkspaceId;
  readonly initial: PleQuestionJsonRead;
  readonly repository: PleQuestionJsonRepository;
  /** The ordinary browser client supplies only answer-free publication review data. */
  readonly api: Pick<ApiClient, "validateWorkspacePublication" | "getQuestionPublicationReview">;
  /** Injected browser-safe validator keeps preview on the same student QuestionResponseControl path. */
  readonly responseValidator: Pick<WasmFacade, "validateResponseFormat">;
  /** Same-route QTI conversion may move focus into the newly replaced draft. */
  readonly focusHeadingOnMount?: boolean;
  /** Clears the route's one-shot focus request after the unlocked heading receives it. */
  readonly onHeadingFocusDelivered?: () => void;
  /** Reports the exact saved revision and whether this editor has local changes. */
  readonly onDraftDisplayStateChange?: (state: PleQuestionJsonDraftDisplayState) => void;
  /** Prevents edits while QTI conversion is replacing and refetching this draft. */
  readonly replacementPending?: boolean;
}

export interface PleQuestionJsonDraftDisplayState {
  readonly revision: string;
  readonly dirty: boolean;
}
