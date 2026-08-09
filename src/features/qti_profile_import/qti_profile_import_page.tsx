// qti_profile_import_page.tsx - private, answer-free QTI profile review and conversion.

import { For, Show, createEffect, createMemo, createSignal, type JSX } from "solid-js";

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { WorkspaceImportId } from "../../../generated/api/WorkspaceImportId";
import type {
  QtiProfileDiagnostic,
  QtiProfileImportReadyReport,
  QtiProfileImportResponse,
  QtiProfileItemReport,
} from "./qti_profile_import_contract";
import {
  QtiProfileImportConflictError,
  type QtiProfileImportClient,
} from "./qti_profile_import_client";
import {
  EMPTY_QTI_PROFILE_REVIEW,
  acknowledgeQtiProfileReport,
  qtiConversionBlockReason,
  receiveQtiProfileReport,
  selectQtiProfileItem,
  shouldKeepQtiReplacementLocked,
  shouldRetrySameQtiImport,
  type QtiConversionDraftState,
  type QtiProfileReviewState,
} from "./qti_profile_import_model";
import { QTI_PROFILE_IMPORT_STYLES } from "./qti_profile_import_styles";

export interface QtiProfileImportPageProps {
  readonly workspace: WorkspaceId;
  readonly client: QtiProfileImportClient;
  /** The exact revision and local-change state represented by the visible editor. */
  readonly draftState: () => QtiConversionDraftState | null;
  /** Lets the route reload and reveal the converted flat draft without navigation. */
  readonly onConverted: () => Promise<void>;
  /** Makes the displayed editor inert across the POST-to-refetch replacement window. */
  readonly onConversionHandoffChange: (active: boolean) => void;
}

interface NoticeListProps {
  readonly heading: string;
  readonly notices: ReadonlyArray<QtiProfileDiagnostic>;
}

function NoticeList(props: NoticeListProps): JSX.Element {
  return (
    <Show when={props.notices.length > 0}>
      <section>
        <h5>{props.heading}</h5>
        <ul class="qti-profile-import__item-notices">
          <For each={props.notices}>
            {(notice) => (
              <li>
                {notice.detail} <span class="qti-profile-import__item-identity">{notice.code}</span>
                <Show when={notice.location.length > 0}>
                  <span> Location: {notice.location}</span>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </section>
    </Show>
  );
}

function isReady(
  response: QtiProfileImportResponse | null,
): response is QtiProfileImportReadyReport {
  return response?.state === "ready";
}

function itemTitle(item: QtiProfileItemReport): string {
  return item.title ?? "Untitled source item";
}

function boundedVisibleText(
  value: string,
  maximum: number,
  fallback = "Details unavailable",
): string {
  const safe = Array.from(value, (character) => {
    const point = character.codePointAt(0) ?? 0;
    const isControl = point <= 0x1f || point === 0x7f;
    const isBidirectionalControl =
      (point >= 0x202a && point <= 0x202e) || (point >= 0x2066 && point <= 0x2069);
    return isControl || isBidirectionalControl ? " " : character;
  })
    .join("")
    .trim();
  const characters = Array.from(safe.length === 0 ? fallback : safe);
  return characters.length <= maximum
    ? characters.join("")
    : `${characters.slice(0, maximum).join("")}...`;
}

/**
 * Keeps the archive in component memory and renders only the server's answer-free report. ZIP and
 * XML parsing, conversion decisions, and draft replacement remain server-owned.
 */
export function QtiProfileImportPage(props: QtiProfileImportPageProps): JSX.Element {
  const [archive, setArchive] = createSignal<File | null>(null);
  const [activeImport, setActiveImport] = createSignal<WorkspaceImportId | null>(null);
  const [response, setResponse] = createSignal<QtiProfileImportResponse | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [status, setStatus] = createSignal<string | null>(null);
  const [alert, setAlert] = createSignal<string | null>(null);
  const [uploadConflict, setUploadConflict] = createSignal(false);
  const [replacementRecoveryRequired, setReplacementRecoveryRequired] = createSignal(false);
  const [review, setReview] = createSignal<QtiProfileReviewState>(EMPTY_QTI_PROFILE_REVIEW);
  let archiveInput: HTMLInputElement | null = null;
  let reportHeading: HTMLHeadingElement | null = null;
  let alertRegion: HTMLParagraphElement | null = null;

  createEffect(() => {
    if (alert() !== null) queueMicrotask(() => alertRegion?.focus());
  });

  const readyReport = createMemo(() => {
    return review().report;
  });
  const acceptedCount = createMemo(
    () => readyReport()?.items.filter((item) => item.status === "accepted").length ?? 0,
  );
  const rejectedCount = createMemo(
    () => readyReport()?.items.filter((item) => item.status === "rejected").length ?? 0,
  );
  const conversionBlock = createMemo(() => qtiConversionBlockReason(review(), props.draftState()));

  function clearReview(): void {
    setReview((current) => ({ ...current, acknowledged: false, selectedItem: null }));
  }

  function applyResponse(next: QtiProfileImportResponse): void {
    const previous = response();
    setReview((current) =>
      next.state === "ready" ? receiveQtiProfileReport(current, next) : EMPTY_QTI_PROFILE_REVIEW,
    );
    setResponse(next);
    setAlert(null);
    setUploadConflict(false);
    if (next.state === "queued") {
      setStatus("The archive is queued for server-side review. Refresh status when ready.");
      return;
    }
    if (next.state === "processing") {
      setStatus("The server is reviewing the archive. Refresh status in a moment.");
      return;
    }
    if (next.state === "failed") {
      setStatus(null);
      setAlert(
        `The archive could not be processed. ${boundedVisibleText(next.error, 240)} Retry this archive as a new import or choose a different archive.`,
      );
      return;
    }
    if (next.state === "unsupportedProfile") {
      setStatus(null);
      setAlert(
        `This package does not match a supported Canvas or Blackboard conversion profile. ${boundedVisibleText(next.error, 240)}`,
      );
      return;
    }
    setStatus("The answer-free import report is ready for review.");
    if (!isReady(previous)) queueMicrotask(() => reportHeading?.focus());
  }

  function chooseArchive(event: InputEvent & { readonly currentTarget: HTMLInputElement }): void {
    if (replacementRecoveryRequired()) return;
    const selected = event.currentTarget.files?.item(0) ?? null;
    const previous = archive();
    if (
      selected !== null &&
      previous !== null &&
      (selected.name !== previous.name ||
        selected.size !== previous.size ||
        selected.lastModified !== previous.lastModified)
    ) {
      setActiveImport(null);
      setResponse(null);
      setUploadConflict(false);
      setReview(EMPTY_QTI_PROFILE_REVIEW);
    }
    setArchive(selected);
    setAlert(null);
    if (selected !== null) {
      setStatus(
        `Selected ${boundedVisibleText(selected.name, 160, "unnamed archive")}. It has not been uploaded.`,
      );
    }
  }

  async function uploadArchive(): Promise<void> {
    if (busy() || replacementRecoveryRequired()) return;
    const selected = archive();
    if (selected === null) {
      setAlert("Choose a QTI .zip archive before starting the import.");
      return;
    }
    if (!selected.name.toLowerCase().endsWith(".zip")) {
      setAlert("Choose a file with the .zip extension.");
      return;
    }
    setBusy(true);
    setAlert(null);
    setUploadConflict(false);
    setStatus("Uploading the QTI archive...");
    clearReview();
    const previousImport = activeImport();
    const exactRetry = shouldRetrySameQtiImport(previousImport, response());
    const importId: WorkspaceImportId =
      exactRetry && previousImport !== null ? previousImport : globalThis.crypto.randomUUID();
    setActiveImport(importId);
    if (!exactRetry) setResponse(null);
    try {
      applyResponse(await props.client.upload(props.workspace, importId, selected));
    } catch (error: unknown) {
      if (error instanceof QtiProfileImportConflictError) {
        setUploadConflict(true);
        setAlert(
          "This import already belongs to a different archive. Start a new import to upload this file with a fresh identity.",
        );
      } else {
        setAlert("The archive could not be uploaded. It remains selected so you can retry.");
      }
      setStatus(null);
    } finally {
      setBusy(false);
    }
  }

  async function refreshStatus(): Promise<void> {
    if (busy() || replacementRecoveryRequired()) return;
    const importId = activeImport();
    if (importId === null) return;
    setBusy(true);
    setAlert(null);
    setUploadConflict(false);
    setStatus("Refreshing the server report...");
    try {
      applyResponse(await props.client.report(props.workspace, importId));
    } catch (_error: unknown) {
      setAlert("The import status could not be refreshed. The current report remains visible.");
      setStatus(null);
    } finally {
      setBusy(false);
    }
  }

  async function convertSelectedItem(): Promise<void> {
    if (busy()) return;
    const report = readyReport();
    const item = review().selectedItem;
    const displayedDraft = props.draftState();
    const blocked = qtiConversionBlockReason(review(), displayedDraft);
    if (blocked === "draftUnavailable") {
      setAlert("Wait for the current workspace draft to finish loading before conversion.");
      return;
    }
    if (blocked === "draftDirty") {
      setAlert("Save or reload the current editor changes before replacing this private draft.");
      return;
    }
    if (blocked !== null || report === null || item === null || displayedDraft === null) return;
    if (
      !report.items.some(
        (candidate) => candidate.sourceIdentifier === item && candidate.status === "accepted",
      )
    ) {
      setAlert("Choose an accepted item from the current report.");
      return;
    }
    setBusy(true);
    setAlert(null);
    setStatus("Replacing the current draft with the selected converted item...");
    let handoffStarted = false;
    let conversionCommitted = false;
    let replacementLoaded = false;
    try {
      handoffStarted = true;
      props.onConversionHandoffChange(true);
      await props.client.convert(
        props.workspace,
        report.importId,
        item,
        { reportRevision: report.reportRevision, reviewToken: report.reviewToken },
        displayedDraft.revision,
      );
      conversionCommitted = true;
      setArchive(null);
      setActiveImport(null);
      setResponse(null);
      setReview(EMPTY_QTI_PROFILE_REVIEW);
      if (archiveInput !== null) archiveInput.value = "";
      setStatus("The selected item replaced the current private draft.");
      const editorAfterConversion = props.draftState();
      if (
        editorAfterConversion === null ||
        editorAfterConversion.dirty ||
        editorAfterConversion.revision !== displayedDraft.revision
      ) {
        setReplacementRecoveryRequired(true);
        setAlert(
          "The item was converted, but the converted draft has not loaded. The previous editor remains locked; use Reload converted draft to finish recovery.",
        );
        return;
      }
      await props.onConverted();
      replacementLoaded = true;
    } catch (error: unknown) {
      if (conversionCommitted) {
        setReplacementRecoveryRequired(true);
        setAlert(
          "The item was converted, but the converted draft could not load. The previous editor remains locked; use Reload converted draft to try again.",
        );
      } else if (error instanceof QtiProfileImportConflictError) {
        clearReview();
        setAlert(
          "The draft or import report changed. Refresh status, review the report again, and then retry conversion.",
        );
      } else {
        setAlert("The item could not be converted. The reviewed report remains visible.");
      }
      setStatus(null);
    } finally {
      if (
        handoffStarted &&
        !shouldKeepQtiReplacementLocked(conversionCommitted, replacementLoaded)
      ) {
        props.onConversionHandoffChange(false);
      }
      setBusy(false);
    }
  }

  async function reloadConvertedDraft(): Promise<void> {
    if (busy() || !replacementRecoveryRequired()) return;
    setBusy(true);
    setAlert(null);
    setStatus("Reloading the converted private draft...");
    try {
      await props.onConverted();
      props.onConversionHandoffChange(false);
      setReplacementRecoveryRequired(false);
      setStatus("The converted private draft is ready for review.");
    } catch (_error: unknown) {
      setStatus(null);
      setAlert(
        "The converted draft still could not load. The previous editor remains locked; use Reload converted draft to try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  function startNewImport(): void {
    if (busy() || replacementRecoveryRequired()) return;
    setArchive(null);
    setActiveImport(null);
    setResponse(null);
    setReview(EMPTY_QTI_PROFILE_REVIEW);
    setAlert(null);
    setUploadConflict(false);
    setStatus("Choose a new QTI .zip archive.");
    if (archiveInput !== null) archiveInput.value = "";
    queueMicrotask(() => archiveInput?.focus());
  }

  function retrySelectedArchiveWithNewIdentity(): void {
    if (busy() || replacementRecoveryRequired()) return;
    setActiveImport(null);
    setResponse(null);
    setReview(EMPTY_QTI_PROFILE_REVIEW);
    setAlert(null);
    setUploadConflict(false);
    setStatus("The selected archive is ready to upload as a new import.");
  }

  return (
    <section class="page" data-route-surface="qtiProfileImport">
      <style>{QTI_PROFILE_IMPORT_STYLES}</style>
      <details class="qti-profile-import" open aria-busy={busy()}>
        <summary>
          <span>Import a QTI package</span>
          <span class="qti-profile-import__summary-help">
            Review recognized items before replacing this private draft.
          </span>
        </summary>
        <div class="qti-profile-import__body">
          <p class="qti-profile-import__intro">
            Upload one QTI ZIP archive. PLE reviews it on the server and shows accepted items,
            rejected items, defaults, and warnings before conversion.
          </p>

          <Show when={archive()}>
            {(selected) => (
              <p class="qti-profile-import__archive-context">
                Selected archive:{" "}
                <strong>{boundedVisibleText(selected().name, 160, "unnamed archive")}</strong> (
                {selected().size} bytes). It remains available in this page for the retry actions
                described below.
              </p>
            )}
          </Show>

          <Show
            when={
              !replacementRecoveryRequired() &&
              (response() === null || response()?.state === "failed")
            }
          >
            <label class="qti-profile-import__field">
              <span>QTI ZIP archive</span>
              <input
                ref={(node) => (archiveInput = node)}
                type="file"
                accept=".zip,application/zip"
                disabled={busy()}
                onInput={chooseArchive}
              />
              <span class="qti-profile-import__field-help">
                The selected file stays only in this page until upload and is cleared after a
                successful conversion.
              </span>
            </label>
            <div class="qti-profile-import__actions">
              <button
                type="button"
                class="primary-action"
                disabled={busy() || archive() === null}
                onClick={() => void uploadArchive()}
              >
                {response()?.state === "failed"
                  ? "Retry archive as a new import"
                  : activeImport() === null
                    ? "Start import"
                    : "Retry the same import"}
              </button>
              <Show when={uploadConflict()}>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={busy()}
                  onClick={retrySelectedArchiveWithNewIdentity}
                >
                  Use selected archive in a new import
                </button>
              </Show>
            </div>
          </Show>

          <Show when={status()}>
            {(message) => (
              <p class="qti-profile-import__status" role="status">
                {message()}
              </p>
            )}
          </Show>
          <Show when={alert()}>
            {(message) => (
              <p
                ref={(node) => (alertRegion = node)}
                class="qti-profile-import__alert"
                role="alert"
                tabindex="-1"
              >
                {message()}
              </p>
            )}
          </Show>

          <Show when={replacementRecoveryRequired()}>
            <section
              class="qti-profile-import__review"
              aria-labelledby="qti-replacement-recovery-heading"
            >
              <h3 id="qti-replacement-recovery-heading">Converted draft reload required</h3>
              <p>
                Conversion is complete, but the converted draft has not loaded. The previous editor
                stays locked so it cannot overwrite the converted draft.
              </p>
              <div class="qti-profile-import__actions">
                <button
                  type="button"
                  class="primary-action"
                  disabled={busy()}
                  onClick={() => void reloadConvertedDraft()}
                >
                  Reload converted draft
                </button>
              </div>
            </section>
          </Show>

          <Show when={response()?.state === "queued" || response()?.state === "processing"}>
            <div class="qti-profile-import__actions">
              <button
                type="button"
                class="primary-action"
                disabled={busy()}
                onClick={() => void refreshStatus()}
              >
                Refresh status
              </button>
              <button type="button" class="quiet-action" disabled={busy()} onClick={startNewImport}>
                Choose a different archive
              </button>
            </div>
          </Show>

          <Show when={response()?.state === "failed" || response()?.state === "unsupportedProfile"}>
            <div class="qti-profile-import__actions">
              <Show when={response()?.state === "failed"}>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={busy()}
                  onClick={() => void refreshStatus()}
                >
                  Refresh status
                </button>
              </Show>
              <button type="button" class="quiet-action" disabled={busy()} onClick={startNewImport}>
                Choose a different archive
              </button>
            </div>
          </Show>

          <Show when={readyReport()}>
            {(report) => (
              <section
                class="qti-profile-import__report"
                aria-labelledby="qti-import-report-heading"
              >
                <header>
                  <h2
                    id="qti-import-report-heading"
                    ref={(node) => (reportHeading = node)}
                    tabindex="-1"
                  >
                    QTI import report
                  </h2>
                  <p>
                    {acceptedCount()} accepted; {rejectedCount()} rejected. Source order is
                    preserved below.
                  </p>
                </header>
                <dl class="qti-profile-import__profile">
                  <div>
                    <dt>Recognized profile</dt>
                    <dd>{report().profileLabel}</dd>
                  </div>
                  <div>
                    <dt>Profile version</dt>
                    <dd>{report().profileVersion}</dd>
                  </div>
                  <div>
                    <dt>Profile identifier</dt>
                    <dd>{report().profileId}</dd>
                  </div>
                </dl>

                <Show when={report().pleDefaults.length > 0}>
                  <section>
                    <h3>PLE conversion defaults</h3>
                    <p>
                      Review these values in the flat editor after conversion and change them if
                      needed.
                    </p>
                    <ul class="qti-profile-import__notice-list">
                      <For each={report().pleDefaults}>
                        {(notice) => (
                          <li>
                            {notice.detail}{" "}
                            <span class="qti-profile-import__item-identity">{notice.code}</span>
                          </li>
                        )}
                      </For>
                    </ul>
                  </section>
                </Show>

                <section aria-labelledby="qti-import-items-heading">
                  <h3 id="qti-import-items-heading">Package items</h3>
                  <ol class="qti-profile-import__items">
                    <For each={report().items}>
                      {(item, index) => (
                        <li
                          class={`qti-profile-import__item qti-profile-import__item--${item.status}`}
                        >
                          <div class="qti-profile-import__item-heading">
                            <span class="qti-profile-import__item-icon" aria-hidden="true">
                              {item.status === "accepted" ? <>&#10003;</> : <>&#10007;</>}
                            </span>
                            <h4>
                              {item.status === "accepted" ? "Accepted" : "Rejected"}:{" "}
                              {itemTitle(item)}
                            </h4>
                          </div>
                          <p class="qti-profile-import__item-identity">
                            Source identifier: {item.sourceIdentifier}
                          </p>
                          <NoticeList
                            heading="Why this item was rejected"
                            notices={item.diagnostics}
                          />
                          <NoticeList
                            heading="Defaults applied to this item"
                            notices={item.defaults}
                          />
                          <NoticeList heading="Warnings for this item" notices={item.warnings} />
                          <Show when={item.status === "accepted"}>
                            <label class="qti-profile-import__choice">
                              <input
                                type="radio"
                                name="qti-item-to-convert"
                                value={item.sourceIdentifier}
                                checked={review().selectedItem === item.sourceIdentifier}
                                disabled={busy()}
                                aria-describedby={`qti-item-${index()}-replacement-warning`}
                                onChange={() =>
                                  setReview((current) => selectQtiProfileItem(current, item))
                                }
                              />
                              <span>Select this item for conversion</span>
                            </label>
                            <p
                              id={`qti-item-${index()}-replacement-warning`}
                              class="qti-profile-import__field-help"
                            >
                              Conversion replaces the current private draft in this workspace.
                            </p>
                          </Show>
                        </li>
                      )}
                    </For>
                  </ol>
                </section>

                <Show
                  when={acceptedCount() > 0}
                  fallback={
                    <section
                      class="qti-profile-import__review"
                      aria-labelledby="qti-no-convertible-items-heading"
                    >
                      <h3 id="qti-no-convertible-items-heading">No items can be converted</h3>
                      <p>Review the rejection reasons above, then choose another package.</p>
                      <div class="qti-profile-import__actions">
                        <button
                          type="button"
                          class="quiet-action"
                          disabled={busy()}
                          onClick={startNewImport}
                        >
                          Choose a different archive
                        </button>
                      </div>
                    </section>
                  }
                >
                  <section class="qti-profile-import__review" aria-labelledby="qti-review-heading">
                    <h3 id="qti-review-heading">Confirm your review</h3>
                    <label class="qti-profile-import__acknowledgement">
                      <input
                        type="checkbox"
                        checked={review().acknowledged}
                        disabled={busy()}
                        onChange={(event) =>
                          setReview((current) =>
                            acknowledgeQtiProfileReport(current, event.currentTarget.checked),
                          )
                        }
                      />
                      <span>
                        I reviewed the profile, accepted and rejected items, defaults, and warnings
                        shown in this report.
                      </span>
                    </label>
                    <p>
                      Conversion replaces the current private draft. It does not publish the
                      question.
                    </p>
                    <Show when={conversionBlock() === "draftUnavailable"}>
                      <p id="qti-conversion-draft-guidance" class="qti-profile-import__field-help">
                        Wait for the current workspace draft to finish loading before conversion.
                      </p>
                    </Show>
                    <Show when={conversionBlock() === "draftDirty"}>
                      <p id="qti-conversion-draft-guidance" class="qti-profile-import__alert">
                        Save or reload the current editor changes before replacing this private
                        draft.
                      </p>
                    </Show>
                    <div class="qti-profile-import__actions">
                      <button
                        type="button"
                        class="primary-action"
                        disabled={busy() || conversionBlock() !== null}
                        aria-describedby={
                          conversionBlock() === "draftDirty" ||
                          conversionBlock() === "draftUnavailable"
                            ? "qti-conversion-draft-guidance"
                            : undefined
                        }
                        onClick={() => void convertSelectedItem()}
                      >
                        Convert selected item
                      </button>
                      <button
                        type="button"
                        class="quiet-action"
                        disabled={busy()}
                        onClick={() => void refreshStatus()}
                      >
                        Refresh status
                      </button>
                      <button
                        type="button"
                        class="quiet-action"
                        disabled={busy()}
                        onClick={startNewImport}
                      >
                        Choose a different archive
                      </button>
                    </div>
                  </section>
                </Show>
              </section>
            )}
          </Show>
        </div>
      </details>
    </section>
  );
}
