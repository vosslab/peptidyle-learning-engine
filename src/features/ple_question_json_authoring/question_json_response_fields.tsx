// question_json_response_fields.tsx - response-format controls for ple-question-json authoring.

import { Show, type JSX } from "solid-js";

import { PleQuestionJsonChoiceList } from "./question_json_choice_list";
import { PleQuestionJsonMatchingEditor } from "./question_json_matching_editor";
import { PleQuestionJsonAdvancedResponseFields } from "./question_json_advanced_response_fields";
import {
  addChoice,
  addMatchingPair,
  removeChoice,
  removeMatchingPair,
  reorderMatchingSide,
  setChoiceFeedback,
  setChoiceText,
  setCorrectChoice,
  setPleQuestionJsonResponseKind,
  setMatchingSideText,
  setMatchingPair,
} from "./question_json_editor_model";
import type { PleQuestionJsonDocument } from "./question_json_source";

function singleChoiceResponse(
  source: PleQuestionJsonDocument,
): Extract<PleQuestionJsonDocument["response"], { readonly kind: "singleChoice" }> | null {
  return source.response.kind === "singleChoice" ? source.response : null;
}

function matchingResponse(
  source: PleQuestionJsonDocument,
): Extract<PleQuestionJsonDocument["response"], { readonly kind: "matching" }> | null {
  return source.response.kind === "matching" ? source.response : null;
}

function isEditableResponseKind(
  value: string,
): value is Exclude<PleQuestionJsonDocument["response"]["kind"], "hotspot"> {
  return (
    value === "singleChoice" ||
    value === "multipleAnswer" ||
    value === "fillIn" ||
    value === "multiFillIn" ||
    value === "numeric" ||
    value === "matching" ||
    value === "ordering"
  );
}

export function PleQuestionJsonResponseFields(props: {
  readonly source: () => PleQuestionJsonDocument;
  readonly fieldErrors: Readonly<Record<string, string>>;
  readonly disabled: boolean;
  readonly numericAnswerLiteral: () => string;
  readonly onNumericAnswerLiteralChange: (literal: string) => void;
  readonly onEdit: (source: PleQuestionJsonDocument) => void;
  readonly onMoveChoice: (choiceId: string, direction: "up" | "down") => void;
  readonly onStatus: (message: string) => void;
  readonly selectedKind: () => PleQuestionJsonDocument["response"]["kind"];
}): JSX.Element {
  const responseKind = props.selectedKind;

  function chooseFormat(
    kind: Exclude<PleQuestionJsonDocument["response"]["kind"], "hotspot">,
  ): void {
    props.onEdit(setPleQuestionJsonResponseKind(props.source(), kind));
  }

  return (
    <>
      <label class="ple-question-json-authoring__field">
        <span>Question format</span>
        <select
          value={responseKind()}
          disabled={props.disabled}
          onChange={(event) => {
            const kind = event.currentTarget.value;
            if (isEditableResponseKind(kind)) chooseFormat(kind);
          }}
        >
          <option value="singleChoice">Multiple choice (one answer)</option>
          <option value="multipleAnswer">Multiple answer (select all)</option>
          <option value="fillIn">Fill in the blank</option>
          <option value="multiFillIn">Multiple fill in the blank</option>
          <option value="numeric">Numerical entry</option>
          <option value="matching">Matching pairs</option>
          <option value="ordering">Ordered list</option>
        </select>
        <span class="ple-question-json-authoring__help">
          Choose the student task first. Changing the format starts a valid private draft for that
          format.
        </span>
      </label>
      <Show when={responseKind() === "singleChoice"}>
        {(_isSingleChoice) => (
          <PleQuestionJsonChoiceList
            choices={singleChoiceResponse(props.source())?.choices ?? []}
            correctChoice={singleChoiceResponse(props.source())?.correctChoice ?? ""}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onChoiceChange={(id, patch) => {
              const next =
                patch.text === undefined
                  ? setChoiceFeedback(props.source(), id, patch.feedback ?? null)
                  : setChoiceText(props.source(), id, patch.text);
              if (next.changed) props.onEdit(next.source);
            }}
            onCorrectChoiceChange={(id) => {
              const next = setCorrectChoice(props.source(), id);
              if (next.changed) props.onEdit(next.source);
            }}
            onAddChoice={() => {
              const next = addChoice(props.source());
              if (next.changed) props.onEdit(next.source);
            }}
            onRemoveChoice={(id) => {
              const next = removeChoice(props.source(), id);
              if (next.changed) props.onEdit(next.source);
            }}
            onMoveChoice={props.onMoveChoice}
          />
        )}
      </Show>
      <Show when={responseKind() === "matching"}>
        {(_isMatching) => (
          <PleQuestionJsonMatchingEditor
            prompts={matchingResponse(props.source())?.prompts ?? []}
            choices={matchingResponse(props.source())?.choices ?? []}
            matches={matchingResponse(props.source())?.matches ?? []}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onPromptTextChange={(id, text) => {
              const next = setMatchingSideText(props.source(), "prompts", id, text);
              if (next.changed) props.onEdit(next.source);
            }}
            onChoiceTextChange={(id, text) => {
              const next = setMatchingSideText(props.source(), "choices", id, text);
              if (next.changed) props.onEdit(next.source);
            }}
            onMatchChange={(prompt, choice) => {
              const next = setMatchingPair(props.source(), prompt, choice);
              if (next.changed) props.onEdit(next.source);
            }}
            onAddPair={() => {
              const next = addMatchingPair(props.source());
              if (next.changed) props.onEdit(next.source);
            }}
            onRemovePair={(prompt) => {
              const next = removeMatchingPair(props.source(), prompt);
              if (next.changed) props.onEdit(next.source);
              else if (next.error !== null) props.onStatus(next.error);
            }}
            onMoveItem={(side, id, direction) => {
              const response = matchingResponse(props.source());
              if (response === null) return;
              const items = response[side];
              const index = items.findIndex((item) => item.id === id);
              const other = direction === "earlier" ? index - 1 : index + 1;
              if (index < 0 || other < 0 || other >= items.length) return;
              const ordered = items.map((item) => item.id);
              const displaced = ordered[other];
              if (displaced === undefined) return;
              ordered[other] = id;
              ordered[index] = displaced;
              const next = reorderMatchingSide(props.source(), side, ordered);
              if (next.changed) props.onEdit(next.source);
              else if (next.error !== null) props.onStatus(next.error);
            }}
            onStatus={props.onStatus}
          />
        )}
      </Show>
      <PleQuestionJsonAdvancedResponseFields
        source={props.source}
        fieldErrors={props.fieldErrors}
        disabled={props.disabled}
        numericAnswerLiteral={props.numericAnswerLiteral}
        onNumericAnswerLiteralChange={props.onNumericAnswerLiteralChange}
        onEdit={props.onEdit}
        onStatus={props.onStatus}
        selectedKind={responseKind}
      />
    </>
  );
}
