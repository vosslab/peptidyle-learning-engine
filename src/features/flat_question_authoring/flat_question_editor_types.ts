// Public composition contract for the flat-question editor surface.

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { ApiClient } from "../../api/client";
import type { WasmFacade } from "../../wasm/index";
import type { FlatQuestionAssetClient } from "./flat_question_asset_client";
import type { FlatQuestionRead } from "./flat_question_client";
import type { FlatQuestionRepository } from "./flat_question_repository";

export interface FlatQuestionEditorPageProps {
  readonly workspace: WorkspaceId;
  readonly initial: FlatQuestionRead;
  readonly repository: FlatQuestionRepository;
  /** The ordinary browser client supplies only answer-free publication review data. */
  readonly api: Pick<ApiClient, "validateWorkspacePublication" | "getWorkspacePublicationDiff">;
  /** Injected browser-safe validator keeps preview on the same student QuestionResponseControl path. */
  readonly responseValidator: Pick<WasmFacade, "validateResponseFormat">;
  /** Protected image metadata client; absent only in a deliberately limited embedded fixture. */
  readonly assetClient?: FlatQuestionAssetClient;
  /** Same-route QTI conversion may move focus into the newly replaced draft. */
  readonly focusHeadingOnMount?: boolean;
  /** Clears the route's one-shot focus request after the unlocked heading receives it. */
  readonly onHeadingFocusDelivered?: () => void;
  /** Reports the exact saved revision and whether this editor has local changes. */
  readonly onDraftDisplayStateChange?: (state: FlatQuestionDraftDisplayState) => void;
  /** Prevents edits while QTI conversion is replacing and refetching this draft. */
  readonly replacementPending?: boolean;
}

export interface FlatQuestionDraftDisplayState {
  readonly revision: string;
  readonly dirty: boolean;
}
