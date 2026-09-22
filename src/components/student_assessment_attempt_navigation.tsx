// student_assessment_attempt_navigation.tsx - answer-free Student Question position controls.

import { createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";

import "./student_assessment_attempt_navigation.css";

import {
  studentAssessmentAttemptQuestionStateLabel,
  studentAssessmentAttemptVisiblePositionIndexes,
  type StudentAssessmentAttemptQuestionState,
} from "./student_assessment_attempt_navigation_model";

export type { StudentAssessmentAttemptQuestionState } from "./student_assessment_attempt_navigation_model";

export interface StudentAssessmentAttemptQuestionPosition {
  readonly position: number;
  readonly responseState: StudentAssessmentAttemptQuestionState;
}

export interface StudentAssessmentAttemptNavigationProps {
  /** Ordered, answer-free positions from the authorized Assessment Attempt. */
  readonly positions: ReadonlyArray<StudentAssessmentAttemptQuestionPosition>;
  readonly currentPosition: number | null;
  readonly onPositionActivate: (position: number) => void;
}

/**
 * Presents every Question position without exposing library titles, identifiers, or responses.
 * Page-level loading, saving, and route transitions remain with the Assessment Attempt surface.
 */
export function StudentAssessmentAttemptNavigation(
  props: StudentAssessmentAttemptNavigationProps,
): JSX.Element {
  let navigation: HTMLElement | undefined;
  const [numberSlots, setNumberSlots] = createSignal(5);
  const currentIndex = createMemo(() =>
    props.positions.findIndex((question) => question.position === props.currentPosition),
  );
  const visiblePositions = createMemo(() => {
    const indexes = studentAssessmentAttemptVisiblePositionIndexes(
      props.positions.length,
      currentIndex(),
      numberSlots(),
    );
    return indexes.flatMap((index) => {
      const question = props.positions[index];
      return question === undefined ? [] : [question];
    });
  });
  const previous = (): StudentAssessmentAttemptQuestionPosition | undefined =>
    props.positions[currentIndex() - 1];
  const next = (): StudentAssessmentAttemptQuestionPosition | undefined =>
    props.positions[currentIndex() + 1];
  function hasGapBefore(
    question: StudentAssessmentAttemptQuestionPosition,
    index: number,
  ): boolean {
    const before = visiblePositions()[index - 1];
    return (
      before !== undefined &&
      props.positions.indexOf(question) - props.positions.indexOf(before) > 1
    );
  }
  const activate = (question: StudentAssessmentAttemptQuestionPosition | undefined): void => {
    if (question && question.responseState !== "closed") {
      props.onPositionActivate(question.position);
    }
  };
  function measureNumberSlots(element: HTMLElement): number {
    const row = element.querySelector<HTMLElement>(".student-assessment-question-navigation-row");
    const entry = element.querySelector<HTMLElement>(
      ".student-assessment-question-navigation-entry",
    );
    if (row === null || entry === null) return 5;
    const rowStyle = getComputedStyle(row);
    const list = element.querySelector<HTMLElement>(".student-assessment-question-navigation-list");
    const listGap = list === null ? 0 : Number.parseFloat(getComputedStyle(list).columnGap) || 0;
    const rowGap = Number.parseFloat(rowStyle.columnGap) || 0;
    const moveWidth = [
      ...element.querySelectorAll<HTMLElement>(".student-assessment-question-navigation-move"),
    ].reduce((total, button) => total + button.getBoundingClientRect().width, 0);
    const numberWidth = entry.getBoundingClientRect().width;
    const markerWidth = Math.max(numberWidth * 0.35, Number.parseFloat(rowStyle.fontSize) || 16);
    const available = element.getBoundingClientRect().width - moveWidth - rowGap * 2;
    // Reserve the two possible omitted markers while keeping the measured
    // number-button width as the source of the range budget.
    return Math.max(
      5,
      Math.floor((available - markerWidth * 2 - listGap * 2 + listGap) / (numberWidth + listGap)),
    );
  }
  onMount(() => {
    if (!navigation) return;
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      // Enlarged text and localized movement labels reduce the visible range.
      if (entry.target instanceof HTMLElement) setNumberSlots(measureNumberSlots(entry.target));
    });
    observer.observe(navigation);
    queueMicrotask(() => setNumberSlots(measureNumberSlots(navigation as HTMLElement)));
    onCleanup(() => observer.disconnect());
  });
  return (
    <nav
      ref={(element) => {
        navigation = element;
      }}
      class="student-assessment-question-navigation"
      aria-label="Assessment questions"
    >
      <Show
        when={props.positions.length > 0}
        fallback={
          <p class="student-assessment-question-navigation-empty">
            Questions will appear here when ready.
          </p>
        }
      >
        <div class="student-assessment-question-navigation-row">
          <button
            type="button"
            class="student-assessment-question-navigation-move"
            aria-label="Previous question"
            disabled={!previous() || previous()?.responseState === "closed"}
            onClick={() => activate(previous())}
          >
            Prev
          </button>
          <ol class="student-assessment-question-navigation-list">
            <For each={visiblePositions()}>
              {(question, index) => {
                const isCurrent = (): boolean => question.position === props.currentPosition;
                const isClosed = (): boolean => question.responseState === "closed";
                const label = (): string =>
                  `Question ${question.position}: ${studentAssessmentAttemptQuestionStateLabel(question.responseState)}${
                    isCurrent() ? ", current" : ""
                  }`;
                return (
                  <>
                    <Show when={hasGapBefore(question, index())}>
                      <li class="student-assessment-question-navigation-gap" aria-hidden="true">
                        &hellip;
                      </li>
                    </Show>
                    <li>
                      <button
                        type="button"
                        classList={{
                          "student-assessment-question-navigation-entry": true,
                          "is-current": isCurrent(),
                          "is-saved": question.responseState === "saved",
                        }}
                        aria-current={isCurrent() ? "step" : undefined}
                        aria-label={label()}
                        title={label()}
                        disabled={isClosed()}
                        onClick={() => props.onPositionActivate(question.position)}
                      >
                        <span aria-hidden="true">{question.position}</span>
                        <span
                          class="student-assessment-question-navigation-state"
                          aria-hidden="true"
                        >
                          <Show
                            when={question.responseState === "saved"}
                            fallback={<span>&nbsp;</span>}
                          >
                            <span>&#10003;</span>
                          </Show>
                        </span>
                      </button>
                    </li>
                  </>
                );
              }}
            </For>
          </ol>
          <button
            type="button"
            class="student-assessment-question-navigation-move"
            aria-label="Next question"
            disabled={!next() || next()?.responseState === "closed"}
            onClick={() => activate(next())}
          >
            Next
          </button>
        </div>
        <p class="student-assessment-question-navigation-summary">
          <span>
            Question {props.currentPosition ?? "-"} of {props.positions.length}
          </span>
          <span>
            {" "}
            &middot;{" "}
            {props.positions.filter((question) => question.responseState === "saved").length} saved
          </span>
          <span class="student-assessment-question-navigation-legend">
            {" "}
            &middot; &#10003; Saved
          </span>
        </p>
      </Show>
    </nav>
  );
}
