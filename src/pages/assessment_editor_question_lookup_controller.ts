import { createSignal, type Accessor } from "solid-js";

import type { AssessmentQuestionRow } from "./assessment_editor_model";
import { parseExactQuestionIds } from "./assessment_editor_model";
import type { AssessmentEditorRepository } from "./assessment_editor_repository";

export interface AssessmentEditorQuestionLookupController {
  readonly replacementText: Accessor<string>;
  readonly setReplacementText: (value: string) => void;
  readonly selected: Accessor<AssessmentQuestionRow | undefined>;
  readonly setSelected: (value: AssessmentQuestionRow | undefined) => void;
  readonly lookup: (value: string) => Promise<AssessmentQuestionRow>;
  readonly chooseReplacement: (onMessage: (message: string) => void) => Promise<void>;
}

export function createAssessmentEditorQuestionLookupController(
  repository: AssessmentEditorRepository,
): AssessmentEditorQuestionLookupController {
  const [replacementText, setReplacementText] = createSignal("");
  const [selected, setSelected] = createSignal<AssessmentQuestionRow>();

  async function lookup(value: string): Promise<AssessmentQuestionRow> {
    const ids = parseExactQuestionIds(value);
    if (ids.length !== 1) throw new Error("Choose one Question ID for this action.");
    const id = ids[0];
    if (id === undefined) throw new Error("Choose one Question ID for this action.");
    return await repository.resolvePublished(id);
  }

  async function chooseReplacement(onMessage: (message: string) => void): Promise<void> {
    try {
      const row = await lookup(replacementText());
      setSelected(row);
      onMessage(`${row.questionId} is ready to replace the selected assessment question.`);
    } catch (error: unknown) {
      onMessage(error instanceof Error ? error.message : "That Question ID could not be found.");
    }
  }

  return {
    replacementText,
    setReplacementText,
    selected,
    setSelected,
    lookup,
    chooseReplacement,
  };
}
