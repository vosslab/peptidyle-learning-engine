// Browser-local QPM conversion followed by ordinary, sequential Draft saves.

import { For, Show, createResource, createSignal, type JSX } from "solid-js";

import type { FormatInfo, Warning } from "../../assets/qpm_wasm/src/index.js";
import { createPleQuestionJsonClient } from "../features/ple_question_json_authoring/question_json_client";
import { saveQuestionImportItem, type QuestionImportResult } from "./question_import_items";
import { createQuestionDraftCreationClient } from "../api/question_draft_creation";

declare const __QPM_WASM_ASSET_VERSION__: string;
type Qpm = typeof import("../../assets/qpm_wasm/src/index.js");
/** Loads the vendored wrapper and WASM only while the import screen is open. */
async function loadQpm(): Promise<Qpm> {
  const base = `/wasm/qpm-${__QPM_WASM_ASSET_VERSION__}`;
  const wrapper = `${base}/src/index.js`;
  const qpm = (await import(wrapper)) as Qpm;
  const response = await fetch(`${base}/generated/qpm_wasm_bg.wasm`);
  if (!response.ok) throw new Error("The question importer could not be loaded.");
  await qpm.initialize(new Uint8Array(await response.arrayBuffer()));
  return qpm;
}

/** Imports supported banks into saved Drafts; existing preview/publication validate Native JSON. */
export function QuestionImportScreen(props: { readonly onClose: () => void }): JSX.Element {
  const [qpm] = createResource(loadQpm);
  const [file, setFile] = createSignal<File>();
  const [format, setFormat] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [warnings, setWarnings] = createSignal<ReadonlyArray<Warning>>([]);
  const [results, setResults] = createSignal<ReadonlyArray<QuestionImportResult>>([]);
  const client = createQuestionDraftCreationClient();
  const imageClient = createPleQuestionJsonClient();
  const readableFormats = (): ReadonlyArray<FormatInfo> =>
    qpm.state === "ready"
      ? qpm()
          .formats()
          .formats.filter((item) => item.canRead)
      : [];

  async function importQuestions(): Promise<void> {
    const selected = file();
    if (busy() || selected === undefined || qpm.state !== "ready" || format() === "") return;
    const converter = qpm();
    setBusy(true);
    setResults([]);
    setWarnings([]);
    setMessage("Converting question bank...");
    try {
      const converted = converter.convert({
        inputFormat: format(),
        outputFormat: "ple_native_json",
        allowMixed: true,
        input: {
          kind: "file",
          name: selected.name,
          bytes: new Uint8Array(await selected.arrayBuffer()),
        },
      });
      setWarnings(converted.warnings);
      if (converted.status === "error") {
        setMessage(converted.error.message);
        return;
      }
      if (converted.artifact?.kind !== "directory") {
        throw new Error("The converter did not return Native JSON question files.");
      }
      const items = converted.artifact.entries
        .filter((item) => /^item_[0-9]+\.json$/u.test(item.name))
        .sort((left, right) => left.name.localeCompare(right.name));
      for (const [index, item] of items.entries()) {
        setMessage(`Saving question ${index + 1} of ${items.length}...`);
        const result = await saveQuestionImportItem(
          item,
          converted.artifact.entries,
          client,
          imageClient,
        );
        setResults((previous) => [...previous, result]);
      }
      setMessage(
        items.length === 0
          ? "No questions were returned by the converter."
          : `Import complete. ${results().filter((item) => item.href !== undefined).length} of ${items.length} questions saved as Drafts. ${results().filter((item) => item.status === "created").length} ready to preview.`,
      );
    } catch (error: unknown) {
      setMessage(error instanceof Error ? error.message : "The question bank could not be read.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="question-draft-creation" aria-labelledby="question-import-heading">
      <h2
        id="question-import-heading"
        tabIndex={-1}
        ref={(element) => queueMicrotask(() => element.focus())}
      >
        Import questions
      </h2>
      <p>
        Select a question bank to convert on this device. Created questions are saved as private
        Drafts. Open each Draft to preview it, add metadata, and publish.
      </p>
      <p>
        Referenced images are saved with each Draft. Create HOTSPOT questions in the Draft editor.
      </p>
      <Show when={qpm.loading}>
        <p role="status">Loading question importer...</p>
      </Show>
      <Show when={qpm.error !== undefined}>
        <p role="alert">The question importer could not be loaded. Close and open Import again.</p>
      </Show>
      <form
        class="question-draft-creation__form"
        onSubmit={(event) => {
          event.preventDefault();
          void importQuestions();
        }}
      >
        <label class="question-draft-creation__field">
          <span>Question bank file</span>
          <input
            type="file"
            disabled={busy()}
            onChange={(event) => setFile(event.currentTarget.files?.[0])}
          />
        </label>
        <label class="question-draft-creation__field">
          <span>Question bank format</span>
          <select
            value={format()}
            disabled={busy() || qpm.loading}
            onChange={(event) => setFormat(event.currentTarget.value)}
          >
            <option value="">Choose a format</option>
            <For each={readableFormats()}>
              {(item) => <option value={item.name}>{item.name}</option>}
            </For>
          </select>
        </label>
        <div class="action-row">
          <button
            class="primary-action"
            type="submit"
            disabled={
              busy() ||
              file() === undefined ||
              format() === "" ||
              qpm.loading ||
              qpm.error !== undefined
            }
          >
            {busy() ? "Importing..." : "Import into Drafts"}
          </button>
          <button class="quiet-action" type="button" disabled={busy()} onClick={props.onClose}>
            Close import
          </button>
        </div>
      </form>
      <p role="status" aria-live="polite">
        {message()}
      </p>
      <Show when={warnings().length > 0}>
        <section aria-label="Conversion warnings">
          <h3>Conversion warnings</h3>
          <ul>
            <For each={warnings()}>
              {(warning) => (
                <li>
                  {warning.item ? `${warning.item}: ` : ""}
                  {warning.message}
                </li>
              )}
            </For>
          </ul>
        </section>
      </Show>
      <Show when={results().length > 0}>
        <section aria-label="Import results">
          <h3>Import results</h3>
          <ol>
            <For each={results()}>
              {(item) => (
                <li>
                  <strong>{item.name}</strong>: {item.status} - {item.message}
                  <Show when={item.href}>
                    {(href) => (
                      <>
                        {" "}
                        <a href={href()} target="_blank" rel="noopener">
                          Open Draft ({item.name})
                        </a>
                      </>
                    )}
                  </Show>
                </li>
              )}
            </For>
          </ol>
        </section>
      </Show>
    </section>
  );
}
