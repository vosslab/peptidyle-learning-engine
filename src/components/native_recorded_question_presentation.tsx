// Immutable native Question review, without response controls.

import { For, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { QuestionPresentationResponseFormat } from "../../generated/api/QuestionPresentationResponseFormat";
import { ContentBlockList } from "./student_feedback_panel";
import {
  QuestionPresentationRenderer,
  resolveSameOriginQuestionImageUrl,
  type QuestionImageUrlResolver,
} from "./question_renderer";

type PresentedItem = { readonly body: ReadonlyArray<QuestionContentBlock> };

function PresentedItemList(props: {
  readonly title: string;
  readonly items: ReadonlyArray<PresentedItem>;
  readonly presentation: QuestionPresentation;
  readonly questionImageUrl: QuestionImageUrlResolver;
}): JSX.Element {
  return (
    <section class="attempt-history__response-format" aria-label={props.title}>
      <h5>{props.title}</h5>
      <ol class="attempt-history__format-items">
        <For each={props.items}>
          {(item) => (
            <li>
              <ContentBlockList
                blocks={item.body}
                publishedQuestionRevisionTuple={props.presentation.publishedQuestionRevisionTuple}
                questionImageUrl={props.questionImageUrl}
              />
            </li>
          )}
        </For>
      </ol>
    </section>
  );
}

function RecordedResponseFormat(props: {
  readonly presentation: QuestionPresentation;
  readonly questionImageUrl: QuestionImageUrlResolver;
}): JSX.Element {
  const format: QuestionPresentationResponseFormat = props.presentation.response;
  switch (format.kind) {
    case "singleChoice":
    case "multipleAnswer":
      return (
        <PresentedItemList
          title="Answer choices"
          items={format.choices}
          presentation={props.presentation}
          questionImageUrl={props.questionImageUrl}
        />
      );
    case "matching":
      return (
        <>
          <PresentedItemList
            title="Matching prompts"
            items={format.prompts}
            presentation={props.presentation}
            questionImageUrl={props.questionImageUrl}
          />
          <PresentedItemList
            title="Matching choices"
            items={format.choices}
            presentation={props.presentation}
            questionImageUrl={props.questionImageUrl}
          />
        </>
      );
    case "ordering":
      return (
        <PresentedItemList
          title="Ordering items"
          items={format.items}
          presentation={props.presentation}
          questionImageUrl={props.questionImageUrl}
        />
      );
    case "multiFillIn":
      return (
        <section class="attempt-history__response-format" aria-label="Blanks">
          <h5>Blanks</h5>
          <ol class="attempt-history__format-items">
            <For each={format.blanks}>
              {(blank) => (
                <li>
                  <ContentBlockList
                    blocks={blank.label}
                    publishedQuestionRevisionTuple={
                      props.presentation.publishedQuestionRevisionTuple
                    }
                    questionImageUrl={props.questionImageUrl}
                  />
                </li>
              )}
            </For>
          </ol>
        </section>
      );
    case "hotspot": {
      const imageUrl = (): string =>
        resolveSameOriginQuestionImageUrl(
          format.surface.questionImageAssetTuple,
          props.presentation.publishedQuestionRevisionTuple,
          props.questionImageUrl,
        );
      return (
        <section class="attempt-history__response-format" aria-label="Labeled image regions">
          <h5>Labeled image regions</h5>
          <img
            class="attempt-history__hotspot-image"
            src={imageUrl()}
            alt={format.surface.description}
          />
          <PresentedItemList
            title="Regions"
            items={format.surface.regions.map((region) => ({ body: region.label }))}
            presentation={props.presentation}
            questionImageUrl={props.questionImageUrl}
          />
        </section>
      );
    }
    case "fillIn":
      return <p class="attempt-history__response-format">Fill in the blank.</p>;
    case "numerical":
      return <p class="attempt-history__response-format">Enter a numerical response.</p>;
    case "imathasQuestionBackend":
    case "backendOwned":
      return null;
    default: {
      const exhaustive: never = format;
      throw new Error(`Unhandled recorded response format: ${JSON.stringify(exhaustive)}`);
    }
  }
}

/** Shows the exact answer-free native presentation without disabled editing controls. */
export function NativeRecordedQuestionPresentation(props: {
  readonly questionPosition: number;
  readonly presentation: QuestionPresentation;
  readonly questionImageUrl: QuestionImageUrlResolver;
}): JSX.Element {
  return (
    <section
      class="attempt-history__question-presentation"
      aria-labelledby={`question-${props.questionPosition}-prompt`}
    >
      <h4 id={`question-${props.questionPosition}-prompt`}>Question</h4>
      <QuestionPresentationRenderer
        presentation={props.presentation}
        questionImageUrl={props.questionImageUrl}
      />
      <RecordedResponseFormat
        presentation={props.presentation}
        questionImageUrl={props.questionImageUrl}
      />
    </section>
  );
}
