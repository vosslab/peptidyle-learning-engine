// Browser adapter from the shared WASM boundary to the editor's narrow preview contract.

import type { WasmFacade } from "../wasm/index";
import type { EditorPreview, PreviewFacade } from "./editor_page_model";

/** Uses the actual key-free WASM bridge; no local question generator exists in the editor. */
export function createEditorPreviewFacade(wasm: WasmFacade): PreviewFacade {
  return {
    preview: async (draft): Promise<EditorPreview> => {
      const result = await wasm.previewPleDraft({
        workspace: draft.workspace,
        questionBackend: draft.questionBackend,
        webworkPgPath: draft.webworkPgPath,
        qtiPackageItemIdentifier: draft.qtiPackageItemIdentifier,
        workspaceImportId: draft.workspaceImportId,
        draftImathasQuestionBackendBinding: draft.draftImathasQuestionBackendBinding,
        title: draft.title,
        prompt: draft.prompt,
        response: draft.response,
      });
      if (result.kind === "unavailable") {
        throw new Error(
          `${result.backend} drafts need a backend preview; offline preview is unavailable.`,
        );
      }
      return result.preview;
    },
  };
}
