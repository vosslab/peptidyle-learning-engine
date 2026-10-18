import { Show, type Accessor, type JSX } from "solid-js";

import { useRecordInteractions } from "./data_sheet_interactions";

import type { RecordAction } from "./record_content";

/** One semantic action renderer for record rows and collection actions. */
export function RecordActionControl(props: {
  readonly action: Accessor<RecordAction>;
  readonly toolbar?: boolean;
  readonly disabled?: Accessor<boolean>;
}): JSX.Element {
  const interactions = useRecordInteractions();
  const command = (): Extract<RecordAction, { readonly kind: "command" }> =>
    props.action() as Extract<RecordAction, { readonly kind: "command" }>;
  const link = (): Extract<RecordAction, { readonly kind: "link" }> =>
    props.action() as Extract<RecordAction, { readonly kind: "link" }>;
  const className = (): string =>
    props.toolbar
      ? props.action().primary
        ? "primary-action"
        : "quiet-action"
      : "record-list__action";
  const disabled = (): boolean => props.disabled?.() === true;

  return (
    <Show
      when={props.action().kind === "link"}
      fallback={
        <button
          class={className()}
          classList={{
            "record-list__action--primary": !props.toolbar && props.action().primary === true,
          }}
          type="button"
          disabled={disabled() || command().disabled}
          title={command().title ?? command().label}
          aria-pressed={command().pressed}
          aria-expanded={command().expanded}
          aria-controls={command().controls}
          ref={(element) => command().ref?.(element)}
          onClick={(event) => {
            interactions?.clear();
            command().onClick(event);
          }}
        >
          <span class="record-list__action-label">{props.action().label}</span>
        </button>
      }
    >
      <a
        class={className()}
        classList={{
          "record-list__action--primary": !props.toolbar && link().primary === true,
          "primary-link": link().primary === true,
        }}
        href={link().href}
        target="_blank"
        rel="noopener"
        title={link().title ?? link().label}
        aria-disabled={disabled()}
        ref={(element) => link().ref?.(element)}
        onClick={(event) => {
          if (disabled()) event.preventDefault();
          else link().onFollow?.(event);
        }}
      >
        <span class="record-list__action-label">{props.action().label}</span>
      </a>
    </Show>
  );
}
