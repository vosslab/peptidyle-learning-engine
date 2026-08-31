// library_route_page.tsx - runtime composition for the production Question Library surface.

import type { JSX } from "solid-js";

import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { createQuestionCurationRepository } from "../features/question_curation/question_curation_repository";
import { mayMutatePersonalCuration } from "../features/question_curation/question_curation_model";
import { LibraryPage } from "./library_page";

export function LibraryRoutePage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const sessionState = session.state();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  const curation = createQuestionCurationRepository(runtime.client, questionLibrary);
  return (
    <LibraryPage
      repository={questionLibrary}
      curation={curation.curation}
      pickerRepository={curation.picker}
      mayMutatePersonalCuration={mayMutatePersonalCuration(
        sessionState.kind === "authenticated" ? sessionState.session : undefined,
      )}
    />
  );
}
