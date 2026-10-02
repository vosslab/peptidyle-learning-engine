// profile_account_id.tsx - the signed-in Account's labeled public ID.

import { Show, createSignal, type JSX } from "solid-js";

import { validateCanonicalPublicId } from "../question_id";

/**
 * Students and Instructors see this labeled fact only on Profile.
 * ASVS 2.2.1: a non-canonical value is not shown or copied.
 */
export function ProfileAccountId(props: { readonly accountId: string }): JSX.Element {
  const accountId = validateCanonicalPublicId("account", props.accountId);
  const [copyStatus, setCopyStatus] = createSignal("");
  async function copyAccountId(): Promise<void> {
    if (accountId === null) return;
    try {
      await navigator.clipboard.writeText(accountId);
      setCopyStatus(`Copied ${accountId}.`);
    } catch {
      setCopyStatus(`Copy failed. Select ${accountId} and copy it manually.`);
    }
  }
  return (
    <Show when={accountId}>
      {(id) => (
        <section aria-labelledby="profile-account-id-heading">
          <h2 id="profile-account-id-heading">Account ID</h2>
          <p>
            <code aria-label={`Account ID ${id()}`}>{id()}</code>
          </p>
          <button
            class="quiet-action"
            type="button"
            aria-label={`Copy Account ID ${id()}`}
            onClick={() => void copyAccountId()}
          >
            Copy ID
          </button>
          <p class="calm-status" role="status" aria-live="polite">
            {copyStatus()}
          </p>
        </section>
      )}
    </Show>
  );
}
