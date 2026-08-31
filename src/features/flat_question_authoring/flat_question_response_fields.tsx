// flat_question_response_fields.tsx - response-format controls for flat-question authoring.

import { Show, type JSX } from "solid-js";

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import { FlatChoiceList } from "./flat_choice_list";
import { FlatMatchingEditor } from "./flat_matching_editor";
import { FlatQuestionAdvancedResponseFields } from "./flat_question_advanced_response_fields";
import type {
  FlatQuestionAssetClient,
  FlatQuestionAssetDescriptor,
} from "./flat_question_asset_client";
import {
  addChoice,
  addMatchingPair,
  removeChoice,
  removeMatchingPair,
  reorderMatchingItems,
  setChoiceFeedback,
  setChoiceText,
  setCorrectChoice,
  setFlatQuestionResponseKind,
  setMatchingItemText,
  setMatchingPair,
} from "./flat_question_editor_model";
import type { FlatQuestionHotspotResponse, FlatQuestionSourceV2 } from "./flat_question_source";

function singleChoiceResponse(
  source: FlatQuestionSourceV2,
): Extract<FlatQuestionSourceV2["response"], { readonly kind: "singleChoice" }> | null {
  return source.response.kind === "singleChoice" ? source.response : null;
}

function matchingResponse(
  source: FlatQuestionSourceV2,
): Extract<FlatQuestionSourceV2["response"], { readonly kind: "matching" }> | null {
  return source.response.kind === "matching" ? source.response : null;
}

function isEditableResponseKind(
  value: string,
): value is Exclude<FlatQuestionSourceV2["response"]["kind"], "hotspot"> {
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

export function FlatQuestionResponseFields(props: {
  readonly source: () => FlatQuestionSourceV2;
  readonly fieldErrors: Readonly<Record<string, string>>;
  readonly disabled: boolean;
  readonly numericAnswerLiteral: () => string;
  readonly onNumericAnswerLiteralChange: (literal: string) => void;
  readonly onEdit: (source: FlatQuestionSourceV2) => void;
  readonly onMoveChoice: (choiceId: string, direction: "up" | "down") => void;
  readonly onStatus: (message: string) => void;
  readonly selectedKind: () => FlatQuestionSourceV2["response"]["kind"];
  readonly hotspotResponse: () => FlatQuestionHotspotResponse | null;
  readonly pendingHotspotDescription: () => string;
  readonly assetClient: FlatQuestionAssetClient | undefined;
  readonly workspace: WorkspaceId;
  readonly onChooseHotspot: () => void;
  readonly onSelectHotspotAsset: (asset: FlatQuestionAssetDescriptor) => void;
  readonly onPendingHotspotDescriptionChange: (description: string) => void;
  readonly onChooseOrdinaryFormat: () => void;
}): JSX.Element {
  const responseKind = props.selectedKind;

  function chooseFormat(kind: Exclude<FlatQuestionSourceV2["response"]["kind"], "hotspot">): void {
    props.onEdit(setFlatQuestionResponseKind(props.source(), kind));
  }

  return (
    <>
      <label class="flat-question-authoring__field">
        <span>Question format</span>
        <select
          value={responseKind()}
          disabled={props.disabled}
          onChange={(event) => {
            const kind = event.currentTarget.value;
            if (isEditableResponseKind(kind)) {
              props.onChooseOrdinaryFormat();
              chooseFormat(kind);
            } else if (kind === "hotspot") props.onChooseHotspot();
          }}
        >
          <option value="singleChoice">Multiple choice (one answer)</option>
          <option value="multipleAnswer">Multiple answer (select all)</option>
          <option value="fillIn">Fill in the blank</option>
          <option value="multiFillIn">Multiple fill in the blank</option>
          <option value="numeric">Numerical entry</option>
          <option value="matching">Matching pairs</option>
          <option value="ordering">Ordered list</option>
          <option value="hotspot">Image hotspot</option>
        </select>
        <span class="flat-question-authoring__help">
          Choose the student task first. Changing the format starts a valid private draft for that
          format. Image hotspot starts with a verified image and student-facing description.
        </span>
      </label>
      <Show when={responseKind() === "singleChoice"}>
        {(_isSingleChoice) => (
          <FlatChoiceList
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
          <FlatMatchingEditor
            prompts={matchingResponse(props.source())?.prompts ?? []}
            choices={matchingResponse(props.source())?.choices ?? []}
            matches={matchingResponse(props.source())?.matches ?? []}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onPromptTextChange={(id, text) => {
              const next = setMatchingItemText(props.source(), "prompts", id, text);
              if (next.changed) props.onEdit(next.source);
            }}
            onChoiceTextChange={(id, text) => {
              const next = setMatchingItemText(props.source(), "choices", id, text);
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
              const next = reorderMatchingItems(props.source(), side, ordered);
              if (next.changed) props.onEdit(next.source);
              else if (next.error !== null) props.onStatus(next.error);
            }}
            onStatus={props.onStatus}
          />
        )}
      </Show>
      <FlatQuestionAdvancedResponseFields
        source={props.source}
        fieldErrors={props.fieldErrors}
        disabled={props.disabled}
        numericAnswerLiteral={props.numericAnswerLiteral}
        onNumericAnswerLiteralChange={props.onNumericAnswerLiteralChange}
        onEdit={props.onEdit}
        onStatus={props.onStatus}
        selectedKind={responseKind}
        hotspotResponse={props.hotspotResponse}
        pendingHotspotDescription={props.pendingHotspotDescription}
        assetClient={props.assetClient}
        workspace={props.workspace}
        onSelectHotspotAsset={props.onSelectHotspotAsset}
        onPendingHotspotDescriptionChange={props.onPendingHotspotDescriptionChange}
      />
    </>
  );
}
