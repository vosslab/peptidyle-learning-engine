// library_route_page.tsx - runtime composition for the production Question Library surface.

import type { JSX } from "solid-js";

import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { useApplicationApi } from "../api/application_api";
import { LibraryWorkspace } from "../features/question_library/library_workspace";
import { PageFrame } from "../components/page_frame/page_frame";

export function LibraryRoutePage(): JSX.Element {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  return (
    <PageFrame routeSurface="library">
      <LibraryWorkspace
        mode="search"
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

export function BrowseLibraryRoutePage(): JSX.Element {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  return (
    <PageFrame routeSurface="library-browse">
      <LibraryWorkspace
        mode="browse"
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
