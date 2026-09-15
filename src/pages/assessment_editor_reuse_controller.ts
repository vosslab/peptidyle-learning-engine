// assessment_editor_reuse_controller.ts - retained Assessment Question selection state.

import { createSignal, type Accessor } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import type { AssessmentId } from "../../generated/api/AssessmentId";

import type {
  AssessmentEditorRepository,
  RetainedAssessmentQuestionSource,
} from "./assessment_editor_repository";

export interface AssessmentEditorRetainedAssessmentController {
  readonly sources: Accessor<ReadonlyArray<RetainedAssessmentQuestionSource>>;
  readonly message: Accessor<string>;
  readonly sourceIndex: Accessor<number | undefined>;
  readonly questionIndexes: Accessor<ReadonlySet<number>>;
  readonly selectedSource: () => RetainedAssessmentQuestionSource | undefined;
  readonly load: () => Promise<void>;
  readonly chooseSource: (index: number) => void;
  readonly toggleQuestion: (index: number, checked: boolean) => void;
}

export function createAssessmentEditorRetainedAssessmentController(
  repository: AssessmentEditorRepository,
  courseId: CourseId,
  exclude?: AssessmentId,
): AssessmentEditorRetainedAssessmentController {
  const [sources, setSources] = createSignal<ReadonlyArray<RetainedAssessmentQuestionSource>>([]);
  const [message, setMessage] = createSignal("");
  const [sourceIndex, setSourceIndex] = createSignal<number>();
  const [questionIndexes, setQuestionIndexes] = createSignal<ReadonlySet<number>>(new Set());

  function selectedSource(): RetainedAssessmentQuestionSource | undefined {
    const index = sourceIndex();
    return index === undefined ? undefined : sources()[index];
  }

  async function load(): Promise<void> {
    try {
      const values = await repository.listRetainedAssessmentQuestionSources(courseId, exclude);
      setSources(values);
      setSourceIndex(values.length > 0 ? 0 : undefined);
      setQuestionIndexes(new Set(values[0]?.questions.map((_question, index) => index) ?? []));
    } catch {
      setMessage("Existing assessments could not be loaded. Your current work is unchanged.");
    }
  }

  function chooseSource(index: number): void {
    const source = sources()[index];
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
    sources,
    message,
    sourceIndex,
    questionIndexes,
    selectedSource,
    load,
    chooseSource,
    toggleQuestion,
  };
}
