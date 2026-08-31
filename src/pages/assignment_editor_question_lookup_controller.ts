import { createSignal, type Accessor } from "solid-js";

import type { AssignmentCatalogRow } from "./assignment_editor_model";
import { parseExactProblemDisplayReferences } from "./assignment_editor_model";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";

export interface AssignmentEditorQuestionLookupController {
  readonly replacementText: Accessor<string>;
  readonly setReplacementText: (value: string) => void;
  readonly selected: Accessor<AssignmentCatalogRow | undefined>;
  readonly setSelected: (value: AssignmentCatalogRow | undefined) => void;
  readonly lookup: (value: string) => Promise<AssignmentCatalogRow>;
  readonly chooseReplacement: (onMessage: (message: string) => void) => Promise<void>;
}

export function createAssignmentEditorQuestionLookupController(
  repository: AssignmentEditorRepository,
): AssignmentEditorQuestionLookupController {
  const [replacementText, setReplacementText] = createSignal("");
  const [selected, setSelected] = createSignal<AssignmentCatalogRow>();

  async function lookup(value: string): Promise<AssignmentCatalogRow> {
    const ids = parseExactProblemDisplayReferences(value);
    if (ids.length !== 1) throw new Error("Choose one Question ID for this action.");
    const id = ids[0];
    if (id === undefined) throw new Error("Choose one Question ID for this action.");
    return await repository.resolvePublished(id);
  }

  async function chooseReplacement(onMessage: (message: string) => void): Promise<void> {
    try {
      const row = await lookup(replacementText());
      setSelected(row);
      onMessage(`${row.questionId} is ready to replace the selected assignment question.`);
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
