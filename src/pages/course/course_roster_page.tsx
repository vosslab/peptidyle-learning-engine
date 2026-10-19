// course_roster_page.tsx - Course Roster Import and protected roster projection.

import { A, useParams } from "@solidjs/router";
import { Show, createResource, createSignal, type JSX } from "solid-js";

import "./course_roster_page.css";

import { parseRosterImportRows } from "./roster_import_template";
import {
  RosterConfirmationDialog,
  type PendingRosterConfirmation,
} from "./roster_confirmation_dialog";
import { useApplicationApi } from "../../api/application_api";
import { PageFrame } from "../../components/page_frame/page_frame";
import type { RecordContent } from "../../components/data_sheet/record_content";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import type { CourseRosterEntry } from "../../api/course_roster";
import { type CourseInstanceRouteId, parseCourseInstanceId } from "../../navigation/public_route";
import { rosterFields, rosterStateLabel } from "./course_roster_gradebook_fields";

type RosterFeedback = Readonly<{ kind: "success" | "error"; text: string }>;

/** Current Active Instructor surface for one Course Instance's roster. */
function CourseRosterCoursePage(props: {
  readonly courseInstanceId: CourseInstanceRouteId;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [roster, { refetch }] = createResource(
    () => props.courseInstanceId,
    applicationApi.client.getLiveCourseRoster,
  );
  const [importText, setImportText] = createSignal("");
  const [message, setMessage] = createSignal<RosterFeedback>();
  const [toolMessage, setToolMessage] = createSignal<RosterFeedback>();
  const [busy, setBusy] = createSignal(false);
  const [pendingRemoval, setPendingRemoval] = createSignal<PendingRosterConfirmation | null>(null);
  let rosterTools: HTMLDetailsElement | undefined;
  let rosterImport: HTMLTextAreaElement | undefined;

  // A keyed Course child owns the cache, query, dialogs, and FERPA drafts.
  const rosterSearch = createLocalCompleteSearch(
    {
      rows: roster,
      loading: () => roster.loading,
      error: (): unknown => roster.error,
      refetchSource: refetch,
    },
    {
      fields: rosterFields,
      rowId: (entry) => entry.rosterId,
      keyword: (entry) => `${entry.rosterName} ${entry.rosterId} ${rosterStateLabel(entry.state)}`,
      // ASVS 1.2.1: RecordList renders instructor-provided names as escaped text.
      content: (entry): RecordContent => ({
        title: entry.rosterName,
        description: entry.rosterId,
        details: [{ kind: "text", label: "State", value: rosterStateLabel(entry.state) }],
        actions: [
          {
            id: "remove-course-access",
            kind: "command",
            label: "Remove course access",
            disabled: busy(),
            onClick: (event) => askRemoval(entry, event.currentTarget),
          },
        ],
      }),
    },
  );

  async function importRoster(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const course = props.courseInstanceId;
    setBusy(true);
    setMessage(undefined);
    setToolMessage(undefined);
    try {
      const entries = parseRosterImportRows(importText());
      await applicationApi.client.importLiveCourseRoster(course, { entries });
      setImportText("");
      setToolMessage({
        kind: "success",
        text: "Roster import recorded. Course roster names updated; new Students remain pending until they claim their Course Invitation.",
      });
      await refetch();
    } catch (error) {
      setToolMessage({
        kind: "error",
        text:
          error instanceof Error && error.message.startsWith("Use ")
            ? error.message
            : "The roster import could not be recorded. Check each email, stored roster ID, and Course roster name.",
      });
    } finally {
      setBusy(false);
      if (rosterTools !== undefined) rosterTools.open = true;
    }
  }

  async function revoke(rosterId: string): Promise<void> {
    const course = props.courseInstanceId;
    setBusy(true);
    setMessage(undefined);
    setToolMessage(undefined);
    try {
      await applicationApi.client.revokeLiveCourseRosterEntry(course, rosterId);
      setMessage({
        kind: "success",
        text: "Course access was removed. Protected educational records remain under retention.",
      });
      await refetch();
    } catch {
      setMessage({
        kind: "error",
        text: "This roster entry could not be changed. Reload and try again.",
      });
    } finally {
      setBusy(false);
    }
  }

  async function downloadPendingInvitations(): Promise<void> {
    const course = props.courseInstanceId;
    setBusy(true);
    setMessage(undefined);
    setToolMessage(undefined);
    try {
      const exportBlob = await applicationApi.client.downloadLiveInvitationExport(course);
      const downloadUrl = URL.createObjectURL(exportBlob);
      const link = document.createElement("a");
      link.href = downloadUrl;
      link.download = "ple-invitations.json";
      document.body.append(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(downloadUrl), 0);
      setToolMessage({
        kind: "success",
        text: "Pending Course Invitations downloaded for the attended mailer.",
      });
    } catch {
      setToolMessage({
        kind: "error",
        text: "Pending Course Invitations could not be downloaded.",
      });
    } finally {
      setBusy(false);
      if (rosterTools !== undefined) rosterTools.open = true;
    }
  }

  function askRemoval(entry: CourseRosterEntry, trigger: HTMLButtonElement): void {
    if (busy()) return;
    setPendingRemoval({
      rosterId: entry.rosterId,
      displayName: entry.rosterName,
      state: entry.state,
      trigger,
    });
  }

  function startRosterImport(): void {
    if (rosterTools !== undefined) rosterTools.open = true;
    requestAnimationFrame(() => rosterImport?.focus());
  }

  return (
    <PageFrame
      // Roster feedback and the table section reset. PageFrame owns the stack.
      contentClass="roster-page"
      routeSurface="courseRoster"
    >
      <Show when={message()}>
        {(feedback) => (
          <p
            class={feedback().kind === "error" ? "inline-error" : "roster-feedback-success"}
            role={feedback().kind === "error" ? "alert" : "status"}
          >
            {feedback().text}
          </p>
        )}
      </Show>

      <section class="roster-section" aria-labelledby="current-roster-heading">
        <h2 id="current-roster-heading">Current roster</h2>
        <SearchBlock
          layout="above"
          controls={<SearchControls state={rosterSearch.state} textLabel="Search roster" />}
          state={rosterSearch.state}
          collection={{
            ariaLabel: "Current Course roster",
            ...rosterSearch.collection,
            get emptyState() {
              return {
                title:
                  rosterSearch.state.query().text || rosterSearch.state.query().filters.size
                    ? "No roster entries match your search."
                    : "No roster entries yet.",
              };
            },
          }}
        />
        <Show when={roster.state === "ready" && roster()?.length === 0}>
          <div class="roster-empty-state">
            <button
              class="primary-action"
              type="button"
              aria-controls="live-course-roster-import"
              onClick={startRosterImport}
            >
              Import Students
            </button>
          </div>
        </Show>
      </section>
      <details class="roster-tools" ref={(element) => (rosterTools = element)}>
        <summary>Roster tools</summary>
        <p class="field-help roster-tools-help">
          Import reviewed Student Authentication Email, Course roster ID, and Course roster name
          rows. New entries create pending Course Invitations, not Student Work. To correct a name,
          re-import the same email and stored roster ID with the corrected name. Only the Course
          roster name changes; authentication email and roster identity do not change.
        </p>
        <div class="roster-tools-grid">
          <form class="auth-form" onSubmit={(event) => void importRoster(event)}>
            <h2>Import roster</h2>
            <label for="live-course-roster-import">Email, roster ID, Course roster name</label>
            <textarea
              id="live-course-roster-import"
              rows={4}
              required
              placeholder={
                'synthetic-student@example.edu,synthetic-001,"Example, Synthetic Student"'
              }
              value={importText()}
              onInput={(event) => setImportText(event.currentTarget.value)}
              aria-describedby="live-course-roster-import-help"
              ref={(element) => (rosterImport = element)}
            />
            <p id="live-course-roster-import-help" class="field-help">
              One CSV row per line, up to 50 rows; optional header: email,roster_id,roster_name.
              Quote names containing commas and double any quotation marks inside quoted fields.
              Names are Course-local labels, may repeat, and need 1-200 characters without controls.
              Roster IDs are not sign-in credentials. The placeholder is explicitly synthetic.
            </p>
            <button class="primary-action" type="submit" disabled={busy()}>
              Import roster
            </button>
          </form>
          <section aria-labelledby="invitation-export-heading">
            <h2 id="invitation-export-heading">Pending Course Invitations</h2>
            <p class="field-help">
              Download the protected mailer input for pending Course Invitations. This page does not
              send email.
            </p>
            <button
              class="quiet-action"
              type="button"
              disabled={busy()}
              onClick={() => void downloadPendingInvitations()}
            >
              Download pending invitations
            </button>
          </section>
        </div>
        <Show when={toolMessage()}>
          {(feedback) => (
            <p
              class={feedback().kind === "error" ? "inline-error" : "roster-feedback-success"}
              role={feedback().kind === "error" ? "alert" : "status"}
            >
              {feedback().text}
            </p>
          )}
        </Show>
      </details>
      <p>
        <A href="/">Return to Course Instances</A>
      </p>
      <Show when={pendingRemoval()}>
        {(confirmation) => (
          <RosterConfirmationDialog
            confirmation={confirmation()}
            onCancel={() => {
              const trigger = confirmation().trigger;
              setPendingRemoval(null);
              trigger.focus();
            }}
            onConfirm={async () => {
              const current = confirmation();
              setPendingRemoval(null);
              await revoke(current.rosterId);
              current.trigger.focus();
            }}
          />
        )}
      </Show>
    </PageFrame>
  );
}

/** Route changes create a new scoped collection and discard prior Course data. */
export function CourseRosterPage(): JSX.Element {
  const params = useParams();
  const course = (): CourseInstanceRouteId | null =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
  return (
    <Show
      when={course()}
      keyed
      fallback={
        <PageFrame routeSurface="courseRoster">
          <section class="route-error" role="alert">
            <p>Return to your course list, then open the roster again.</p>
          </section>
        </PageFrame>
      }
    >
      {(loadedCourse) => <CourseRosterCoursePage courseInstanceId={loadedCourse} />}
    </Show>
  );
}
