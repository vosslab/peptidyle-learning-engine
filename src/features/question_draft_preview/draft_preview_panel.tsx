import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  ErrorBoundary,
  Show,
  type JSX,
} from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";
import {
  OpaqueWebworkPreviewFrame,
  type OpaqueWebworkResponsePairs,
} from "../../components/opaque_webwork_preview_frame";
import { QuestionResponseControl } from "../../components/question_response_controls/question_response_control";
import { QuestionPromptRenderer } from "../../components/question_renderer";
import type { QuestionResponseControlBaseProps } from "../../components/question_response_controls/common";
import type { WasmFacade } from "../../wasm/index";
import type { DraftQuestionRouteId } from "../../navigation/public_route";
import {
  createDraftQuestionPreviewClient,
  draftPreviewSeed,
  type DraftPreviewClientConfig,
  type DraftQuestionPreviewClient,
  type DraftSourceBinding,
  type DraftTestResult,
} from "./draft_preview_client";

export type DraftPreviewPanelProps = {
  readonly draftQuestion: DraftQuestionRouteId;
  readonly draftQuestionEditNumber: string;
  readonly binding: DraftSourceBinding;
  readonly isSaved: boolean;
  readonly responseValidator: Pick<WasmFacade, "validateResponseFormat">;
  readonly hotspotDraftQuestionImage?: QuestionResponseControlBaseProps["hotspotDraftQuestionImage"];
  readonly client?: DraftQuestionPreviewClient;
  readonly clientConfig?: DraftPreviewClientConfig;
};

function resultMessage(result: DraftTestResult): string {
  if (result.kind === "ungraded") return "The Question Backend did not score this response.";
  if (result.correct) return "Correct. The Question Backend awarded full credit.";
  if (result.normalizedCredit > 0)
    return `Partially correct. The Question Backend awarded ${Math.round(result.normalizedCredit * 100)}% credit.`;
  return "Not correct. The Question Backend awarded no credit.";
}

function safeErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.length > 0 && error.message.length < 240)
    return error.message;
  return fallback;
}

/** Renders and transiently tests the exact saved Draft through its selected backend. */
export function DraftPreviewPanel(props: DraftPreviewPanelProps): JSX.Element {
  const client = props.client ?? createDraftQuestionPreviewClient(props.clientConfig);
  const [nativeResponse, setNativeResponse] = createSignal<StudentResponse>();
  const [testResult, setTestResult] = createSignal<DraftTestResult>();
  const [testError, setTestError] = createSignal("");
  const [testing, setTesting] = createSignal(false);
  const [seed] = createSignal(draftPreviewSeed());
  const nativePreviewKey = createMemo(() =>
    props.binding.backend === "ple" && props.isSaved ? props.draftQuestionEditNumber : undefined,
  );
  createEffect(() => {
    nativePreviewKey();
    setNativeResponse(undefined);
    setTestResult(undefined);
    setTestError("");
  });
  const [nativePreview] = createResource(
    nativePreviewKey,
    async (editNumber) => await client.loadNativePreview(props.draftQuestion, editNumber),
  );
  const nativePreviewError = (): unknown => nativePreview.error as unknown;
  const webworkPreviewSource = createMemo(() => {
    if (props.binding.backend !== "webwork" || !props.isSaved) return undefined;
    return client.webworkPreviewPath(props.draftQuestion, props.draftQuestionEditNumber, seed());
  });

  async function testResponse(response: StudentResponse, testSeed?: number): Promise<void> {
    if (!props.isSaved || testing()) return;
    const testedEditNumber = props.draftQuestionEditNumber;
    setTesting(true);
    setTestError("");
    setTestResult(undefined);
    try {
      const result = await client.test(props.draftQuestion, testedEditNumber, response, testSeed);
      if (props.isSaved && props.draftQuestionEditNumber === testedEditNumber) {
        setTestResult(result);
      }
    } catch (error: unknown) {
      if (props.isSaved && props.draftQuestionEditNumber === testedEditNumber) {
        setTestError(
          safeErrorMessage(
            error,
            "Draft testing failed. Your saved source and local edits remain available.",
          ),
        );
      }
    } finally {
      setTesting(false);
    }
  }

  function testWebworkResponse(pairs: OpaqueWebworkResponsePairs): void {
    let response: StudentResponse;
    try {
      response = client.webworkResponse(pairs);
    } catch (error: unknown) {
      setTestError(safeErrorMessage(error, "The Draft response could not be read."));
      return;
    }
    void testResponse(response, seed());
  }

  return (
    <section class="draft-preview-panel" aria-labelledby="draft-preview-heading">
      <h2 id="draft-preview-heading">Saved Draft preview and test</h2>
      <Show
        when={props.isSaved}
        fallback={
          <p role="status">Save the latest Draft edits before previewing or testing them.</p>
        }
      >
        <Show when={props.binding.backend === "ple"}>
          <Show
            when={nativePreview()}
            keyed
            fallback={
              <Show
                when={nativePreviewError()}
                fallback={<p role="status">Loading the saved Native Draft preview...</p>}
              >
                <p role="alert">
                  {safeErrorMessage(
                    nativePreviewError(),
                    "Native Draft preview failed. The saved source remains available for editing.",
                  )}
                </p>
              </Show>
            }
          >
            {(preview) => (
              <div class="draft-preview-native">
                <article aria-label="Native Draft question prompt">
                  <h3>{preview.questionTitle}</h3>
                  <ErrorBoundary
                    fallback={(_error, reset) => (
                      <section role="alert">
                        <p>
                          The saved Native Draft prompt could not be rendered. Your source remains
                          available for editing.
                        </p>
                        <button type="button" onClick={reset}>
                          Retry prompt rendering
                        </button>
                      </section>
                    )}
                  >
                    <QuestionPromptRenderer
                      blocks={preview.prompt}
                      draftQuestion={props.draftQuestion}
                      questionImageUrl={(asset) => {
                        const resolve = props.hotspotDraftQuestionImage?.questionImageUrl;
                        if (resolve === undefined)
                          throw new Error("Draft image preview is unavailable.");
                        return resolve(asset);
                      }}
                    />
                  </ErrorBoundary>
                </article>
                <QuestionResponseControl
                  attemptId="draft-preview-response"
                  mode="formatOnly"
                  responseFormat={preview.response}
                  validator={props.responseValidator}
                  hotspotDraftQuestionImage={props.hotspotDraftQuestionImage}
                  onEscape={() => undefined}
                  onResponseChange={(response, validation) =>
                    setNativeResponse(validation.issues.length === 0 ? response : undefined)
                  }
                />
                <button
                  type="button"
                  disabled={testing() || nativeResponse() === undefined}
                  onClick={() => {
                    const response = nativeResponse();
                    if (response !== undefined) void testResponse(response);
                  }}
                >
                  {testing() ? "Testing response..." : "Test response"}
                </button>
              </div>
            )}
          </Show>
        </Show>
        <Show when={props.binding.backend === "webwork"}>
          <Show
            when={webworkPreviewSource()}
            fallback={<p role="status">Loading the saved WebWork Draft preview...</p>}
          >
            {(src) => (
              <OpaqueWebworkPreviewFrame
                class="draft-preview-webwork-frame"
                src={src()}
                title={`${props.binding.format === "webworkPgml" ? "PGML" : "PG"} Draft preview`}
                onDraftTestResponse={testWebworkResponse}
              />
            )}
          </Show>
        </Show>
      </Show>
      <Show when={testing() || testResult() !== undefined || testError()}>
        <p role={testError() ? "alert" : "status"}>
          {testError() ||
            (testResult() === undefined ? "Testing response..." : resultMessage(testResult()!))}
        </p>
      </Show>
      <p class="draft-preview-panel__privacy">
        Testing is private and temporary. It creates no Assessment Attempt or Student Work.
      </p>
    </section>
  );
}
