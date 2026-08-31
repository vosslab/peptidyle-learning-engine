// editor_live_pages.tsx - route composition for accepted workspace CRUD only.

import { useLocation, useNavigate, useParams } from "@solidjs/router";
import { Show, createResource, createSignal, type JSX } from "solid-js";

import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import { useApplicationApi } from "../api/application_api";
import { useWasmFacade } from "../wasm/context";
import {
  createFlatQuestionClient,
  FlatQuestionRequestError,
  type FlatQuestionRead,
} from "../features/flat_question_authoring/flat_question_client";
import { createDefaultFlatQuestionSource } from "../features/flat_question_authoring/flat_question_defaults";
import { createFlatQuestionAssetClient } from "../features/flat_question_authoring/flat_question_asset_client";
import { FlatQuestionEditorPage } from "../features/flat_question_authoring/flat_question_editor_page";
import { createFlatQuestionRepository } from "../features/flat_question_authoring/flat_question_repository";
import { createQtiProfileImportClient } from "../features/qti_profile_import/qti_profile_import_client";
import type { QtiConversionDraftState } from "../features/qti_profile_import/qti_profile_import_model";
import { QtiProfileImportPage } from "../features/qti_profile_import/qti_profile_import_page";
import { WasmEditorPage } from "./editor_page";
import { createInstructorPreviewClient } from "./editor_instructor_preview";
import { createWorkspaceEditorRepository } from "./editor_workspace_repository";
import { authoringWorkspaceRouteReference } from "../navigation/public_route";
import { resolveWorkspaceRoute } from "../navigation/resolved_route";

/** `/workspace`: a server-backed private draft list with its first draft selected. */
export function WorkspaceListLivePage(): JSX.Element {
  const runtime = useApplicationApi();
  const navigate = useNavigate();
  const flatRepository = createFlatQuestionRepository(createFlatQuestionClient());
  async function createFlatQuestion(): Promise<void> {
    const workspace: WorkspaceId = globalThis.crypto.randomUUID();
    await flatRepository.save(workspace, createDefaultFlatQuestionSource());
    navigate("/workspace/new", { state: { workspace } });
  }
  return (
    <WasmEditorPage
      repository={createWorkspaceEditorRepository(runtime.client, createInstructorPreviewClient())}
      onOpenDraft={(draft) =>
        navigate(`/workspace/${authoringWorkspaceRouteReference(draft.reference)}`)
      }
      onCreateFlatQuestion={createFlatQuestion}
    />
  );
}

interface WorkspaceEditorResolvedProps {
  readonly workspace: WorkspaceId;
}

function WorkspaceEditorResolved(props: WorkspaceEditorResolvedProps): JSX.Element {
  const runtime = useApplicationApi();
  const wasm = useWasmFacade();
  const workspace = props.workspace;
  const flatRepository = createFlatQuestionRepository(createFlatQuestionClient());
  const flatQuestionAssetClient = createFlatQuestionAssetClient();
  const qtiClient = createQtiProfileImportClient();
  const [focusConvertedDraft, setFocusConvertedDraft] = createSignal(false);
  const [displayedDraft, setDisplayedDraft] = createSignal<QtiConversionDraftState | null>(null);
  const [replacementPending, setReplacementPending] = createSignal(false);
  const [replacementFlatRead, setReplacementFlatRead] = createSignal<FlatQuestionRead | null>(null);
  const [flatRead] = createResource(
    () => workspace,
    async (selected): Promise<Awaited<ReturnType<typeof flatRepository.load>> | null> =>
      selected === undefined ? null : await flatRepository.load(selected),
  );
  const fallback = (): JSX.Element => (
    <WasmEditorPage
      repository={createWorkspaceEditorRepository(runtime.client, createInstructorPreviewClient())}
      initialWorkspace={workspace}
      onDraftDisplayStateChange={setDisplayedDraft}
      replacementPending={replacementPending()}
    />
  );
  const displayedFlatRead = (): FlatQuestionRead | null => {
    const replacement = replacementFlatRead();
    if (replacement !== null) return replacement;
    if (flatRead.state !== "ready") return null;
    return flatRead() ?? null;
  };
  return (
    <>
      <QtiProfileImportPage
        workspace={workspace}
        client={qtiClient}
        draftState={displayedDraft}
        onConversionHandoffChange={setReplacementPending}
        onConverted={async () => {
          setFocusConvertedDraft(true);
          const converted = await flatRepository.load(workspace);
          setReplacementFlatRead(converted);
        }}
      />
      <Show
        when={!flatRead.loading}
        fallback={
          <section class="page" aria-busy="true">
            <p role="status">Loading private workspace...</p>
          </section>
        }
      >
        <Show
          keyed
          when={displayedFlatRead()}
          fallback={
            flatRead.error instanceof FlatQuestionRequestError && flatRead.error.status === 404 ? (
              fallback()
            ) : (
              <section class="page" data-route-surface="flatQuestionLoadError" role="alert">
                <h1>Private question draft unavailable</h1>
                <p>Refresh the page to retry loading this private draft.</p>
              </section>
            )
          }
        >
          {(initial) => (
            <FlatQuestionEditorPage
              workspace={workspace}
              initial={initial}
              repository={flatRepository}
              api={runtime.client}
              responseValidator={wasm}
              assetClient={flatQuestionAssetClient}
              focusHeadingOnMount={focusConvertedDraft()}
              onHeadingFocusDelivered={() => setFocusConvertedDraft(false)}
              onDraftDisplayStateChange={setDisplayedDraft}
              replacementPending={replacementPending()}
            />
          )}
        </Show>
      </Show>
    </>
  );
}

/** Resolves a visible `W-n` locator before mounting the private editor transport. */
export function WorkspaceEditorLivePage(): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  const location = useLocation<{ readonly workspace?: unknown }>();
  const [workspace] = createResource(
    () => params["workspaceRef"],
    async (reference): Promise<WorkspaceId> => {
      if (reference === "new") {
        const state = location.state;
        if (typeof state?.workspace !== "string") {
          throw new Error("This new draft address has expired. Start again from the workspace.");
        }
        return state.workspace;
      }
      return await resolveWorkspaceRoute(runtime.client, reference);
    },
  );
  return (
    <Show
      when={workspace()}
      keyed
      fallback={
        <section class="page" aria-busy={workspace.loading}>
          <p role={workspace.error === undefined ? "status" : "alert"}>
            {workspace.error === undefined
              ? "Opening private workspace..."
              : "This private draft is unavailable. Return to the workspace and choose it again."}
          </p>
        </section>
      }
    >
      {(selected) => <WorkspaceEditorResolved workspace={selected} />}
    </Show>
  );
}
