// question_detail_page.tsx - safe current Question Details and Question Revision lineage View.

import { A, createAsync, useParams, useSearchParams } from "@solidjs/router";
import { Show, Suspense, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { CopyableQuestionId } from "../components/copyable_question_id";
import { QuestionWatchControl } from "../components/question_watch_control";
import { QuestionStarControl } from "../components/question_star_control";
import { QuestionPromptRenderer } from "../components/question_renderer";
import { parseQuestionRouteReference } from "../navigation/public_route";
import {
  parseQuestionLibraryReturnToken,
  questionLibraryReturnPath,
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
} from "./library_page_model";
import { QuestionStatisticsPanel, QuestionUsePanel } from "./question_statistics_panel";
import "./question_detail_page.css";

function webworkFormatLabel(value: string | null): string | null {
  if (value === "webworkPg") return "PG";
  if (value === "webworkPgml") return "PGML";
  return null;
}

export function QuestionDetailPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  const [searchParams] = useSearchParams();
  const libraryReturnToken = () =>
    parseQuestionLibraryReturnToken(searchParams[QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER]);
  const libraryReturnHref = () => {
    const token = libraryReturnToken();
    return token === null ? "/library" : questionLibraryReturnPath(token);
  };
  const detail = createAsync(() => {
    const questionReference = params["questionRef"];
    if (
      questionReference === undefined ||
      parseQuestionRouteReference(questionReference) === null
    ) {
      throw new Error("The Question ID address is incomplete.");
    }
    return applicationApi.client
      .resolveQuestion(questionReference)
      .then((summary) => applicationApi.queries.questionDetails(summary.questionId));
  });
  return (
    <section class="page" data-route-surface="questionDetail">
      <A class="quiet-link" href={libraryReturnHref()}>
        Return to question library
      </A>
      <Suspense
        fallback={
          <p class="loading-state" role="status">
            Loading question...
          </p>
        }
      >
        <Show
          when={detail()}
          fallback={
            <section class="route-error" role="alert">
              <h1>Question unavailable</h1>
              <p>Return to the library and try again.</p>
            </section>
          }
        >
          {(record) => (
            <article>
              <p class="eyebrow">Published question</p>
              <h1>{record().summary.metadata.questionTitle}</h1>
              <section aria-label="Question Description">
                <h2>Question Description</h2>
                <p>{record().summary.metadata.questionDescription}</p>
              </section>
              <CopyableQuestionId
                questionTitle={record().summary.metadata.questionTitle}
                displayId={record().summary.questionId}
              />
              <QuestionStarControl questionId={record().summary.questionId} />
              <QuestionWatchControl questionId={record().summary.questionId} />
              <p aria-label="Question Authors">
                Authors:{" "}
                {record()
                  .summary.authorship.authors.map((author) => author.displayName)
                  .join(", ")}
              </p>
              <p>
                {`Backend: ${record().summary.backend}`}
                <Show when={webworkFormatLabel(record().summary.questionFormat)}>
                  {(format) => ` · Format: ${format()}`}
                </Show>
              </p>
              <Show when={record().prompt.kind === "generatedExample"}>
                <aside class="question-library-generated-example" aria-label="Generated example">
                  <strong>Generated example</strong>
                  <p>
                    This example uses resolved values for Question Library viewing. Assigned
                    versions may use different values.
                  </p>
                </aside>
              </Show>
              <section aria-label="Question prompt">
                <QuestionPromptRenderer
                  blocks={record().prompt.blocks}
                  questionRevision={record().summary.latestQuestionRevision}
                  assetUrl={(asset) =>
                    new URL(
                      applicationApi.client.assetUrl(
                        record().summary.latestQuestionRevision,
                        asset.questionAsset,
                      ),
                      window.location.origin,
                    )
                  }
                />
              </section>
              <QuestionStatisticsPanel evidence={record().evidence} />
              <QuestionUsePanel usage={record().usage} />
            </article>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
