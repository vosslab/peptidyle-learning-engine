// Small, browser-owned exchange controls over the canonical Blueprint JSON contract.

import { useNavigate } from "@solidjs/router";
import { Show, createSignal, onCleanup, type JSX } from "solid-js";

import type { BlueprintCourseId } from "../../../generated/api/BlueprintCourseId";
import { decodeCanonicalBlueprintCourse } from "../../api/decoders/blueprint_comparison";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { ApiRequestError } from "../../api/http_client";

const EXPORT_FILENAME = "ple-blueprint-course.json";

function detailPath(blueprintCourseId: string): string {
  return `/blueprint-courses/${encodeURIComponent(blueprintCourseId)}`;
}

/** Downloads only the server's current canonical reusable-structure projection. */
export function BlueprintCourseExport(props: {
  readonly client: BlueprintCourseClient;
  readonly blueprintCourseId: BlueprintCourseId;
}): JSX.Element {
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [failed, setFailed] = createSignal(false);

  async function exportCurrent(): Promise<void> {
    if (busy()) return;
    setBusy(true);
    setMessage("");
    setFailed(false);
    try {
      const exchange = await props.client.exportBlueprintCourse(props.blueprintCourseId);
      const url = URL.createObjectURL(
        new Blob([`${JSON.stringify(exchange, undefined, 2)}\n`], { type: "application/json" }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = EXPORT_FILENAME;
      document.body.append(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(url), 0);
      setMessage("Current canonical Blueprint JSON downloaded.");
    } catch {
      setFailed(true);
      setMessage("The canonical Blueprint JSON could not be created. Retry when ready.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="blueprint-course-exchange" aria-labelledby="blueprint-export-heading">
      <div>
        <h2 id="blueprint-export-heading">Export reusable structure</h2>
        <p>
          Download the current canonical Blueprint JSON. It excludes ownership and delivery data.
        </p>
      </div>
      <button type="button" disabled={busy()} onClick={() => void exportCurrent()}>
        {busy() ? "Exporting Blueprint JSON..." : "Export Blueprint JSON"}
      </button>
      <Show when={message()}>
        <p role={failed() ? "alert" : "status"}>{message()}</p>
      </Show>
    </section>
  );
}

/** Imports pasted canonical JSON, validates it locally, then opens the new Private course. */
export function BlueprintCourseImport(props: {
  readonly client: BlueprintCourseClient;
}): JSX.Element {
  const navigate = useNavigate();
  const [open, setOpen] = createSignal(false);
  const [json, setJson] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [retry, setRetry] = createSignal(false);
  let dialog: HTMLDialogElement | undefined;
  let input: HTMLTextAreaElement | undefined;
  let opener: HTMLButtonElement | undefined;
  let importAction: { readonly payload: string; readonly key: string } | undefined;

  function close(): void {
    if (dialog?.open) dialog.close();
    setOpen(false);
    queueMicrotask(() => opener?.focus());
  }

  async function importCurrent(): Promise<void> {
    if (busy()) return;
    let exchange;
    try {
      exchange = decodeCanonicalBlueprintCourse(JSON.parse(json()), "import");
    } catch {
      setRetry(false);
      setMessage("Paste complete canonical Blueprint JSON before importing.");
      return;
    }
    const payload = JSON.stringify(exchange);
    setBusy(true);
    setMessage("");
    try {
      // An uncertain retry keeps the operation key for the same normalized command, even after
      // closing or editing the textarea back to its prior value.
      if (importAction?.payload !== payload) {
        importAction = { payload, key: crypto.randomUUID() };
      }
      const result = await props.client.importBlueprintCourse(exchange, importAction.key);
      importAction = undefined;
      close();
      navigate(detailPath(result.blueprintCourse.id));
    } catch (error: unknown) {
      if (error instanceof ApiRequestError && error.status === 401) {
        setRetry(false);
        setMessage("Your session ended. Sign in again, then return to Blueprint Courses.");
      } else if (error instanceof ApiRequestError && [403, 404].includes(error.status)) {
        setRetry(false);
        setMessage("This Blueprint JSON is unavailable for your Account.");
      } else {
        setRetry(true);
        setMessage(
          "The Blueprint import could not be confirmed. Retry to confirm the same import request.",
        );
      }
    } finally {
      setBusy(false);
    }
  }

  onCleanup(() => {
    if (dialog?.open) dialog.close();
  });

  return (
    <>
      <button
        type="button"
        ref={(element) => {
          opener = element;
        }}
        onClick={() => {
          setMessage("");
          setOpen(true);
          queueMicrotask(() => {
            dialog?.showModal();
            input?.focus();
          });
        }}
      >
        Import Blueprint JSON
      </button>
      <Show when={open()}>
        <dialog
          class="blueprint-course-create-dialog blueprint-course-import-dialog"
          aria-labelledby="blueprint-import-heading"
          ref={(element) => {
            dialog = element;
          }}
          onCancel={(event) => {
            event.preventDefault();
            if (!busy()) close();
          }}
        >
          <div class="blueprint-course-section-heading">
            <div>
              <h2 id="blueprint-import-heading">Import canonical Blueprint JSON</h2>
              <p>Import creates a new Private Blueprint Course from reusable structure.</p>
            </div>
            <button type="button" class="quiet-action" disabled={busy()} onClick={close}>
              Close import
            </button>
          </div>
          <label class="blueprint-course-exchange__json">
            Canonical Blueprint JSON
            <textarea
              ref={(element) => {
                input = element;
              }}
              value={json()}
              onInput={(event) => {
                if (event.currentTarget.value !== json()) setRetry(false);
                setJson(event.currentTarget.value);
              }}
              spellcheck={false}
              aria-describedby="blueprint-import-help"
            />
          </label>
          <p id="blueprint-import-help" class="blueprint-course-field-help">
            Validate the complete JSON in this browser before sending it. Imported ownership and
            delivery data are never accepted.
          </p>
          <Show when={message()}>
            <p role="alert">{message()}</p>
          </Show>
          <div class="blueprint-course-inline-actions">
            <button type="button" disabled={busy()} onClick={() => void importCurrent()}>
              {busy()
                ? "Importing Blueprint JSON..."
                : retry()
                  ? "Retry import"
                  : "Create Private Blueprint Course"}
            </button>
          </div>
        </dialog>
      </Show>
    </>
  );
}
