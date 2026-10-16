// my_pools_page.tsx - owned Question Pool Library route composition.

import type { JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { LibraryPage } from "./library_page";

/** A predefined current-owner Pool search through the shared Library surface. */
export function MyPoolsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  return (
    <LibraryPage
      mode="search"
      collectionScope="myPools"
      repository={questionLibrary}
      metadataClient={runtime.client}
      poolMetadataClient={runtime.client}
      classificationClient={runtime.client}
      questionPoolClient={runtime.client}
      getQuestionDetails={runtime.client.getQuestionDetails}
      loadQuestionInspection={runtime.client.getQuestionRevision}
      questionRevisionPreviewDocumentUrl={runtime.client.questionRevisionPreviewDocumentUrl}
      questionImageUrl={runtime.client.questionImageUrl}
    />
  );
}
