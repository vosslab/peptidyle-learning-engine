// student_assignment_attempt_navigation.tsx - answer-free Student Question position controls.

import { For, Show, type JSX } from "solid-js";

import "./student_assignment_attempt_navigation.css";

import {
  studentAssignmentAttemptQuestionStateLabel,
  type StudentAssignmentAttemptQuestionState,
} from "./student_assignment_attempt_navigation_model";

export type { StudentAssignmentAttemptQuestionState } from "./student_assignment_attempt_navigation_model";

export interface StudentAssignmentAttemptQuestionPosition {
  readonly position: number;
  readonly responseState: StudentAssignmentAttemptQuestionState;
}

export interface StudentAssignmentAttemptNavigationProps {
  /** Ordered, answer-free positions from the authorized Assignment Attempt. */
  readonly positions: ReadonlyArray<StudentAssignmentAttemptQuestionPosition>;
  readonly currentPosition: number | null;
  readonly onPositionActivate: (position: number) => void;
}

/**
 * Presents every Question position without exposing library titles, identifiers, or responses.
 * Page-level loading, saving, and route transitions remain with the Assignment Attempt surface.
 */
export function StudentAssignmentAttemptNavigation(
  props: StudentAssignmentAttemptNavigationProps,
): JSX.Element {
  return (
    <nav class="student-assignment-question-navigation" aria-label="Assignment questions">
      <Show
        when={props.positions.length > 0}
        fallback={
          <p class="student-assignment-question-navigation-empty">
            Questions will appear here when ready.
          </p>
        }
      >
        <ol class="student-assignment-question-navigation-list">
          <For each={props.positions}>
            {(question) => {
              const isCurrent = (): boolean => question.position === props.currentPosition;
              const isClosed = (): boolean => question.responseState === "closed";
              const label = (): string =>
                `Question ${question.position}: ${studentAssignmentAttemptQuestionStateLabel(question.responseState)}${
                  isCurrent() ? ", current" : ""
                }`;
              return (
                <li>
                  <button
                    type="button"
                    classList={{
                      "student-assignment-question-navigation-entry": true,
                      "is-current": isCurrent(),
                    }}
                    aria-current={isCurrent() ? "step" : undefined}
                    aria-label={label()}
                    disabled={isClosed()}
                    onClick={() => props.onPositionActivate(question.position)}
                  >
                    <span>Question {question.position}</span>
                    <span class="student-assignment-question-navigation-state">
                      {studentAssignmentAttemptQuestionStateLabel(question.responseState)}
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
