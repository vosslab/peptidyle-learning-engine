// Subject, Topic, and Subtopic choices sit with the control that creates the next name.

import { createEffect, createSignal, onCleanup, Show, type JSX } from "solid-js";

import type { ContentClassificationItem } from "../api/content_classification";
import { ContentClassificationSelect } from "./content_classification_select";

export interface VocabularyCreation {
  readonly uuid: string;
  readonly name: string;
  readonly needsAcceptance: boolean;
}

export function AuthoringClassificationLevel(props: {
  readonly label: string;
  readonly value: string | null;
  readonly parentUuid: string | null;
  readonly required?: boolean;
  readonly disabled?: boolean;
  readonly load: (parentUuid: string) => Promise<ReadonlyArray<ContentClassificationItem>>;
  readonly onChange: (uuid: string | null) => void;
  readonly createName: (name: string, parentUuid: string) => Promise<VocabularyCreation>;
  readonly acceptExisting?: (
    uuid: string,
    parentUuid: string,
  ) => Promise<ContentClassificationItem>;
}): JSX.Element {
  const [reloadToken, setReloadToken] = createSignal(0);
  const [name, setName] = createSignal("");
  const [pending, setPending] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [offer, setOffer] = createSignal<{
    uuid: string;
    name: string;
    parentUuid: string;
    generation: number;
  } | null>(null);
  const inputId = `create-${props.label.toLowerCase().replace(/ /g, "-")}-name`;
  let generation = 0;
  let currentParentUuid = props.parentUuid;

  createEffect(() => {
    const parentUuid = props.parentUuid;
    if (parentUuid === currentParentUuid) return;
    currentParentUuid = parentUuid;
    ++generation;
    setOffer(null);
    setPending(false);
    setError(null);
  });
  onCleanup(() => {
    ++generation;
  });

  function selectCreated(uuid: string): void {
    setName("");
    setOffer(null);
    setReloadToken((current) => current + 1);
    props.onChange(uuid);
  }

  async function create(): Promise<void> {
    const parentUuid = props.parentUuid;
    if (parentUuid === null || pending() || name().trim() === "") return;
    const requestGeneration = generation;
    setPending(true);
    setError(null);
    setOffer(null);
    try {
      const creation = await props.createName(name(), parentUuid);
      if (requestGeneration !== generation) return;
      if (creation.needsAcceptance) {
        if (props.acceptExisting === undefined) {
          setError(`${creation.name} already exists.`);
          return;
        }
        setOffer({
          uuid: creation.uuid,
          name: creation.name,
          parentUuid,
          generation: requestGeneration,
        });
        return;
      }
      selectCreated(creation.uuid);
    } catch {
      if (requestGeneration === generation) {
        setError(`That ${props.label} could not be created.`);
      }
    } finally {
      if (requestGeneration === generation) setPending(false);
    }
  }

  async function accept(): Promise<void> {
    const parentUuid = props.parentUuid;
    const pendingOffer = offer();
    if (
      pendingOffer === null ||
      pendingOffer.parentUuid !== parentUuid ||
      pendingOffer.generation !== generation ||
      props.acceptExisting === undefined ||
      pending()
    ) {
      return;
    }
    const requestGeneration = generation;
    setPending(true);
    setError(null);
    try {
      const item = await props.acceptExisting(pendingOffer.uuid, parentUuid);
      if (requestGeneration !== generation) return;
      selectCreated(item.uuid);
    } catch {
      if (requestGeneration === generation) {
        setError(`That ${props.label} could not be associated.`);
      }
    } finally {
      if (requestGeneration === generation) setPending(false);
    }
  }

  return (
    <div class="classification-vocabulary-level">
      <ContentClassificationSelect
        label={props.label}
        required={props.required}
        value={props.value}
        parentUuid={props.parentUuid}
        disabled={props.disabled}
        reloadToken={reloadToken()}
        load={props.load}
        onChange={props.onChange}
      />
      <Show when={props.parentUuid !== null}>
        <div class="classification-vocabulary-create">
          <label for={inputId}>
            New {props.label} name
            <input
              id={inputId}
              type="text"
              value={name()}
              disabled={props.disabled || pending()}
              onInput={(event) => {
                setName(event.currentTarget.value);
                setOffer(null);
              }}
              onKeyDown={(event) => {
                if (event.key !== "Enter") return;
                event.preventDefault();
                void create();
              }}
            />
          </label>
          <button
            class="quiet-action"
            type="button"
            disabled={props.disabled || pending() || name().trim() === ""}
            onClick={() => void create()}
          >
            {pending() && offer() === null ? `Creating ${props.label}...` : `Create ${props.label}`}
          </button>
          <Show when={offer()}>
            {(current) => (
              <>
                <p role="status">
                  {current().name} already exists. Associate it with this Discipline?
                </p>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={props.disabled || pending()}
                  onClick={() => void accept()}
                >
                  {pending() ? "Associating Subject..." : "Associate existing Subject"}
                </button>
              </>
            )}
          </Show>
          <Show when={error()}>
            {(message) => (
              <span class="inline-error" role="alert">
                {message()}
              </span>
            )}
          </Show>
        </div>
      </Show>
    </div>
  );
}
