// Inert, answer-free native response controls for Published Question inspection.
import { For, Show, createUniqueId, type JSX } from "solid-js";
import type { QuestionResponsePreview } from "../../generated/api/QuestionResponsePreview";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import { QuestionPromptRenderer, type QuestionPromptRendererProps } from "./question_renderer";
import "./question_response_preview.css";

export function QuestionResponsePreviewControl(
  props: Omit<QuestionPromptRendererProps, "blocks"> & {
    readonly preview: QuestionResponsePreview;
  },
): JSX.Element {
  const id = createUniqueId();
  const content = (blocks: ReadonlyArray<QuestionContentBlock>): JSX.Element => (
    <QuestionPromptRenderer
      blocks={blocks}
      publishedQuestionRevisionTuple={props.publishedQuestionRevisionTuple}
      questionImageUrl={props.questionImageUrl}
    />
  );
  function controls(preview: QuestionResponsePreview): JSX.Element {
    switch (preview.kind) {
      case "numeric":
        return (
          <label>
            Numeric response <input type="text" inputmode="decimal" /> {preview.unit}
          </label>
        );
      case "shortText":
        return (
          <label>
            Text response <input type="text" />
          </label>
        );
      case "multipleChoice":
        return (
          <For each={preview.choices}>
            {(choice, index) => (
              <div class="question-preview-choice">
                <input
                  type={preview.selection.kind === "exactlyOne" ? "radio" : "checkbox"}
                  name={id}
                  aria-labelledby={`${id}-choice-${index()}`}
                />
                <div id={`${id}-choice-${index()}`}>{content(choice)}</div>
              </div>
            )}
          </For>
        );
      case "multiBlank":
        return (
          <For each={preview.labels}>
            {(label, index) => (
              <div>
                <div id={`${id}-blank-${index()}`}>{content(label)}</div>
                <input type="text" aria-labelledby={`${id}-blank-${index()}`} />
              </div>
            )}
          </For>
        );
      case "matching":
        return (
          <div class="question-preview-matching">
            <section aria-label="Matching prompts">
              <For each={preview.prompts}>
                {(prompt, index) => (
                  <div>
                    <div id={`${id}-match-${index()}`}>{content(prompt)}</div>
                    <input
                      type="text"
                      placeholder="Choose from the bank"
                      aria-labelledby={`${id}-match-${index()}`}
                    />
                  </div>
                )}
              </For>
            </section>
            <section aria-label="Choice bank">
              <h3>Choice bank</h3>
              <ul>
                <For each={preview.choices}>{(choice) => <li>{content(choice)}</li>}</For>
              </ul>
            </section>
          </div>
        );
      case "ordering":
        return (
          <ol>
            <For each={preview.items}>
              {(item) => (
                <li>
                  {content(item)}
                  <button type="button">Move earlier</button>{" "}
                  <button type="button">Move later</button>
                </li>
              )}
            </For>
          </ol>
        );
      case "hotspot":
        return (
          <>
            <div class="question-preview-hotspot">
              <img
                src={props.questionImageUrl(preview.questionImageAssetTuple).href}
                alt={preview.description}
              />
              <For each={preview.regions}>
                {(region, index) => (
                  <span
                    class="question-preview-region"
                    aria-hidden="true"
                    style={{
                      left: `${region.x / 100}%`,
                      top: `${region.y / 100}%`,
                      width: `${region.width / 100}%`,
                      height: `${region.height / 100}%`,
                    }}
                  >
                    {index() + 1}
                  </span>
                )}
              </For>
            </div>
            <For each={preview.regions}>
              {(region, index) => (
                <div class="question-preview-choice">
                  <input
                    type={preview.selection.kind === "exactlyOne" ? "radio" : "checkbox"}
                    name={id}
                    aria-labelledby={`${id}-region-${index()}`}
                  />
                  <span>{index() + 1}.</span>
                  <div id={`${id}-region-${index()}`}>{content(region.label)}</div>
                </div>
              )}
            </For>
          </>
        );
    }
  }
  return (
    <fieldset class="question-response-preview" disabled aria-describedby={`${id}-notice`}>
      <legend>Response preview</legend>
      <p id={`${id}-notice`}>Preview only. Response controls are inactive.</p>
      <Show when={props.preview} keyed>
        {controls}
      </Show>
    </fieldset>
  );
}
