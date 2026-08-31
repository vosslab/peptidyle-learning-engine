// library_route_page.tsx - runtime composition for the production catalog surface.

import type { JSX } from "solid-js";

import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { createQuestionCurationRepository } from "../features/question_curation/question_curation_repository";
import { mayMutatePersonalCuration } from "../features/question_curation/question_curation_model";
import { LibraryPage } from "./library_page";

export function LibraryRoutePage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const sessionState = session.state();
  const catalog = createQuestionLibraryRepository(runtime.client);
  const curation = createQuestionCurationRepository(runtime.client, catalog);
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
