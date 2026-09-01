// question_json_advanced_response_fields.tsx - protected editors for advanced PLE Question JSON response kinds.

import { Show, type JSX } from "solid-js";

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import { PleQuestionJsonHotspotEditor } from "./question_json_hotspot_editor";
import {
  addHotspotRegion,
  moveHotspotRegion,
  removeHotspotRegion,
  setHotspotCorrectRegion,
  setHotspotDescription,
  setHotspotRegionCoordinate,
  setHotspotRegionLabel,
} from "./question_json_hotspot_editor_model";
import { PleQuestionJsonFillInEditor } from "./question_json_fill_in_editor";
import { PleQuestionJsonMultiFillInEditor } from "./question_json_multi_fill_in_editor";
import {
  addMultiFillBlank,
  addMultiFillBlankAnswer,
  removeMultiFillBlank,
  removeMultiFillBlankAnswer,
  reorderMultiFillBlanks,
  setMultiFillBlankAnswer,
  setMultiFillBlankLabel,
  setMultiFillBlankMatchMode,
  setMultiFillBlankMaxLength,
} from "./question_json_multi_fill_in_editor_model";
import { PleQuestionJsonMultipleAnswerEditor } from "./question_json_multiple_answer_editor";
import {
  addMultipleAnswerChoice,
  moveMultipleAnswerChoice,
  removeMultipleAnswerChoice,
  setMultipleAnswerChoiceFeedback,
  setMultipleAnswerChoiceText,
  setMultipleAnswerCorrect,
} from "./question_json_multiple_answer_model";
import { PleQuestionJsonNumericEditor } from "./question_json_numeric_editor";
import { numericResponseFromAuthoring } from "./question_json_numeric_model";
import { PleQuestionJsonOrderingEditor } from "./question_json_ordering_editor";
import {
  addOrderingItem,
  moveOrderingItem,
  removeOrderingItem,
  setOrderingItemText,
} from "./question_json_ordering_editor_model";
import type {
  PleQuestionJsonAssetClient,
  PleQuestionJsonAssetDescriptor,
} from "./question_json_asset_client";
import type {
  PleQuestionJsonHotspotResponse,
  PleQuestionJsonNumericResponseTolerance,
  PleQuestionJsonDocument,
} from "./question_json_source";

type SourceEditResult = {
  readonly source: PleQuestionJsonDocument;
  readonly changed: boolean;
  readonly error: string | null;
  readonly status: string | null;
};

export interface PleQuestionJsonAdvancedResponseFieldsProps {
  readonly source: () => PleQuestionJsonDocument;
  readonly fieldErrors: Readonly<Record<string, string>>;
  readonly disabled: boolean;
  readonly numericAnswerLiteral: () => string;
  readonly onNumericAnswerLiteralChange: (literal: string) => void;
  readonly onEdit: (source: PleQuestionJsonDocument) => void;
  readonly onStatus: (message: string) => void;
  readonly selectedKind: () => PleQuestionJsonDocument["response"]["kind"];
  readonly hotspotResponse: () => PleQuestionJsonHotspotResponse | null;
  readonly pendingHotspotDescription: () => string;
  readonly assetClient: PleQuestionJsonAssetClient | undefined;
  readonly workspace: WorkspaceId;
  readonly onSelectHotspotAsset: (asset: PleQuestionJsonAssetDescriptor) => void;
  readonly onPendingHotspotDescriptionChange: (description: string) => void;
}

function applyResult(
  result: SourceEditResult,
  onEdit: (source: PleQuestionJsonDocument) => void,
  onStatus: (message: string) => void,
): void {
  if (result.changed) onEdit(result.source);
  if (result.error !== null) onStatus(result.error);
  if (result.status !== null) onStatus(result.status);
}

/** Keeps complex response editors separate from page-level saving and publication orchestration. */
export function PleQuestionJsonAdvancedResponseFields(
  props: PleQuestionJsonAdvancedResponseFieldsProps,
): JSX.Element {
  const multipleAnswer = (): Extract<
    PleQuestionJsonDocument["response"],
    { readonly kind: "multipleAnswer" }
  > | null => {
    const response = props.source().response;
    return response.kind === "multipleAnswer" ? response : null;
  };
  const fillIn = (): Extract<
    PleQuestionJsonDocument["response"],
    { readonly kind: "fillIn" }
  > | null => {
    const response = props.source().response;
    return response.kind === "fillIn" ? response : null;
  };
  const multiFillIn = (): Extract<
    PleQuestionJsonDocument["response"],
    { readonly kind: "multiFillIn" }
  > | null => {
    const response = props.source().response;
    return response.kind === "multiFillIn" ? response : null;
  };
  const numeric = (): Extract<
    PleQuestionJsonDocument["response"],
    { readonly kind: "numeric" }
  > | null => {
    const response = props.source().response;
    return response.kind === "numeric" ? response : null;
  };
  const ordering = (): Extract<
    PleQuestionJsonDocument["response"],
    { readonly kind: "ordering" }
  > | null => {
    const response = props.source().response;
    return response.kind === "ordering" ? response : null;
  };

  function editResponse(response: PleQuestionJsonDocument["response"]): void {
    props.onEdit({ ...props.source(), response });
  }

  function moveBlank(blankId: string, direction: "earlier" | "later"): void {
    const response = multiFillIn();
    if (response === null) return;
    const index = response.blanks.findIndex((blank) => blank.id === blankId);
    const other = direction === "earlier" ? index - 1 : index + 1;
    if (index < 0 || other < 0 || other >= response.blanks.length) return;
    const ordered = response.blanks.map((blank) => blank.id);
    const displaced = ordered[other];
    if (displaced === undefined) return;
    ordered[other] = blankId;
    ordered[index] = displaced;
    applyResult(reorderMultiFillBlanks(props.source(), ordered), props.onEdit, props.onStatus);
  }

  function applyNumeric(
    tolerance: PleQuestionJsonNumericResponseTolerance,
    unit: string | null,
  ): void {
    const response = numeric();
    if (response === null) return;
    const next = numericResponseFromAuthoring(
      response,
      props.numericAnswerLiteral(),
      tolerance,
      unit,
    );
    if (next !== null) editResponse(next);
  }

  function applyHotspot(
    edit: (response: PleQuestionJsonHotspotResponse) => {
      readonly response: PleQuestionJsonHotspotResponse;
      readonly changed: boolean;
      readonly error: string | null;
    },
  ): void {
    const response = props.hotspotResponse();
    if (response === null) return;
    const next = edit(response);
    if (next.changed) editResponse(next.response);
    else if (next.error !== null) props.onStatus(next.error);
  }

  return (
    <>
      <Show when={props.selectedKind() === "multipleAnswer" && multipleAnswer()}>
        {(response) => (
          <PleQuestionJsonMultipleAnswerEditor
            response={response}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onChoiceTextChange={(id, text) => {
              const next = setMultipleAnswerChoiceText(response(), id, text);
              if (next.changed) editResponse(next.response);
            }}
            onChoiceFeedbackChange={(id, feedback) => {
              const next = setMultipleAnswerChoiceFeedback(response(), id, feedback);
              if (next.changed) editResponse(next.response);
            }}
            onCorrectChoiceChange={(id, correct) => {
              const next = setMultipleAnswerCorrect(response(), id, correct);
              if (next.changed) editResponse(next.response);
            }}
            onAddChoice={() => {
              const next = addMultipleAnswerChoice(response());
              if (next.changed) editResponse(next.response);
            }}
            onRemoveChoice={(id) => {
              const next = removeMultipleAnswerChoice(response(), id);
              if (next.changed) editResponse(next.response);
            }}
            onMoveChoice={(id, direction) => {
              const next = moveMultipleAnswerChoice(response(), id, direction);
              if (next.changed) editResponse(next.response);
            }}
          />
        )}
      </Show>
      <Show when={props.selectedKind() === "fillIn" && fillIn()}>
        {(response) => (
          <PleQuestionJsonFillInEditor
            response={response}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onResponseChange={editResponse}
          />
        )}
      </Show>
      <Show when={props.selectedKind() === "multiFillIn" && multiFillIn()}>
        {(response) => (
          <PleQuestionJsonMultiFillInEditor
            blanks={() => response().blanks}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onStatus={props.onStatus}
            onBlankLabelChange={(id, label) =>
              applyResult(
                setMultiFillBlankLabel(props.source(), id, label),
                props.onEdit,
                props.onStatus,
              )
            }
            onBlankMatchModeChange={(id, mode) =>
              applyResult(
                setMultiFillBlankMatchMode(props.source(), id, mode),
                props.onEdit,
                props.onStatus,
              )
            }
            onBlankMaxLengthChange={(id, length) =>
              applyResult(
                setMultiFillBlankMaxLength(props.source(), id, length),
                props.onEdit,
                props.onStatus,
              )
            }
            onAnswerChange={(id, index, answer) =>
              applyResult(
                setMultiFillBlankAnswer(props.source(), id, index, answer),
                props.onEdit,
                props.onStatus,
              )
            }
            onAddAnswer={(id) =>
              applyResult(addMultiFillBlankAnswer(props.source(), id), props.onEdit, props.onStatus)
            }
            onRemoveAnswer={(id, index) =>
              applyResult(
                removeMultiFillBlankAnswer(props.source(), id, index),
                props.onEdit,
                props.onStatus,
              )
            }
            onAddBlank={() =>
              applyResult(addMultiFillBlank(props.source()), props.onEdit, props.onStatus)
            }
            onRemoveBlank={(id) =>
              applyResult(removeMultiFillBlank(props.source(), id), props.onEdit, props.onStatus)
            }
            onMoveBlank={moveBlank}
          />
        )}
      </Show>
      <Show when={props.selectedKind() === "numeric" && numeric()}>
        {(response) => (
          <PleQuestionJsonNumericEditor
            answerLiteral={props.numericAnswerLiteral()}
            tolerance={() => response().tolerance}
            unit={() => response().unit}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onAnswerLiteralChange={props.onNumericAnswerLiteralChange}
            onToleranceChange={(tolerance) => applyNumeric(tolerance, response().unit)}
            onUnitChange={(unit) => applyNumeric(response().tolerance, unit)}
          />
        )}
      </Show>
      <Show when={props.selectedKind() === "ordering" && ordering()}>
        {(response) => (
          <PleQuestionJsonOrderingEditor
            items={() => response().items}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onStatus={props.onStatus}
            onItemTextChange={(id, text) =>
              applyResult(
                setOrderingItemText(props.source(), id, text),
                props.onEdit,
                props.onStatus,
              )
            }
            onAddItem={() =>
              applyResult(addOrderingItem(props.source()), props.onEdit, props.onStatus)
            }
            onRemoveItem={(id) =>
              applyResult(removeOrderingItem(props.source(), id), props.onEdit, props.onStatus)
            }
            onMoveItem={(id, direction) =>
              applyResult(
                moveOrderingItem(props.source(), id, direction),
                props.onEdit,
                props.onStatus,
              )
            }
          />
        )}
      </Show>
      <Show when={props.selectedKind() === "hotspot"}>
        <PleQuestionJsonHotspotEditor
          response={props.hotspotResponse}
          assetClient={props.assetClient}
          workspace={props.workspace}
          pendingDescription={props.pendingHotspotDescription}
          fieldErrors={props.fieldErrors}
          disabled={props.disabled}
          onSelectAsset={props.onSelectHotspotAsset}
          onPendingDescriptionChange={props.onPendingHotspotDescriptionChange}
          onDescriptionChange={(description) =>
            applyHotspot((response) => setHotspotDescription(response, description))
          }
          onRegionLabelChange={(regionId, label) =>
            applyHotspot((response) => setHotspotRegionLabel(response, regionId, label))
          }
          onRegionCoordinateChange={(regionId, coordinate, value) =>
            applyHotspot((response) =>
              setHotspotRegionCoordinate(response, regionId, coordinate, value),
            )
          }
          onCorrectChange={(regionId, correct) =>
            applyHotspot((response) => setHotspotCorrectRegion(response, regionId, correct))
          }
          onAddRegion={() => applyHotspot(addHotspotRegion)}
          onRemoveRegion={(regionId) =>
            applyHotspot((response) => removeHotspotRegion(response, regionId))
          }
          onMoveRegion={(regionId, direction) =>
            applyHotspot((response) => moveHotspotRegion(response, regionId, direction))
          }
          onStatus={props.onStatus}
        />
      </Show>
    </>
  );
}
