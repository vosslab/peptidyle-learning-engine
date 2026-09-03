// library_route_page.tsx - runtime composition for the production Question Library surface.

import type { JSX } from "solid-js";

import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { useApplicationApi } from "../api/application_api";
import { LibraryPage } from "./library_page";

export function LibraryRoutePage(): JSX.Element {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  return <LibraryPage repository={questionLibrary} />;
}
