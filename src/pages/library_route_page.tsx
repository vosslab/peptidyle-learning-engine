// library_route_page.tsx - runtime composition for the production catalog surface.

import type { JSX } from "solid-js";

import { createCatalogRepository } from "../api/catalog_repository";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { createProblemCurationRepository } from "../features/problem_curation/problem_curation_repository";
import { mayMutatePersonalCuration } from "../features/problem_curation/problem_curation_model";
import { LibraryPage } from "./library_page";

export function LibraryRoutePage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const sessionState = session.state();
  const catalog = createCatalogRepository(runtime.client);
  const curation = createProblemCurationRepository(runtime.client, catalog);
  return (
    <LibraryPage
      repository={catalog}
      curation={curation.curation}
      pickerRepository={curation.picker}
      mayMutatePersonalCuration={mayMutatePersonalCuration(
        sessionState.kind === "authenticated" ? sessionState.session : undefined,
      )}
    />
  );
}
