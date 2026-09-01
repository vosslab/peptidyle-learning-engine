// editor_live_pages.tsx - route composition for accepted workspace CRUD only.

import { useLocation, useNavigate, useParams } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import { useApplicationApi } from "../api/application_api";
import { useWasmFacade } from "../wasm/context";
import {
  createPleQuestionJsonClient,
  PleQuestionJsonRequestError,
  type PleQuestionJsonRead,
} from "../features/ple_question_json_authoring/question_json_client";
import { createDefaultPleQuestionJsonSource } from "../features/ple_question_json_authoring/question_json_defaults";
import { createPleQuestionJsonAssetClient } from "../features/ple_question_json_authoring/question_json_asset_client";
import { PleQuestionJsonEditorPage } from "../features/ple_question_json_authoring/question_json_editor_page";
import { createPleQuestionJsonRepository } from "../features/ple_question_json_authoring/question_json_repository";
import { WasmEditorPage } from "./editor_page";
import { createInstructorPreviewClient } from "./editor_instructor_preview";
import { createWorkspaceEditorRepository } from "./editor_workspace_repository";
import { authoringWorkspaceRouteReference } from "../navigation/public_route";
import { resolveWorkspaceRoute } from "../navigation/resolved_route";

/** `/workspace`: a server-backed private draft list with its first draft selected. */
export function WorkspaceListLivePage(): JSX.Element {
  const runtime = useApplicationApi();
  const navigate = useNavigate();
  const flatRepository = createPleQuestionJsonRepository(createPleQuestionJsonClient());
  async function createPleQuestionJson(): Promise<void> {
    const workspace: WorkspaceId = globalThis.crypto.randomUUID();
    await flatRepository.save(workspace, createDefaultPleQuestionJsonSource());
    navigate("/workspace/new", { state: { workspace } });
  }
  return (
    <WasmEditorPage
      repository={createWorkspaceEditorRepository(runtime.client, createInstructorPreviewClient())}
      onOpenDraft={(draft) =>
        navigate(`/workspace/${authoringWorkspaceRouteReference(draft.authoringWorkspace)}`)
      }
      onCreatePleQuestionJson={createPleQuestionJson}
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
  const flatRepository = createPleQuestionJsonRepository(createPleQuestionJsonClient());
  const pleQuestionJsonAssetClient = createPleQuestionJsonAssetClient();
  const [flatRead] = createResource(
    () => workspace,
    async (selected): Promise<Awaited<ReturnType<typeof flatRepository.load>> | null> =>
      selected === undefined ? null : await flatRepository.load(selected),
  );
  const fallback = (): JSX.Element => (
    <WasmEditorPage
      repository={createWorkspaceEditorRepository(runtime.client, createInstructorPreviewClient())}
      initialWorkspace={workspace}
    />
  );
  const displayedFlatRead = (): PleQuestionJsonRead | null => {
    if (flatRead.state !== "ready") return null;
    return flatRead() ?? null;
  };
  return (
    <>
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
            flatRead.error instanceof PleQuestionJsonRequestError &&
            flatRead.error.status === 404 ? (
              fallback()
            ) : (
              <section class="page" data-route-surface="pleQuestionJsonLoadError" role="alert">
                <h1>Private question draft unavailable</h1>
                <p>Refresh the page to retry loading this private draft.</p>
              </section>
            )
          }
        >
          {(initial) => (
            <PleQuestionJsonEditorPage
              workspace={workspace}
              initial={initial}
              repository={flatRepository}
              api={runtime.client}
              responseValidator={wasm}
              assetClient={pleQuestionJsonAssetClient}
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
          throw new Error(
            "This new draft address has expired. Start again from My Question Drafts.",
          );
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
              ? "Opening My Question Drafts..."
              : "This private draft is unavailable. Return to My Question Drafts and choose it again."}
          </p>
        </section>
      }
    >
      {(selected) => <WorkspaceEditorResolved workspace={selected} />}
    </Show>
  );
}
