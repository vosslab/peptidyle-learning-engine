// assignment_workspace_overview_page.tsx - compact Instructor assignment home.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import { assignmentWorkspacePath } from "./assignment_workspace_nav";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";

function stateCopy(state: string): string {
  return state
    .replace(/([a-z])([A-Z])/gu, "$1 $2")
    .replace(/^./u, (letter) => letter.toUpperCase());
}

function localTime(value: string | null): string {
  return value === null ? "Not set" : value.replace("T", " ").replace(/\.000$/u, "");
}

export function AssignmentWorkspaceOverviewPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const assignment = workspace.assignment;
  const fixedCount = (): number =>
    assignment().entries.filter(
      (entry) => entry.kind === "fixedQuestion" && entry.availability === "available",
    ).length;
  const poolCount = (): number =>
    assignment().entries.filter((entry) => entry.kind === "questionPool").length;
  const questionPoolItemCount = (): number =>
    assignment().entries.reduce(
      (total, entry) => total + (entry.kind === "questionPool" ? entry.items.length : 0),
      0,
    );
  const base = (): string =>
    assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference);

  return (
    <section class="assignment-workspace-overview" aria-labelledby="assignment-workspace-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment overview</p>
        <h1 id="assignment-workspace-heading">{assignment().title}</h1>
        <p class="page-lede">
          {workspace.course.title} {"\u00b7"} Edit {assignment().revision}
        </p>
      </header>

      <div class="assignment-workspace-grid">
        <section
          class="course-card assignment-workspace-card"
          aria-labelledby="assignment-status-heading"
        >
          <h2 id="assignment-status-heading">Current status</h2>
          <dl class="assignment-facts">
            <div>
              <dt>Assignment status</dt>
              <dd>{stateCopy(assignment().assignmentStatus)}</dd>
            </div>
            <div>
              <dt>Assignment availability</dt>
              <dd>{stateCopy(assignment().assignmentAvailability.state)}</dd>
            </div>
            <div>
              <dt>Questions</dt>
              <dd>{fixedCount()}</dd>
            </div>
            <div>
              <dt>Pools</dt>
              <dd>
                {poolCount()} ({questionPoolItemCount()} items)
              </dd>
            </div>
          </dl>
        </section>

        <section
          class="course-card assignment-workspace-card"
          aria-labelledby="assignment-readiness-heading"
        >
          <h2 id="assignment-readiness-heading">Release requirements</h2>
          <Show
            when={assignment().assignmentReleaseValidation.blockingIssues.length === 0}
            fallback={
              <>
                <p role="status">This Assignment is not ready to release.</p>
                <ul class="assignment-workspace-next-actions">
                  <For each={assignment().assignmentReleaseValidation.blockingIssues}>
                    {(issue) => (
                      <li>
                        {issue.kind === "questionsRequired" ? (
                          <A
                            href={assignmentWorkspacePath(
                              workspace.courseReference,
                              workspace.assignmentReference,
                              "questions",
                            )}
                          >
                            Add at least one question
                          </A>
                        ) : (
                          "Review the assignment settings"
                        )}
                      </li>
                    )}
                  </For>
                </ul>
              </>
            }
          >
            <p role="status">The current Assignment meets the known release requirements.</p>
          </Show>
          <p class="assignment-workspace-action-row">
            <A
              class="primary-link"
              href={assignmentWorkspacePath(
                workspace.courseReference,
                workspace.assignmentReference,
                "questions",
              )}
            >
              Review questions
            </A>
            <A
              class="quiet-link"
              href={assignmentWorkspacePath(
                workspace.courseReference,
                workspace.assignmentReference,
                "policies",
              )}
            >
              Review policies
            </A>
          </p>
        </section>

        <section
          class="course-card assignment-workspace-card"
          aria-labelledby="assignment-instructions-heading"
        >
          <h2 id="assignment-instructions-heading">Instructions and delivery</h2>
          <Show
            when={assignment().assignmentAuthoredContent.instructions.length > 0}
            fallback={<p>No Student instructions have been added.</p>}
          >
            <p class="plain-text-instructions">
              {assignment().assignmentAuthoredContent.instructions}
            </p>
          </Show>
          <dl class="assignment-facts">
            <div>
              <dt>Time zone</dt>
              <dd>{assignment().assignmentAuthoredContent.timeZone}</dd>
            </div>
            <div>
              <dt>Available</dt>
              <dd>{localTime(assignment().assignmentAuthoredContent.available_at)}</dd>
            </div>
            <div>
              <dt>Due</dt>
              <dd>{localTime(assignment().assignmentAuthoredContent.due_at)}</dd>
            </div>
            <div>
              <dt>Closes</dt>
              <dd>{localTime(assignment().assignmentAuthoredContent.closes_at)}</dd>
            </div>
          </dl>
        </section>
      </div>

      <section
        class="assignment-workspace-contextual-actions"
        aria-labelledby="assignment-tools-heading"
      >
        <h2 id="assignment-tools-heading">Related teaching tools</h2>
        <A class="quiet-link" href={`${base()}/access`}>
          Access and accommodations
        </A>
        <A class="quiet-link" href={`${base()}/delivery-check`}>
          Check assignment delivery
        </A>
        <A
          class="quiet-link"
          href={assignmentWorkspacePath(
            workspace.courseReference,
            workspace.assignmentReference,
            "studentView",
          )}
        >
          Open Student view
        </A>
      </section>
    </section>
  );
}
