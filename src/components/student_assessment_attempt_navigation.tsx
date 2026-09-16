// student_assessment_attempt_navigation.tsx - answer-free Student Question position controls.

import { createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";

import "./student_assessment_attempt_navigation.css";

import {
  studentAssessmentAttemptQuestionStateLabel,
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
    const count = props.positions.length;
    const slots = numberSlots();
    if (count <= slots) return props.positions;
    // Reserve first/last numbers and room for both omitted-range markers.
    const rangeSize = Math.max(1, slots - 4);
    const start = Math.max(
      1,
      Math.min(currentIndex() - Math.floor(rangeSize / 2), count - rangeSize - 1),
    );
    return props.positions.filter(
      (_question, index) =>
        index === 0 || index === count - 1 || (index >= start && index < start + rangeSize),
    );
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
  onMount(() => {
    if (!navigation) return;
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      // Enlarged text reduces the visible range as well as narrower containers.
      const fontSize = Number.parseFloat(getComputedStyle(entry.target).fontSize);
      setNumberSlots(
        Math.max(5, Math.floor((entry.contentRect.width - 9 * fontSize) / (3.1 * fontSize))),
      );
    });
    observer.observe(navigation);
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
