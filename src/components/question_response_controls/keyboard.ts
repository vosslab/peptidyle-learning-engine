/**
 * Question Response Control extensions deliberately opt in only to answer-entry controls. The primary platform path
 * remains Tab or Shift+Tab to move focus, Space to select, and the explicit submission button.
 * Events from buttons, links, textareas, selects, or future embedded content retain native
 * keyboard semantics.
 */
function isResponseEntryTarget(target: EventTarget | null): target is HTMLInputElement {
  if (!(target instanceof HTMLInputElement)) return false;
  return target.type === "number" || target.type === "radio" || target.type === "checkbox";
}

function isInsideNativeDialog(target: EventTarget | null): boolean {
  return (
    typeof Element !== "undefined" &&
    target instanceof Element &&
    target.closest("dialog, [role='dialog']") !== null
  );
}

/**
 * Escape is a Question Response Control return extension except while an IME composition or native dialog
 * owns the key. Enter-to-submit is a separate opt-in extension for eligible response inputs.
 */
export function handleQuestionResponseControlKeyDown(
  event: KeyboardEvent,
  onEscape: () => void,
  submit: () => void,
  canSubmit: () => boolean,
): void {
  if (event.defaultPrevented || event.isComposing) return;

  if (event.key === "Escape") {
    if (isInsideNativeDialog(event.target)) return;
    event.preventDefault();
    onEscape();
    return;
  }

  if (event.key === "Enter" && isResponseEntryTarget(event.target) && canSubmit()) {
    event.preventDefault();
    submit();
  }
}
