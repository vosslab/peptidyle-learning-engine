// Native modal confirmation for irreversible roster actions.

import type { JSX } from "solid-js";

export type PendingRosterConfirmation = {
  readonly rosterId: string;
  readonly displayName: string;
  readonly state: "invitationPending" | "activeStudent";
  readonly trigger: HTMLButtonElement;
};

interface RosterConfirmationDialogProps {
  readonly confirmation: PendingRosterConfirmation;
  readonly onCancel: () => void;
  readonly onConfirm: () => Promise<void>;
}

function confirmationCopy(confirmation: PendingRosterConfirmation): string {
  const student = `${confirmation.displayName} (${confirmation.rosterId})`;
  if (confirmation.state === "invitationPending") {
    return `Cancel the Course invitation for ${student}. That Student can no longer claim it. You can create a new invitation later.`;
  }
  return `${student} immediately loses course access. Existing education records remain under retention.`;
}

export function RosterConfirmationDialog(props: RosterConfirmationDialogProps): JSX.Element {
  const isInvitation = (): boolean => props.confirmation.state === "invitationPending";

  return (
    <dialog
      class="confirmation-dialog"
      aria-labelledby="roster-confirmation-heading"
      aria-describedby="roster-confirmation-copy"
      ref={(element) => queueMicrotask(() => element.showModal())}
      onCancel={(event) => {
        event.preventDefault();
        props.onCancel();
      }}
    >
      <h2 id="roster-confirmation-heading">
        {isInvitation()
          ? `Cancel the invitation for ${props.confirmation.displayName}?`
          : `Remove course access for ${props.confirmation.displayName}?`}
      </h2>
      <p id="roster-confirmation-copy">{confirmationCopy(props.confirmation)}</p>
      <div class="action-row">
        <button class="quiet-action" type="button" onClick={props.onCancel}>
          Keep it
        </button>
        <button
          ref={(element) => queueMicrotask(() => element.focus())}
          class="primary-action"
          type="button"
          onClick={() => void props.onConfirm()}
        >
          {isInvitation() ? "Cancel invitation" : "Revoke course access"}
        </button>
      </div>
    </dialog>
  );
}
