import { Show, type JSX } from "solid-js";

import "./record_family.css";

/** Loading and failure state shared by every record presentation. */
export type RecordCollectionState =
  | { readonly kind: "ready" }
  | { readonly kind: "loading"; readonly label?: string }
  | {
      readonly kind: "error";
      readonly title?: string;
      readonly message: string;
      readonly retry?: () => void;
      readonly retryLabel?: string;
    };

export type RecordCollectionEmptyState = {
  readonly title: string;
  readonly message?: string;
};

export type RecordCollectionStateProps = {
  readonly state: RecordCollectionState;
  readonly isEmpty: boolean;
  readonly emptyState: RecordCollectionEmptyState;
  readonly children: JSX.Element;
};

/** Presents collection loading, empty, and error states with one accessible contract. */
export function RecordCollectionStateView(props: RecordCollectionStateProps): JSX.Element {
  const loadingLabel = (): string =>
    props.state.kind === "loading"
      ? (props.state.label ?? "Loading records...")
      : "Loading records...";
  const errorTitle = (): string =>
    props.state.kind === "error"
      ? (props.state.title ?? "Records unavailable")
      : "Records unavailable";
  const errorMessage = (): string =>
    props.state.kind === "error" ? props.state.message : "Records could not be loaded.";
  const errorRetryLabel = (): string =>
    props.state.kind === "error" ? (props.state.retryLabel ?? "Retry") : "Retry";

  const hasRows = (): boolean => !props.isEmpty;

  function loadingNotice(): JSX.Element {
    return (
      <p class="record-collection__state record-collection__state--loading" role="status">
        {loadingLabel()}
      </p>
    );
  }

  function errorNotice(): JSX.Element {
    return (
      <section class="record-collection__state record-collection__state--error" role="alert">
        <h2>{errorTitle()}</h2>
        <p>{errorMessage()}</p>
        <Show when={props.state.kind === "error" && props.state.retry !== undefined}>
          <button
            class="quiet-action"
            type="button"
            onClick={() => props.state.kind === "error" && props.state.retry?.()}
          >
            {errorRetryLabel()}
          </button>
        </Show>
      </section>
    );
  }

  return (
    <Show
      when={hasRows()}
      fallback={
        <Show
          when={props.state.kind === "ready"}
          fallback={
            <Show when={props.state.kind === "loading"} fallback={errorNotice()}>
              {loadingNotice()}
            </Show>
          }
        >
          <section class="record-collection__state record-collection__state--empty" role="status">
            <h2>{props.emptyState.title}</h2>
            <Show when={props.emptyState.message}>{(message) => <p>{message()}</p>}</Show>
          </section>
        </Show>
      }
    >
      <Show when={props.state.kind === "loading"}>{loadingNotice()}</Show>
      <Show when={props.state.kind === "error"}>{errorNotice()}</Show>
      {props.children}
    </Show>
  );
}
