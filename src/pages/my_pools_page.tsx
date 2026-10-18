// my_pools_page.tsx - owned Question Pool Library route composition.

import type { JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { LibraryWorkspace } from "../features/question_library/library_workspace";
import { PageFrame } from "../components/page_frame/page_frame";

/** A predefined current-owner Pool search through the shared Library surface. */
export function MyPoolsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  return (
    <PageFrame routeSurface="myPools">
      <LibraryWorkspace
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
    </PageFrame>
  );
}
