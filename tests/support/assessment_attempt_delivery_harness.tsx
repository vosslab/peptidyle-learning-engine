// Mounts the shipped Student Assessment Attempt so one Course Assessment delivers a Question.

import { type JSX } from "solid-js";
import { MemoryRouter, Route, createMemoryHistory, useLocation } from "@solidjs/router";
import { render } from "solid-js/web";

import {
  ApplicationApiProvider,
  createApplicationApi,
  type ApplicationApi,
} from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import { AssessmentAttemptPage } from "../../src/pages/assessment_attempt_page";
import { assessmentAttemptRouteState } from "../../src/navigation/assessment_attempt_route";
import { assessmentAttemptRouteId } from "../../src/navigation/public_route";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";
import { WasmRuntimeProvider } from "../../src/wasm/context";

const ATTEMPT_ID = "11111111-1111-4111-8111-666666666666";
const ATTEMPT_PATH = "/courses/CI7K3M2QAZ/attempt";

export interface AssessmentAttemptPresentationCall {
  readonly assessmentAttemptId: string;
  readonly position: number;
}

let disposeMount: (() => void) | undefined;

export interface AssessmentAttemptTiming {
  readonly expiresAt: number;
  readonly timerRemainingMilliseconds: number;
}

function deliveryApplicationApi(
  prompt: string,
  timing: AssessmentAttemptTiming | null,
): ApplicationApi<OrdinaryBrowserApiClient> {
  const client = {
    getStudentAssessmentAttemptContext(assessmentAttemptId: string): Promise<unknown> {
      window.assessmentAttemptContextIds.push(assessmentAttemptId);
      return Promise.resolve({
        assessmentAttemptId,
        attemptNumber: 1,
        displayTimeZone: "America/Chicago",
        expiresAt: timing === null ? null : timing.expiresAt,
        timerRemainingMilliseconds: timing === null ? null : timing.timerRemainingMilliseconds,
        course: {
          id: "CI7K3M2QAZ",
          shortName: "BIO 101",
          longName: "Introductory Biology",
          theme: "grass",
        },
        assessment: {
          id: "A8H4N6PA6",
          assessmentType: "regular_assignment",
          title: "Membrane review",
        },
      });
    },
    getStudentAssessmentAttemptProgress(assessmentAttemptId: string): Promise<unknown> {
      window.assessmentAttemptProgressIds.push(assessmentAttemptId);
      return Promise.resolve({
        assessmentAttemptId,
        questionCount: 1,
        recommendedPosition: 1,
        positions: [{ position: 1, responseState: "unanswered" as const, displayDurationMs: null }],
      });
    },
    getStudentAssessmentAttemptPresentation(
      assessmentAttemptId: string,
      position: number,
    ): Promise<unknown> {
      window.assessmentAttemptPresentationCalls.push({ assessmentAttemptId, position });
      return Promise.resolve({
        position,
        presentation: {
          publishedQuestionRevisionTuple: {
            publishedQuestionId: "7K3M-79QP",
            revisionNumber: 2,
          },
          prompt: [{ kind: "text" as const, markdown: prompt }],
          response: { kind: "fillIn" as const, maxCharacters: 80 },
        },
        savedResponse: null,
      });
    },
    questionImageUrl(): string {
      return "/question-image";
    },
    checkpointStudentQuestionDisplayDuration(): Promise<{ cumulativeDisplayDurationMs: number }> {
      return Promise.resolve({ cumulativeDisplayDurationMs: 0 });
    },
  };
  return createApplicationApi(client as unknown as OrdinaryBrowserApiClient);
}

function DeliveredAttempt(): JSX.Element {
  const location = useLocation();
  return (
    <RouteScopeProvider pathname={ATTEMPT_PATH} historyState={() => location.state}>
      <AssessmentAttemptPage />
    </RouteScopeProvider>
  );
}

/** Mounts one Course Assessment Attempt through the Student Assessment Attempt client. */
export function mountAssessmentAttemptDelivery(
  target: HTMLElement,
  prompt: string,
  timing: AssessmentAttemptTiming | null = null,
): void {
  disposeMount?.();
  window.assessmentAttemptContextIds = [];
  window.assessmentAttemptProgressIds = [];
  window.assessmentAttemptPresentationCalls = [];
  const history = createMemoryHistory();
  history.set({
    value: ATTEMPT_PATH,
    state: assessmentAttemptRouteState(assessmentAttemptRouteId(ATTEMPT_ID)),
    replace: true,
  });
  const applicationApi = deliveryApplicationApi(prompt, timing);
  disposeMount = render(
    () => (
      <WasmRuntimeProvider
        formatFallback={() => Promise.resolve({ issues: [] })}
        timerFallback={() => Promise.resolve("untimed")}
        capabilityFallback={() => Promise.resolve([])}
      >
        <ApplicationApiProvider applicationApi={applicationApi}>
          <MemoryRouter history={history}>
            <Route path="/courses/:courseInstanceId/attempt" component={DeliveredAttempt} />
          </MemoryRouter>
        </ApplicationApiProvider>
      </WasmRuntimeProvider>
    ),
    target,
  );
}

declare global {
  interface Window {
    assessmentAttemptContextIds: string[];
    assessmentAttemptProgressIds: string[];
    assessmentAttemptPresentationCalls: AssessmentAttemptPresentationCall[];
  }
}
