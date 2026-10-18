// my_questions_page.tsx - authored Question Library route composition.

import type { JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { LibraryWorkspace } from "../features/question_library/library_workspace";
import { PageFrame } from "../components/page_frame/page_frame";

/** A predefined authored-Question search through the shared Library surface. */
export function MyQuestionsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(
    runtime.client,
    "authoredByCurrentAccount",
  );
  return (
    <PageFrame routeSurface="myQuestions">
      <LibraryWorkspace
        mode="search"
        collectionScope="myQuestions"
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
