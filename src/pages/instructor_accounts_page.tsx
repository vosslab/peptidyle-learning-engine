// Live Sysadmin Instructor Account workflow; it never renders Authentication Email.

import { For, Show, createMemo, createResource, createSignal, type JSX } from "solid-js";

import type { InstructorAccountSummary } from "../api/instructor_account";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";

function stateLabel(state: InstructorAccountSummary["state"]): string {
  switch (state) {
    case "active":
      return "Active";
    case "deactivated":
      return "Deactivated";
    case "closed":
      return "Closed";
  }
}

function signInLabel(timestamp: number | null): string {
  if (timestamp === null) return "No successful sign-in recorded";
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? "Sign-in time unavailable" : date.toLocaleString();
}

function failureCopy(): string {
  return "That Instructor Account change could not be completed. Check the account state and try again.";
}

/** Sysadmin-only Account creation and lifecycle actions. */
export function InstructorAccountsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const isSysadmin = createMemo(() => {
    const current = session.state();
    return current.kind === "authenticated" && current.session.account.productRole === "sysadmin";
  });
  const [accounts, { refetch, mutate }] = createResource(isSysadmin, async (allowed) =>
    allowed ? runtime.client.listInstructorAccounts() : [],
  );
  const [email, setEmail] = createSignal("");
  const [reasonByReference, setReasonByReference] = createSignal<Record<string, string>>({});
  const [busyReference, setBusyReference] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");

  function updateAccount(updated: InstructorAccountSummary): void {
    mutate((current) =>
      (current ?? []).map((account) =>
        account.reference === updated.reference ? updated : account,
      ),
    );
  }

  function setReason(reference: string, reason: string): void {
    setReasonByReference((current) => ({ ...current, [reference]: reason }));
  }

  async function createAccount(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (creating()) return;
    const normalizedEmail = email().trim().toLowerCase();
    if (normalizedEmail.length < 3 || normalizedEmail.length > 320) {
      setError("Enter a normalized Instructor Authentication Email within its allowed length.");
      return;
    }
    setCreating(true);
    setError(null);
    try {
      const created = await runtime.client.createInstructorAccount({ normalizedEmail });
      mutate((current) => [created, ...(current ?? [])]);
      setEmail("");
      setAnnouncement("Instructor Account created.");
    } catch {
      // ASVS 5.2.4: never reflect submitted email or a transport/server body.
      setError(
        "The Instructor Account could not be created. Check the normalized email and try again.",
      );
    } finally {
      setCreating(false);
    }
  }

  async function deactivate(account: InstructorAccountSummary): Promise<void> {
    const reason = reasonByReference()[account.reference] ?? "";
    if (reason.length === 0 || reason.length > 1000 || reason !== reason.trim()) {
      setError("Enter a trimmed deactivation reason within 1,000 characters.");
      return;
    }
    setBusyReference(account.reference);
    setError(null);
    try {
      updateAccount(
        await runtime.client.deactivateInstructorAccount(account.reference, { reason }),
      );
      setReason(account.reference, "");
      setAnnouncement(`Instructor Account ${account.reference} deactivated.`);
    } catch {
      setError(failureCopy());
    } finally {
      setBusyReference(null);
    }
  }

  async function reactivate(account: InstructorAccountSummary): Promise<void> {
    setBusyReference(account.reference);
    setError(null);
    try {
      updateAccount(await runtime.client.reactivateInstructorAccount(account.reference));
      setAnnouncement(`Instructor Account ${account.reference} reactivated.`);
    } catch {
      setError(failureCopy());
    } finally {
      setBusyReference(null);
    }
  }

  return (
    <section
      class="page"
      data-route-surface="instructorAccounts"
      aria-labelledby="instructor-accounts-heading"
    >
      <p class="eyebrow">System administration</p>
      <h1 id="instructor-accounts-heading">Instructor Accounts</h1>
      <p class="page-lede">
        Create and manage Instructor Account access. This workspace intentionally lists only an
        account reference, state, and last successful sign-in.
      </p>
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
      <form
        class="auth-panel"
        aria-busy={creating()}
        novalidate
        onSubmit={(event) => void createAccount(event)}
      >
        <h2>Create Instructor Account</h2>
        <label for="instructor-account-email">
          Instructor Authentication Email
          <input
            id="instructor-account-email"
            name="normalizedEmail"
            type="email"
            value={email()}
            onInput={(event) => setEmail(event.currentTarget.value)}
            autocomplete="off"
            autocapitalize="none"
            spellcheck={false}
            maxlength={320}
            required
          />
        </label>
        <button class="primary-action" type="submit" disabled={creating()}>
          {creating() ? "Creating Instructor Account..." : "Create Instructor Account"}
        </button>
      </form>
      <Show when={error()}>
        {(message) => (
          <section class="inline-error" role="alert">
            <p>{message()}</p>
          </section>
        )}
      </Show>
      <Show when={accounts.loading}>
        <p class="loading-state">Loading Instructor Accounts...</p>
      </Show>
      <Show when={accounts.error !== undefined}>
        <section class="inline-error" role="alert">
          <p>Instructor Accounts could not load. Check your connection and try again.</p>
          <button class="quiet-action" type="button" onClick={() => void refetch()}>
            Retry
          </button>
        </section>
      </Show>
      <Show when={accounts() !== undefined && accounts.error === undefined}>
        <section aria-label="Instructor Accounts">
          <For
            each={accounts() ?? []}
            fallback={<p class="empty-state">No Instructor Accounts are available.</p>}
          >
            {(account) => (
              <article class="auth-panel">
                <h2>{account.reference}</h2>
                <p>State: {stateLabel(account.state)}</p>
                <p>Last successful sign-in: {signInLabel(account.lastSuccessfulSignIn)}</p>
                <Show when={account.state === "active"}>
                  <label for={`deactivate-reason-${account.reference}`}>
                    Deactivation reason
                    <input
                      id={`deactivate-reason-${account.reference}`}
                      name={`deactivationReason-${account.reference}`}
                      type="text"
                      value={reasonByReference()[account.reference] ?? ""}
                      onInput={(event) => setReason(account.reference, event.currentTarget.value)}
                      maxlength={1000}
                      required
                    />
                  </label>
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={busyReference() === account.reference}
                    onClick={() => void deactivate(account)}
                  >
                    {busyReference() === account.reference
                      ? "Updating..."
                      : "Deactivate Instructor Account"}
                  </button>
                </Show>
                <Show when={account.state === "deactivated"}>
                  <button
                    class="primary-action"
                    type="button"
                    disabled={busyReference() === account.reference}
                    onClick={() => void reactivate(account)}
                  >
                    {busyReference() === account.reference
                      ? "Updating..."
                      : "Reactivate Instructor Account"}
                  </button>
                </Show>
                <Show when={account.state === "closed"}>
                  <p>This Instructor Account is closed and cannot be changed here.</p>
                </Show>
              </article>
            )}
          </For>
        </section>
      </Show>
    </section>
  );
}
