// editor_live_pages.tsx - route composition for accepted workspace CRUD only.

import { useParams } from "@solidjs/router";
import type { JSX } from "solid-js";

import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { WasmEditorPage } from "./editor_page";
import { createInstructorPreviewClient } from "./editor_instructor_preview";
import { createWorkspaceEditorRepository } from "./editor_workspace_repository";

function currentWorkspace(
  params: Readonly<Record<string, string | undefined>>,
): WorkspaceId | undefined {
  return params["workspaceId"];
}

function mayAuthorWorkspace(): boolean {
  const session = useSessionBootstrap().state();
  if (session.kind !== "authenticated") return false;
  return session.session.user.roles.some((role) =>
    ["instructor", "publisher", "administrator"].includes(role),
  );
}

function WorkspaceAuthoringDenied(): JSX.Element {
  return (
    <section class="page" data-route-surface="workspaceAuthoringDenied" aria-live="polite">
      <p class="eyebrow">Instructor workspace</p>
      <h1>Workspace authoring is not available for this account</h1>
      <p>Your learning space remains available. Ask a course administrator for authoring access.</p>
    </section>
  );
}

/** `/workspace`: a server-backed private draft list with its first draft selected. */
export function WorkspaceListLivePage(): JSX.Element {
  const runtime = useApiRuntime();
  if (!mayAuthorWorkspace()) return <WorkspaceAuthoringDenied />;
  return (
    <WasmEditorPage
      repository={createWorkspaceEditorRepository(runtime.client, createInstructorPreviewClient())}
    />
  );
}

/** `/workspace/:workspaceId`: requests exactly the private workspace selected by the route. */
export function WorkspaceEditorLivePage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  if (!mayAuthorWorkspace()) return <WorkspaceAuthoringDenied />;
  return (
    <WasmEditorPage
      repository={createWorkspaceEditorRepository(runtime.client, createInstructorPreviewClient())}
      initialWorkspace={currentWorkspace(params)}
    />
  );
}
