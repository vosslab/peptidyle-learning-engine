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
  const fixedCount = () =>
    assignment().items.filter((item) => item.deliveryState === "active").length;
  const poolCount = () => assignment().selectionGroups.length;
  const candidateCount = () =>
    assignment().selectionGroups.reduce((total, group) => total + group.candidates.length, 0);
  const base = () =>
    assignmentWorkspacePath(workspace.courseReference, workspace.assignmentReference);

  return (
    <section class="assignment-workspace-overview" aria-labelledby="assignment-workspace-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment overview</p>
        <h1 id="assignment-workspace-heading">{assignment().title}</h1>
        <p class="page-lede">
          {workspace.course.title} · Revision {assignment().revision}
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
              <dt>Lifecycle</dt>
              <dd>{stateCopy(assignment().teachingSettings.lifecycle)}</dd>
            </div>
            <div>
              <dt>Current state</dt>
              <dd>{stateCopy(assignment().currentState.state)}</dd>
            </div>
            <div>
              <dt>Questions</dt>
              <dd>{fixedCount()}</dd>
            </div>
            <div>
              <dt>Pools</dt>
              <dd>
                {poolCount()} ({candidateCount()} candidates)
              </dd>
            </div>
          </dl>
        </section>

        <section
          class="course-card assignment-workspace-card"
          aria-labelledby="assignment-readiness-heading"
        >
          <h2 id="assignment-readiness-heading">Publication readiness</h2>
          <Show
            when={assignment().publicationReadiness.blockingIssues.length === 0}
            fallback={
              <>
                <p role="status">This assignment is not ready to publish.</p>
                <ul class="assignment-workspace-next-actions">
                  <For each={assignment().publicationReadiness.blockingIssues}>
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
            <p role="status">The current definition meets the known publication checks.</p>
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
            when={assignment().teachingSettings.instructions.length > 0}
            fallback={<p>No learner instructions have been added.</p>}
          >
            <p class="plain-text-instructions">{assignment().teachingSettings.instructions}</p>
          </Show>
          <dl class="assignment-facts">
            <div>
              <dt>Time zone</dt>
              <dd>{assignment().teachingSettings.timeZone}</dd>
            </div>
            <div>
              <dt>Available</dt>
              <dd>{localTime(assignment().teachingSettings.availableAt)}</dd>
            </div>
            <div>
              <dt>Due</dt>
              <dd>{localTime(assignment().teachingSettings.dueAt)}</dd>
            </div>
            <div>
              <dt>Closes</dt>
              <dd>{localTime(assignment().teachingSettings.closesAt)}</dd>
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
