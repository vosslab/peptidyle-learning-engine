// student_work_inspection_page.tsx - audited, solution-free Instructor inspection.

import { A, useLocation, useParams } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { StudentResponseInspection } from "../../generated/api/StudentResponseInspection";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type {
  InspectedStudentSubmission,
  InspectedStudentWorkDetail,
} from "../api/decoders/calculated_gradebook";
import { useApplicationApi } from "../api/application_api";
import { QuestionPromptRenderer } from "../components/question_renderer";
import { textFromBlocks } from "../components/question_response_controls/common";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import {
  parseAssignmentReference,
  parseCourseMembershipReference,
  parseAssignmentAttemptReference,
} from "../navigation/public_route";
import {
  inspectedStudentWorkReturnUrl,
  parseInspectedStudentWorkRouteSearch,
} from "./gradebook_navigation";
import { formatPointScore, formatScoreValue } from "../score_format";
import "./student_work_inspection_page.css";

type InspectionState =
  | { readonly kind: "loading" }
  | { readonly kind: "invalidRoute" }
  | { readonly kind: "unavailable" }
  | {
      readonly kind: "ready";
      readonly detail: InspectedStudentWorkDetail;
    };

function formatActivity(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestamp));
}

function responseItemLabel(question: QuestionPresentation, item: string): string {
  switch (question.response.kind) {
    case "multipleChoice":
      return (question.response.choices.find((choice) => choice.id === item)?.body ?? []).length ===
        0
        ? item
        : textFromBlocks(
            question.response.choices.find((choice) => choice.id === item)?.body ?? [],
          );
    case "multiBlank": {
      const blank = question.response.blanks.find((candidate) => candidate.id === item);
      return blank === undefined ? item : textFromBlocks(blank.label);
    }
    case "matching": {
      const prompt = question.response.prompts.find((candidate) => candidate.id === item);
      if (prompt !== undefined) return textFromBlocks(prompt.body);
      const choice = question.response.choices.find((candidate) => candidate.id === item);
      return choice === undefined ? item : textFromBlocks(choice.body);
    }
    case "ordering": {
      const choice = question.response.items.find((candidate) => candidate.id === item);
      return choice === undefined ? item : textFromBlocks(choice.body);
    }
    case "numeric":
    case "shortText":
    case "hotspot":
    case "externalTool":
      return item;
  }
}

function InspectedResponse(props: {
  readonly response: StudentResponseInspection;
  readonly question?: QuestionPresentation;
}): JSX.Element {
  const label = (item: string): string =>
    props.question === undefined ? item : responseItemLabel(props.question, item);
  return (
    <section class="student-work-response" aria-label="Student response">
      <h3>Student response</h3>
      <Switch>
        <Match when={props.response.kind === "numeric" ? props.response : undefined}>
          {(response) => <p>{formatScoreValue(response().value)}</p>}
        </Match>
        <Match when={props.response.kind === "shortText" ? props.response : undefined}>
          {(response) => <blockquote>{response().text}</blockquote>}
        </Match>
        <Match when={props.response.kind === "multipleChoice" ? props.response : undefined}>
          {(response) => (
            <ul>
              <For each={response().selected}>{(item) => <li>{label(item)}</li>}</For>
            </ul>
          )}
        </Match>
        <Match when={props.response.kind === "multiBlank" ? props.response : undefined}>
          {(response) => (
            <dl>
              <For each={response().answers}>
                {(answer) => (
                  <>
                    <dt>{label(answer.slot)}</dt>
                    <dd>{answer.text}</dd>
                  </>
                )}
              </For>
            </dl>
          )}
        </Match>
        <Match when={props.response.kind === "matching" ? props.response : undefined}>
          {(response) => (
            <dl>
              <For each={response().matches}>
                {(pair) => (
                  <>
                    <dt>{label(pair.prompt)}</dt>
                    <dd>{label(pair.choice)}</dd>
                  </>
                )}
              </For>
            </dl>
          )}
        </Match>
        <Match when={props.response.kind === "ordering" ? props.response : undefined}>
          {(response) => (
            <ol>
              <For each={response().order}>{(item) => <li>{label(item)}</li>}</For>
            </ol>
          )}
        </Match>
        <Match when={props.response.kind === "hotspot" ? props.response : undefined}>
          {(response) => (
            <ol>
              <For each={response().selectedRegions}>{() => <li>Selected image region</li>}</For>
            </ol>
          )}
        </Match>
        <Match when={props.response.kind === "externalTool" ? props.response : undefined}>
          <p>External-tool submission recorded.</p>
        </Match>
      </Switch>
    </section>
  );
}

function ScoringEvidence(props: { readonly submission: InspectedStudentSubmission }): JSX.Element {
  const score = (): string | undefined => {
    if (props.submission.assignmentScoringState === "recalculating") return "Recalculating";
    if (props.submission.assignmentScoringState === "failed") return "Needs Instructor attention";
    const feedback = props.submission.feedback;
    if (feedback.pointsEarned !== undefined && feedback.pointsPossible !== undefined) {
      return formatPointScore(feedback.pointsEarned, feedback.pointsPossible);
    }
    if (feedback.pointsEarned !== undefined) return formatScoreValue(feedback.pointsEarned);
    return undefined;
  };
  return (
    <section class="student-work-scoring" aria-label="Automated grading result">
      <h3>Automated grading</h3>
      <Show when={score()} fallback={<p>No score is currently available.</p>}>
        {(value) => <p class="student-work-score">{value()}</p>}
      </Show>
      <Show when={props.submission.feedback.correctness !== undefined}>
        <p>{props.submission.feedback.correctness ? "Correct" : "Not correct"}</p>
      </Show>
      <p>Scoring generation {props.submission.scoringGeneration}</p>
    </section>
  );
}

function SubmissionCard(props: {
  readonly submission: InspectedStudentSubmission;
  readonly position: number;
}): JSX.Element {
  const question = (): QuestionPresentation | undefined =>
    props.submission.evidence.kind === "issuedPresentation"
      ? props.submission.evidence.question
      : undefined;
  const runtime = useApplicationApi();
  return (
    <article class="student-work-submission">
      <header>
        <div>
          <p class="card-kicker">Question {props.position}</p>
          <h2>{question()?.title ?? "Recorded submission"}</h2>
        </div>
        <p>Submitted {formatActivity(props.submission.submittedAt)}</p>
      </header>
      <Show when={question()}>
        {(presentation) => (
          <section class="student-work-prompt" aria-label="Issued question">
            <h3>Question shown to the Student</h3>
            <QuestionPromptRenderer
              blocks={presentation().prompt}
              assetUrl={(asset) =>
                new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
              }
            />
          </section>
        )}
      </Show>
      <div class="student-work-result-grid">
        <InspectedResponse response={props.submission.response} question={question()} />
        <ScoringEvidence submission={props.submission} />
      </div>
      <details class="student-work-evidence">
        <summary>Immutable evidence</summary>
        <Show
          when={
            props.submission.evidence.kind === "issuedPresentation"
              ? props.submission.evidence
              : undefined
          }
          fallback={<p>This Question Response Control has no browser presentation.</p>}
        >
          {(evidence) => (
            <>
              <p>The server verified the exact issued presentation before releasing this view.</p>
              <dl>
                <dt>Presentation SHA-256</dt>
                <dd>
                  <code>{evidence().issuedPresentationDigest}</code>
                </dd>
                <dt>Public asset bindings</dt>
                <dd>{evidence().assetBindings.length}</dd>
              </dl>
            </>
          )}
        </Show>
      </details>
    </article>
  );
}

function StudentWorkCoursePage(props: {
  readonly courseId: CourseId;
  readonly courseReference: CourseInstanceReference;
}): JSX.Element {
  const runtime = useApplicationApi();
  const location = useLocation();
  const params = useParams<{
    membershipRef: string;
    assignmentRef: string;
    assignmentAttemptRef: string;
  }>();
  const [state, setState] = createSignal<InspectionState>({ kind: "loading" });
  let disposed = false;
  let requestSequence = 0;
  let failureHeading: HTMLHeadingElement | undefined;

  const route = createMemo(() => ({
    membership: parseCourseMembershipReference(params.membershipRef),
    assignment: parseAssignmentReference(params.assignmentRef),
    assignmentAttempt: parseAssignmentAttemptReference(params.assignmentAttemptRef),
    operation: parseInspectedStudentWorkRouteSearch(location.search),
  }));

  async function load(request: ReturnType<typeof route>): Promise<void> {
    const requestId = ++requestSequence;
    if (
      request.membership === null ||
      request.assignment === null ||
      request.assignmentAttempt === null ||
      request.operation.kind === "invalid"
    ) {
      setState({ kind: "invalidRoute" });
      return;
    }
    setState({ kind: "loading" });
    try {
      const detail = await runtime.client.getInspectedStudentWork(
        props.courseId,
        request.membership,
        request.assignment,
        request.assignmentAttempt,
        request.operation.operation,
      );
      if (disposed || requestId !== requestSequence) return;
      if (
        detail.course !== props.courseReference ||
        detail.membership !== request.membership ||
        detail.assignment !== request.assignment ||
        detail.run !== request.assignmentAttempt
      ) {
        throw new Error("Inspected work does not match the requested route");
      }
      setState({ kind: "ready", detail });
    } catch {
      if (!disposed && requestId === requestSequence) setState({ kind: "unavailable" });
    }
  }

  createEffect(() => void load(route()));
  onCleanup(() => {
    disposed = true;
    requestSequence += 1;
  });

  createEffect(() => {
    if (state().kind !== "invalidRoute" && state().kind !== "unavailable") return;
    window.requestAnimationFrame(() => failureHeading?.focus({ preventScroll: true }));
  });

  const returnHref = (): string => {
    const current = state();
    return current.kind === "ready"
      ? inspectedStudentWorkReturnUrl(current.detail.returnContext)
      : `/instructor/courses/${props.courseReference}/gradebook`;
  };
  const returnLabel = createMemo(() => {
    const current = state();
    return current.kind === "ready" && current.detail.returnContext.kind === "gradingOperation"
      ? "Back to grading operation"
      : "Back to Gradebook";
  });

  return (
    <section class="page student-work-page" data-route-surface="studentWorkInspection">
      <A class="back-link" href={returnHref()}>
        {returnLabel()}
      </A>
      <Show when={state().kind === "loading"}>
        <p class="loading-state" role="status">
          Loading audited Student work...
        </p>
      </Show>
      <Show when={state().kind === "invalidRoute"}>
        <section class="route-error" role="alert">
          <p class="eyebrow">Student work unavailable</p>
          <h1 ref={(element) => (failureHeading = element)} tabindex="-1">
            This inspection link is invalid
          </h1>
          <p>Return to the current Gradebook and choose submitted work from a visible cell.</p>
        </section>
      </Show>
      <Show when={state().kind === "unavailable"}>
        <section class="route-error" role="alert">
          <p class="eyebrow">Student work unavailable</p>
          <h1 ref={(element) => (failureHeading = element)} tabindex="-1">
            This submitted Assignment Attempt could not be inspected
          </h1>
          <p>
            Return to the current Gradebook. The Assignment Attempt may have changed, or this
            account may not have direct Instructor access to the course.
          </p>
          <button class="primary-action" type="button" onClick={() => void load(route())}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={state().kind === "ready" ? state() : undefined}>
        {(readyState) => {
          const ready = readyState() as Extract<InspectionState, { readonly kind: "ready" }>;
          return (
            <>
              <p class="eyebrow">Audited Student work</p>
              <h1>{ready.detail.assignmentTitle}</h1>
              <p class="page-lede">
                {ready.detail.studentDisplayLabel} · submitted run {ready.detail.run}
              </p>
              <section class="student-work-boundary" aria-label="Inspection privacy boundary">
                This view contains the Student's submitted responses, visible question content, and
                score-only feedback. Answer keys and grader material remain server-owned.
              </section>
              <Show
                when={ready.detail.submissions.length > 0}
                fallback={
                  <section class="gradebook-empty">
                    <h2>No submitted responses</h2>
                    <p>The selected run has no immutable completed responses to inspect.</p>
                  </section>
                }
              >
                <div class="student-work-submissions">
                  <For each={ready.detail.submissions}>
                    {(submission, index) => (
                      <SubmissionCard submission={submission} position={index() + 1} />
                    )}
                  </For>
                </div>
              </Show>
              <A
                class="primary-link"
                href={inspectedStudentWorkReturnUrl(ready.detail.returnContext)}
              >
                {ready.detail.returnContext.kind === "gradebook"
                  ? "Return to this Student in the Gradebook"
                  : "Return to this grading operation"}
              </A>
            </>
          );
        }}
      </Show>
    </section>
  );
}

/** Resolves the route's public course reference through the existing course theme scope. */
export function StudentWorkInspectionPage(): JSX.Element {
  const scopedRoute = useCourseThemeRouteData();
  const course = scopedRoute?.kind === "course" ? courseRouteData(scopedRoute).summary : undefined;
  return (
    <Show
      when={course}
      keyed
      fallback={
        <section class="page route-error" role="alert">
          <p class="eyebrow">Student work unavailable</p>
          <h1>Course route is missing</h1>
          <p>Return to the course Gradebook and choose the submitted work again.</p>
        </section>
      }
    >
      {(loadedCourse) => (
        <StudentWorkCoursePage
          courseId={loadedCourse.id}
          courseReference={loadedCourse.reference}
        />
      )}
    </Show>
  );
}
