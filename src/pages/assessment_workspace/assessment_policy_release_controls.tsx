import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type {
  AssessmentReleaseValidation,
  AssessmentUnreleaseImpact,
  LiveAssessmentWorkspace,
} from "../../api/assessment_release";
import type { BaseAssessmentPolicyPersistence } from "./base_assessment_policy_autosave_model";

type AssessmentPolicyReleaseControlsProps = {
  readonly persistence: BaseAssessmentPolicyPersistence;
  readonly status: LiveAssessmentWorkspace["status"];
  readonly persisted: boolean;
  readonly busy: boolean;
  readonly needsReload: boolean;
  readonly validation: AssessmentReleaseValidation | undefined;
  readonly unreleaseImpact: AssessmentUnreleaseImpact | undefined;
  readonly confirmationTitle: string;
  readonly questionsPath: string;
  readonly studentViewPath: string;
  readonly onRetry: () => void;
  readonly onValidateRelease: () => Promise<void>;
  readonly onRelease: () => Promise<void>;
  readonly onReload: () => Promise<boolean>;
  readonly onConfirmationTitleChange: (title: string) => void;
  readonly onUnrelease: () => Promise<void>;
};

/** Presents release readiness and destructive Unrelease; the page owns every action. */
export function AssessmentPolicyReleaseControls(
  props: AssessmentPolicyReleaseControlsProps,
): JSX.Element {
  return (
    <>
      <p class="assessment-editor-actions">
        <Show when={props.persistence === "failed"}>
          <button class="primary-action" type="button" onClick={props.onRetry}>
            Retry
          </button>
        </Show>
        <Show when={props.status === "unreleased"}>
          <button
            type="button"
            disabled={!props.persisted || props.busy}
            onClick={() => void props.onValidateRelease()}
          >
            {props.busy ? "Checking release readiness..." : "Check release readiness"}
          </button>
          <button
            class="primary-action"
            type="button"
            disabled={!props.persisted || props.busy}
            onClick={() => void props.onRelease()}
          >
            Release assessment
          </button>
        </Show>
        <Show when={props.persistence === "conflict"}>
          <button type="button" onClick={() => void props.onReload()}>
            Reload server state
          </button>
        </Show>
        <A class="quiet-link" href={props.questionsPath}>
          Edit Questions
        </A>
        <A class="quiet-link" href={props.studentViewPath}>
          Open Student View
        </A>
      </p>
      <Show when={props.validation}>
        {(validation) => (
          <section
            class="assessment-workspace-release-readiness"
            aria-labelledby="assessment-release-readiness-heading"
            role={validation().canRelease ? undefined : "alert"}
          >
            <h2 id="assessment-release-readiness-heading">
              {validation().canRelease ? "Ready to release" : "Release needs attention"}
            </h2>
            <Show
              when={!validation().canRelease}
              fallback={<p>All current release requirements are met.</p>}
            >
              <ul>
                <For each={validation().poolIssues}>
                  {(poolIssue) => (
                    <li>
                      Assessment position {poolIssue.assessmentPosition}: Question Pool "
                      {poolIssue.poolTitle}" ({poolIssue.questionPoolId}) requests{" "}
                      {poolIssue.selectionCount} Question
                      {poolIssue.selectionCount === 1 ? "" : "s"};{" "}
                      {poolIssue.issue === "insufficientItems"
                        ? "it has too few valid Questions."
                        : poolIssue.issue === "memberUnavailable"
                          ? "a member Question or its Revision metadata is unavailable."
                          : poolIssue.issue === "memberBackendMismatch"
                            ? "a member Question uses an unsupported or different Backend."
                            : poolIssue.issue === "memberTypeMismatch"
                              ? "a member Question has a different Type."
                              : "a member Question has a different Discipline or Subject."}
                    </li>
                  )}
                </For>
                <For
                  each={validation().issues.filter(
                    (issue) =>
                      issue !== "questionUnavailable" || validation().poolIssues.length === 0,
                  )}
                >
                  {(issue) => (
                    <li>
                      {issue === "noPublishedQuestions"
                        ? "Select and save at least one published Question."
                        : issue === "questionCountExceeded"
                          ? "Reduce the Assessment to at most 250 delivered Questions, counting each Pool's selected Questions."
                          : issue === "questionUnavailable"
                            ? "A selected Question is unavailable. Review and save the Questions list."
                            : issue === "dueDateRequired"
                              ? "Enter a Due date, save, then check release readiness again."
                              : issue === "dueDateLessThan24HoursAhead"
                                ? "Move the Due date to at least 24 hours from now, save, then check release readiness again."
                                : issue === "dueDateAfterCourseActiveUntil"
                                  ? "Move the Due date no later than the Course Active limit, save, then check release readiness again."
                                  : issue === "availabilityAfterDueDate"
                                    ? "Move Available to no later than the Due date, save, then check release readiness again."
                                    : "Move Closes to the Due date or later, save, then check release readiness again."}
                    </li>
                  )}
                </For>
              </ul>
            </Show>
            <Show when={!validation().canRelease}>
              <A class="quiet-link" href={props.questionsPath}>
                Review assessment Questions
              </A>
            </Show>
          </section>
        )}
      </Show>
      <Show when={props.status === "released" && props.unreleaseImpact}>
        {(impact) => (
          <section
            class="assessment-workspace-unrelease-danger-zone"
            aria-labelledby="assessment-unrelease-heading"
          >
            <h2 id="assessment-unrelease-heading">Danger Zone: Unrelease assessment</h2>
            <p>
              Unreleasing makes this Assessment unavailable to Students and permanently deletes its
              Student Work. The current Assessment definition remains available for later editing
              and release.
            </p>
            <dl aria-label="Unrelease deletion impact">
              <div>
                <dt>Attempts</dt>
                <dd>{impact().attemptCount}</dd>
              </div>
              <div>
                <dt>Submissions</dt>
                <dd>{impact().submissionCount}</dd>
              </div>
              <div>
                <dt>Grades</dt>
                <dd>{impact().gradeCount}</dd>
              </div>
            </dl>
            <label class="assessment-editor-field">
              Type <strong>{impact().confirmationTitle}</strong> to confirm
              <input
                aria-describedby="assessment-unrelease-confirmation-help"
                autocomplete="off"
                value={props.confirmationTitle}
                onInput={(event) => props.onConfirmationTitleChange(event.currentTarget.value)}
              />
            </label>
            <p id="assessment-unrelease-confirmation-help">
              Unrelease permanently deletes the Student Work represented by these counts. No Student
              names, responses, or grade details are displayed here.
            </p>
            <button
              class="assessment-workspace-danger-action"
              type="button"
              disabled={props.needsReload || props.confirmationTitle !== impact().confirmationTitle}
              onClick={() => void props.onUnrelease()}
            >
              {props.busy ? "Unreleasing assessment..." : "Unrelease and delete Student Work"}
            </button>
          </section>
        )}
      </Show>
    </>
  );
}
