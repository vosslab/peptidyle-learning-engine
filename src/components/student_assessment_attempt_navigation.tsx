// student_assessment_attempt_navigation.tsx - answer-free Student Question position controls.

import { For, Show, type JSX } from "solid-js";

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
  return (
    <nav class="student-assessment-question-navigation" aria-label="Assessment questions">
      <Show
        when={props.positions.length > 0}
        fallback={
          <p class="student-assessment-question-navigation-empty">
            Questions will appear here when ready.
          </p>
        }
      >
        <ol class="student-assessment-question-navigation-list">
          <For each={props.positions}>
            {(question) => {
              const isCurrent = (): boolean => question.position === props.currentPosition;
              const isClosed = (): boolean => question.responseState === "closed";
              const label = (): string =>
                `Question ${question.position}: ${studentAssessmentAttemptQuestionStateLabel(question.responseState)}${
                  isCurrent() ? ", current" : ""
                }`;
              return (
                <li>
                  <button
                    type="button"
                    classList={{
                      "student-assessment-question-navigation-entry": true,
                      "is-current": isCurrent(),
                    }}
                    aria-current={isCurrent() ? "step" : undefined}
                    aria-label={label()}
                    disabled={isClosed()}
                    onClick={() => props.onPositionActivate(question.position)}
                  >
                    <span>Question {question.position}</span>
                    <span class="student-assessment-question-navigation-state">
                      {studentAssessmentAttemptQuestionStateLabel(question.responseState)}
                      <Show when={isCurrent()}>
                        <span aria-hidden="true"> · Current</span>
                      </Show>
                    </span>
                  </button>
                </li>
              );
            }}
          </For>
        </ol>
      </Show>
    </nav>
  );
}
