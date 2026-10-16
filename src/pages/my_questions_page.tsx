// my_questions_page.tsx - authored Question Library route composition.

import type { JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { LibraryPage } from "./library_page";

/** A predefined authored-Question search through the shared Library surface. */
export function MyQuestionsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(
    runtime.client,
    "authoredByCurrentAccount",
  );
  return (
    <LibraryPage
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
  );
}
