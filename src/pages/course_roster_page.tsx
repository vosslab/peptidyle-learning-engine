// Instructor course-level roster, invitation, import, policy, and manual export workflow.

import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";
import { useParams } from "@solidjs/router";

import type { AssignmentSummary } from "../api/contracts";
import type { AllowedEmailDomain, CourseRosterPage, RosterImportPreview } from "../api/enrollment";
import { newIdempotencyKey, readyRosterRows } from "../api/http_client/enrollment";
import { useApiRuntime } from "../api/runtime";

type RosterState =
  | { readonly kind: "loading" }
  | {
      readonly kind: "ready";
      readonly roster: CourseRosterPage;
      readonly assignments: ReadonlyArray<AssignmentSummary>;
    }
  | { readonly kind: "error"; readonly message: string };

interface LatestInvitationLink {
  readonly email: string;
  readonly url: string;
  readonly emailDelivery: "sent" | "notSent";
}

function rosterError(state: RosterState): string {
  return state.kind === "error" ? state.message : "";
}

function importStatusLabel(status: RosterImportPreview["rows"][number]["status"]): string {
  switch (status) {
    case "readyToInvite":
      return "Ready to invite";
    case "alreadyMember":
      return "Already enrolled";
    case "alreadyPending":
      return "Invitation pending";
    case "duplicate":
      return "Duplicate row";
    case "invalid":
      return "Invalid email or roster ID";
  }
}

function policyLines(rules: ReadonlyArray<AllowedEmailDomain>): string {
  return rules.map((rule) => `${rule.includeSubdomains ? "*." : ""}${rule.domain}`).join("\n");
}

function parsePolicyLines(value: string): ReadonlyArray<AllowedEmailDomain> {
  const rules: Array<AllowedEmailDomain> = [];
  for (const line of value.split(/\r?\n/u)) {
    const trimmed = line.trim();
    if (trimmed.length === 0) continue;
    const includeSubdomains = trimmed.startsWith("*.");
    const domain = includeSubdomains ? trimmed.slice(2) : trimmed;
    rules.push({ domain, includeSubdomains });
  }
  return rules;
}

function downloadExport(filename: string, csv: Blob): void {
  const url = URL.createObjectURL(csv);
  try {
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    link.rel = "noopener";
    link.click();
  } finally {
    queueMicrotask(() => URL.revokeObjectURL(url));
  }
}

export function CourseRosterPage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const courseId = params["courseId"];
  const [state, setState] = createSignal<RosterState>({ kind: "loading" });
  const [email, setEmail] = createSignal("");
  const [rosterId, setRosterId] = createSignal("");
  const [policyDomains, setPolicyDomains] = createSignal("");
  const [signupPosture, setSignupPosture] = createSignal<"invitationOnly" | "permittedDomains">(
    "invitationOnly",
  );
  const [selectedFile, setSelectedFile] = createSignal<File | null>(null);
  const [preview, setPreview] = createSignal<RosterImportPreview | null>(null);
  const [selectedRows, setSelectedRows] = createSignal<ReadonlySet<number>>(new Set());
  const [selectedAssignment, setSelectedAssignment] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [announcement, setAnnouncement] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const [latestInvitationLink, setLatestInvitationLink] = createSignal<LatestInvitationLink | null>(
    null,
  );
  const [inviteKey, setInviteKey] = createSignal(newIdempotencyKey());
  const [previewKey, setPreviewKey] = createSignal(newIdempotencyKey());
  const [commitKey, setCommitKey] = createSignal(newIdempotencyKey());

  const ready = createMemo(() => {
    const current = state();
    return current.kind === "ready" ? current : null;
  });

  async function load(): Promise<void> {
    if (courseId === undefined) {
      setState({ kind: "error", message: "The course route is incomplete." });
      return;
    }
    setState({ kind: "loading" });
    setError(null);
    try {
      const [roster, assignments] = await Promise.all([
        runtime.client.listCourseRoster(courseId),
        runtime.client.listAssignments(courseId),
      ]);
      setState({ kind: "ready", roster, assignments: assignments.items });
      setPolicyDomains(policyLines(roster.allowedEmailDomains));
      setSignupPosture(roster.signupPosture);
      if (selectedAssignment().length === 0 && assignments.items[0] !== undefined) {
        setSelectedAssignment(assignments.items[0].id);
      }
      setAnnouncement(
        `Roster loaded with ${roster.members.length} member${roster.members.length === 1 ? "" : "s"} and ${roster.pendingInvitations.length} pending invitation${roster.pendingInvitations.length === 1 ? "" : "s"}.`,
      );
    } catch {
      setState({ kind: "error", message: "The course roster could not load." });
    }
  }

  async function invite(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const current = ready();
    if (courseId === undefined || current === null) return;
    setBusy(true);
    setError(null);
    try {
      const invitedEmail = email();
      const accepted = await runtime.client.inviteCourseMember(
        courseId,
        invitedEmail,
        rosterId(),
        inviteKey(),
      );
      setLatestInvitationLink({
        email: invitedEmail,
        url: new URL(accepted.redemptionPath, window.location.origin).toString(),
        emailDelivery: accepted.emailDelivery,
      });
      setEmail("");
      setRosterId("");
      setInviteKey(newIdempotencyKey());
      setAnnouncement(
        accepted.emailDelivery === "sent"
          ? "Invitation email sent. A copyable backup link is ready."
          : "Invitation created. Copy the link and share it through a trusted course channel.",
      );
      await load();
    } catch {
      setError("The invitation could not be sent. Check the email, roster ID, and course policy.");
    } finally {
      setBusy(false);
    }
  }

  async function copyInvitationLink(): Promise<void> {
    const invitation = latestInvitationLink();
    if (invitation === null) return;
    setError(null);
    try {
      await navigator.clipboard.writeText(invitation.url);
      setAnnouncement("Invitation link copied.");
    } catch {
      setError("Automatic copy is unavailable. Select the invitation link and copy it manually.");
    }
  }

  async function loadMoreRoster(): Promise<void> {
    const current = ready();
    const cursor = current?.roster.nextCursor;
    if (courseId === undefined || current === null || cursor === null) return;
    setBusy(true);
    setError(null);
    try {
      const next = await runtime.client.listCourseRoster(courseId, cursor);
      if (next.rosterRevision !== current.roster.rosterRevision) {
        setAnnouncement("The roster changed while loading. The current roster was refreshed.");
        await load();
        return;
      }
      const roster = {
        ...next,
        members: [...current.roster.members, ...next.members],
        pendingInvitations: [...current.roster.pendingInvitations, ...next.pendingInvitations],
      };
      setState({ ...current, roster });
      setAnnouncement(
        `Loaded ${next.members.length + next.pendingInvitations.length} more roster entries.`,
      );
    } catch {
      setError("More roster entries could not be loaded. Try again.");
    } finally {
      setBusy(false);
    }
  }

  async function revokeInvitation(invitationId: string): Promise<void> {
    const current = ready();
    if (courseId === undefined || current === null) return;
    setBusy(true);
    setError(null);
    try {
      await runtime.client.revokeCourseInvitation(
        courseId,
        invitationId,
        current.roster.rosterRevision,
      );
      setAnnouncement("Pending invitation canceled.");
      await load();
    } catch {
      setError(
        "The roster changed before that invitation could be canceled. Reload and try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function revokeMember(memberId: string): Promise<void> {
    const current = ready();
    if (courseId === undefined || current === null) return;
    setBusy(true);
    setError(null);
    try {
      await runtime.client.revokeCourseMember(courseId, memberId, current.roster.rosterRevision);
      setAnnouncement("Course access revoked. Existing education records remain under retention.");
      await load();
    } catch {
      setError("The roster changed before access could be revoked. Reload and try again.");
    } finally {
      setBusy(false);
    }
  }

  async function savePolicy(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const current = ready();
    if (courseId === undefined || current === null) return;
    setBusy(true);
    setError(null);
    try {
      const policy = await runtime.client.replaceCourseEnrollmentPolicy(
        courseId,
        {
          allowedEmailDomains: parsePolicyLines(policyDomains()),
          signupPosture: signupPosture(),
        },
        current.roster.rosterRevision,
      );
      setPolicyDomains(policyLines(policy.allowedEmailDomains));
      setAnnouncement("Enrollment policy saved.");
      await load();
    } catch {
      setError(
        "The enrollment policy was not saved. Use exact domains such as mail.roosevelt.edu or *.example.edu.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function previewImport(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const current = ready();
    const file = selectedFile();
    if (courseId === undefined || current === null || file === null) return;
    setBusy(true);
    setError(null);
    try {
      const report = await runtime.client.previewRosterImport(
        courseId,
        file,
        current.roster.rosterRevision,
        previewKey(),
      );
      setPreview(report);
      setSelectedRows(new Set(readyRosterRows(report)));
      setPreviewKey(newIdempotencyKey());
      setCommitKey(newIdempotencyKey());
      setAnnouncement(
        `Roster preview ready. ${readyRosterRows(report).length} row${readyRosterRows(report).length === 1 ? " is" : "s are"} ready to invite.`,
      );
    } catch {
      setError(
        "The roster file could not be previewed. Use UTF-8 CSV with email,roster_id headers.",
      );
    } finally {
      setBusy(false);
    }
  }

  function toggleRow(rowNumber: number): void {
    setSelectedRows((current) => {
      const next = new Set(current);
      if (next.has(rowNumber)) next.delete(rowNumber);
      else next.add(rowNumber);
      return next;
    });
  }

  async function commitImport(): Promise<void> {
    const report = preview();
    if (courseId === undefined || report === null || selectedRows().size === 0) return;
    setBusy(true);
    setError(null);
    try {
      const committed = await runtime.client.commitRosterImport(
        courseId,
        report,
        [...selectedRows()].sort((left, right) => left - right),
        commitKey(),
      );
      setPreview(null);
      setSelectedFile(null);
      setCommitKey(newIdempotencyKey());
      setAnnouncement(
        `${committed.invitationsCreated} invitation${committed.invitationsCreated === 1 ? " was" : "s were"} sent.`,
      );
      await load();
    } catch {
      setError("The preview changed or expired before commit. Preview the file again.");
    } finally {
      setBusy(false);
    }
  }

  async function exportGrades(): Promise<void> {
    if (courseId === undefined || selectedAssignment().length === 0) return;
    setBusy(true);
    setError(null);
    try {
      const exported = await runtime.client.createManualGradeExport(courseId, selectedAssignment());
      downloadExport(exported.filename, exported.csv);
      setAnnouncement("Protected grade export downloaded.");
    } catch {
      setError("The grade export could not be prepared. Choose an assignment and try again.");
    } finally {
      setBusy(false);
    }
  }

  onMount(() => void load());

  return (
    <section class="page roster-page" data-route-surface="courseRoster">
      <p class="eyebrow">Course management</p>
      <h1>Students</h1>
      <p class="page-lede">
        Invite learners, review pending addresses, import a roster, and export grades by the
        course-scoped institutional ID.
      </p>
      <A class="quiet-link" href={`/courses/${courseId ?? ""}`}>
        Back to course
      </A>
      <p class="sr-only" role="status" aria-live="polite" aria-atomic="true">
        {announcement()}
      </p>
      <Show when={error()}>
        {(message) => (
          <p class="inline-error" role="alert">
            {message()}
          </p>
        )}
      </Show>

      <Show when={state().kind === "loading"}>
        <p class="loading-state" role="status">
          Loading course roster...
        </p>
      </Show>
      <Show when={state().kind === "error"}>
        <section class="route-error" role="alert">
          <h2>Roster unavailable</h2>
          <p>{rosterError(state())}</p>
          <button class="primary-action" type="button" onClick={() => void load()}>
            Try again
          </button>
        </section>
      </Show>

      <Show when={ready()}>
        {(current) => (
          <>
            <div class="roster-workflow-grid">
              <form class="auth-panel auth-form" onSubmit={(event) => void invite(event)}>
                <h2>Invite one student</h2>
                <label for="roster-email">Institutional email</label>
                <input
                  id="roster-email"
                  type="email"
                  autocomplete="off"
                  maxlength={320}
                  required
                  value={email()}
                  onInput={(event) => setEmail(event.currentTarget.value)}
                />
                <label for="roster-id">Institutional student ID</label>
                <input
                  id="roster-id"
                  inputmode="text"
                  maxlength={64}
                  pattern="[A-Za-z0-9._-]+"
                  required
                  value={rosterId()}
                  onInput={(event) => setRosterId(event.currentTarget.value)}
                />
                <p class="field-help">
                  This ID is course-scoped and used for manual LMS grade matching, never sign-in.
                </p>
                <p class="field-help">
                  PLE shows a one-time link after creation. You can share it through your LMS even
                  when course-invitation email is unavailable.
                </p>
                <button class="primary-action" type="submit" disabled={busy()}>
                  Create invitation
                </button>
              </form>

              <form class="auth-panel auth-form" onSubmit={(event) => void savePolicy(event)}>
                <h2>Enrollment policy</h2>
                <label for="signup-posture">How learners may join</label>
                <select
                  id="signup-posture"
                  value={signupPosture()}
                  onChange={(event) =>
                    setSignupPosture(
                      event.currentTarget.value === "permittedDomains"
                        ? "permittedDomains"
                        : "invitationOnly",
                    )
                  }
                >
                  <option value="invitationOnly">Invitation only</option>
                  <option value="permittedDomains">Invitations and permitted domains</option>
                </select>
                <label for="permitted-domains">Permitted email domains</label>
                <textarea
                  id="permitted-domains"
                  rows={4}
                  value={policyDomains()}
                  onInput={(event) => setPolicyDomains(event.currentTarget.value)}
                  aria-describedby="permitted-domains-help"
                />
                <p id="permitted-domains-help" class="field-help">
                  One exact domain per line. Prefix with *. only when subdomains are intentional.
                </p>
                <button class="quiet-action" type="submit" disabled={busy()}>
                  Save enrollment policy
                </button>
              </form>
            </div>

            <Show when={latestInvitationLink()}>
              {(invitation) => (
                <section
                  class="roster-section auth-panel auth-form"
                  aria-labelledby="share-invitation-heading"
                >
                  <h2 id="share-invitation-heading">Share this invitation</h2>
                  <p>
                    {invitation().emailDelivery === "sent"
                      ? `PLE sent an email to ${invitation().email}. This link is a backup.`
                      : `PLE did not send email to ${invitation().email}. Share this link through your LMS or another trusted course channel.`}
                  </p>
                  <label for="created-invitation-link">Invitation link</label>
                  <input
                    id="created-invitation-link"
                    type="url"
                    readonly
                    value={invitation().url}
                    aria-describedby="created-invitation-help"
                    onFocus={(event) => event.currentTarget.select()}
                  />
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={busy()}
                    onClick={() => void copyInvitationLink()}
                  >
                    Copy invitation link
                  </button>
                  <p id="created-invitation-help" class="field-help">
                    This bearer link is shown only in this page session. Copy it now, send it only
                    to the intended learner, and cancel the pending invitation if it reaches the
                    wrong person.
                  </p>
                </section>
              )}
            </Show>

            <section class="roster-section" aria-labelledby="pending-invitations-heading">
              <h2 id="pending-invitations-heading">Pending invitations</h2>
              <Show
                when={current().roster.pendingInvitations.length > 0}
                fallback={<p class="empty-state">No invitations are waiting.</p>}
              >
                <div class="roster-table-wrap">
                  <table class="roster-table">
                    <thead>
                      <tr>
                        <th scope="col">Email</th>
                        <th scope="col">Roster ID</th>
                        <th scope="col">Expires</th>
                        <th scope="col">Action</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={current().roster.pendingInvitations}>
                        {(invitation) => (
                          <tr>
                            <td>{invitation.email}</td>
                            <td>
                              <code>{invitation.rosterId}</code>
                            </td>
                            <td>
                              {new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(
                                new Date(invitation.expiresAt),
                              )}
                            </td>
                            <td>
                              <button
                                class="quiet-action"
                                type="button"
                                disabled={busy()}
                                onClick={() => void revokeInvitation(invitation.invitationId)}
                              >
                                Cancel invitation
                              </button>
                            </td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </div>
              </Show>
            </section>

            <section class="roster-section" aria-labelledby="course-members-heading">
              <h2 id="course-members-heading">Course members</h2>
              <Show
                when={current().roster.members.length > 0}
                fallback={<p class="empty-state">No students have claimed an invitation yet.</p>}
              >
                <div class="roster-table-wrap">
                  <table class="roster-table">
                    <thead>
                      <tr>
                        <th scope="col">Student</th>
                        <th scope="col">Email</th>
                        <th scope="col">Roster ID</th>
                        <th scope="col">Status</th>
                        <th scope="col">Action</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={current().roster.members}>
                        {(member) => (
                          <tr
                            tabindex={member.memberId === activatedMemberId() ? -1 : undefined}
                            ref={(element) => {
                              if (member.memberId === activatedMemberId())
                                queueMicrotask(() => element.focus());
                            }}
                          >
                            <th scope="row">{member.displayName}</th>
                            <td>
                              {member.rosterEmail ?? "Not provided"}
                            </td>
                            <td>
                              {member.rosterId === null ? "Not provided" : (
                                <code>{member.rosterId}</code>
                              )}
                            </td>
                            <td>{member.status}</td>
                            <td>
                              <Show
                                when={member.status === "active"}
                                fallback={<span>Access revoked</span>}
                              >
                                <button
                                  class="quiet-action"
                                  type="button"
                                  disabled={busy()}
                                  onClick={() => void revokeMember(member.memberId)}
                                >
                                  Revoke course access
                                </button>
                              </Show>
                            </td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </div>
              </Show>
              <Show when={current().roster.nextCursor !== null}>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={busy()}
                  onClick={() => void loadMoreRoster()}
                >
                  Load more roster entries
                </button>
              </Show>
            </section>

            <section class="roster-section auth-panel" aria-labelledby="roster-import-heading">
              <h2 id="roster-import-heading">Import a CSV roster</h2>
              <p>
                Use exactly two columns: <code>email,roster_id</code>. PLE discards the raw file
                after bounded parsing.
              </p>
              <form class="auth-form" onSubmit={(event) => void previewImport(event)}>
                <label for="roster-file">Roster CSV</label>
                <input
                  id="roster-file"
                  type="file"
                  accept=".csv,text/csv"
                  required
                  onChange={(event) => setSelectedFile(event.currentTarget.files?.[0] ?? null)}
                />
                <button
                  class="quiet-action"
                  type="submit"
                  disabled={busy() || selectedFile() === null}
                >
                  Preview roster
                </button>
              </form>
              <Show when={preview()}>
                {(report) => (
                  <div class="roster-import-preview">
                    <h3>Review before inviting</h3>
                    <div class="roster-table-wrap">
                      <table class="roster-table">
                        <thead>
                          <tr>
                            <th scope="col">Invite</th>
                            <th scope="col">CSV row</th>
                            <th scope="col">Email</th>
                            <th scope="col">Roster ID</th>
                            <th scope="col">Status</th>
                          </tr>
                        </thead>
                        <tbody>
                          <For each={report().rows}>
                            {(row) => (
                              <tr>
                                <td>
                                  <input
                                    type="checkbox"
                                    aria-label={`Invite CSV row ${row.rowNumber}`}
                                    checked={selectedRows().has(row.rowNumber)}
                                    disabled={row.status !== "readyToInvite" || busy()}
                                    onChange={() => toggleRow(row.rowNumber)}
                                  />
                                </td>
                                <td>{row.rowNumber}</td>
                                <td>{row.email ?? "Not retained"}</td>
                                <td>{row.rosterId ?? "Not retained"}</td>
                                <td>{importStatusLabel(row.status)}</td>
                              </tr>
                            )}
                          </For>
                        </tbody>
                      </table>
                    </div>
                    <button
                      class="primary-action"
                      type="button"
                      disabled={busy() || selectedRows().size === 0}
                      onClick={() => void commitImport()}
                    >
                      Send selected invitations
                    </button>
                  </div>
                )}
              </Show>
            </section>

            <section class="roster-section auth-panel" aria-labelledby="grade-export-heading">
              <h2 id="grade-export-heading">Manual LMS grade export</h2>
              <p>
                The download contains only course roster ID, roster email, display name, and the
                selected score for one assignment.
              </p>
              <label for="grade-export-assignment">Assignment</label>
              <select
                id="grade-export-assignment"
                value={selectedAssignment()}
                onChange={(event) => setSelectedAssignment(event.currentTarget.value)}
              >
                <For each={current().assignments}>
                  {(assignment) => <option value={assignment.id}>{assignment.title}</option>}
                </For>
              </select>
              <button
                class="primary-action"
                type="button"
                disabled={busy() || selectedAssignment().length === 0}
                onClick={() => void exportGrades()}
              >
                Download grade CSV
              </button>
            </section>
          </>
        )}
      </Show>
    </section>
  );
}
