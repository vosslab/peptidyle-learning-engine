// assignment_editor_reuse_controller.ts - reusable-assignment selection state for new assignments.

import { createSignal, type Accessor } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import type { AssignmentId } from "../../generated/api/AssignmentId";

import type {
  AssignmentEditorRepository,
  BlueprintAssignment,
} from "./assignment_editor_repository";

export interface AssignmentEditorReuseController {
  readonly reuse: Accessor<ReadonlyArray<BlueprintAssignment>>;
  readonly message: Accessor<string>;
  readonly sourceIndex: Accessor<number | undefined>;
  readonly questionIndexes: Accessor<ReadonlySet<number>>;
  readonly selectedSource: () => BlueprintAssignment | undefined;
  readonly load: () => Promise<void>;
  readonly chooseSource: (index: number) => void;
  readonly toggleQuestion: (index: number, checked: boolean) => void;
}

export function createAssignmentEditorReuseController(
  repository: AssignmentEditorRepository,
  courseId: CourseId,
  exclude?: AssignmentId,
): AssignmentEditorReuseController {
  const [reuse, setReuse] = createSignal<ReadonlyArray<BlueprintAssignment>>([]);
  const [message, setMessage] = createSignal("");
  const [sourceIndex, setSourceIndex] = createSignal<number>();
  const [questionIndexes, setQuestionIndexes] = createSignal<ReadonlySet<number>>(new Set());

  function selectedSource(): BlueprintAssignment | undefined {
    const index = sourceIndex();
    return index === undefined ? undefined : reuse()[index];
  }

  async function load(): Promise<void> {
    try {
      const values = await repository.listBlueprintAssignments(courseId, exclude);
      setReuse(values);
      setSourceIndex(values.length > 0 ? 0 : undefined);
      setQuestionIndexes(new Set(values[0]?.questions.map((_question, index) => index) ?? []));
    } catch {
      setMessage("Existing assignments could not be loaded. Your current work is unchanged.");
    }
  }

  function chooseSource(index: number): void {
    const source = reuse()[index];
    if (source === undefined) return;
    setSourceIndex(index);
    setQuestionIndexes(new Set(source.questions.map((_question, questionIndex) => questionIndex)));
  }

  function toggleQuestion(index: number, checked: boolean): void {
    setQuestionIndexes((previous) => {
      const next = new Set(previous);
      if (checked) next.add(index);
      else next.delete(index);
      return next;
    });
  }

  return {
    reuse,
    message,
    sourceIndex,
    questionIndexes,
    selectedSource,
    load,
    chooseSource,
    toggleQuestion,
  };
}
