// question_response_control_styles.ts - styles loaded only by the Question Response Control dispatcher.

export const QUESTION_RESPONSE_CONTROL_STYLES = `
  .question-response-control { display: grid; gap: var(--ple-space-2, 0.5rem); }
  .question-response-control fieldset { display: grid; gap: 0.15rem; min-width: 0; border: 0; margin: 0; padding: 0; }
  .question-response-control legend { padding: 0; font-size: 1rem; font-weight: 720; }
  .question-response-control .keyboard-instructions { margin: 0; color: var(--ple-muted); font-size: 0.92rem; }
  .question-response-control .completion-progress { margin: 0; color: var(--ple-muted); font-size: 0.92rem; font-weight: 700; }
  .question-response-control .question-response-control__input, .question-response-control .choice-card, .question-response-control .order-action,
  .question-response-control .primary-action, .question-response-control .quiet-action { min-height: var(--ple-response-min-height, 44px); padding: 0.35rem 0.65rem; }
  .question-response-control .question-response-control__input { box-sizing: border-box; width: 100%; padding: 0.4rem 0.55rem; }
  .question-response-control .choice-list, .question-response-control .ordering-list { display: grid; gap: 0.18rem; }
  .question-response-control .choice-card { display: flex; align-items: center; gap: 0.55rem; box-sizing: border-box; padding: 0.38rem 0.55rem; border: 0; border-left: 3px solid transparent; border-radius: var(--ple-radius-control, 0.45rem); background: color-mix(in srgb, var(--ple-surface-soft) 58%, transparent); color: inherit; cursor: pointer; font-weight: 580; }
  .question-response-control .choice-card:hover { background: var(--ple-surface-soft); }
  .question-response-control .choice-card.selected { border-left-color: var(--ple-accent-strong); background: var(--ple-surface-soft); }
  .question-response-control .choice-card:has(input:focus-visible) { outline: 2px solid var(--ple-focus); outline-offset: 2px; }
  .question-response-control .choice-card input { width: 1.125rem; height: 1.125rem; margin: 0; accent-color: var(--ple-accent-strong); }
  .question-response-control .choice-card input:focus-visible { outline: 0; }
  .question-response-control .choice-number { display: inline-grid; width: 1.45rem; height: 1.45rem; place-items: center; border-radius: 999px; background: color-mix(in srgb, var(--ple-accent) 10%, white); color: var(--ple-accent-strong); font-size: 0.76rem; font-weight: 760; }
  .question-response-control .matching-progress { margin: 0; font-weight: 700; }
  .question-response-control .matching-choice-content { display: grid; gap: 0.2rem; }
  .question-response-control .matching-choice-state { font-size: 0.875rem; font-weight: 700; }
  .question-response-control .matching-choice-card { width: 100%; color: inherit; font: inherit; text-align: left; }
  .question-response-control .matching-choice-card:focus-visible { outline: 2px solid var(--ple-focus); outline-offset: 2px; }
  .question-response-control .choice-card.unavailable { cursor: not-allowed; opacity: 0.7; }
  .question-response-control .ordering-row { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; align-items: center; gap: 0.25rem; }
  .question-response-control .order-action { min-width: 44px; }
  .question-response-control .format-status, .question-response-control .field-help { margin: 0; }
  .question-response-control .format-status { min-height: 2.25rem; padding: 0.35rem 0.5rem; border-left: 3px solid var(--ple-accent); background: var(--ple-surface-soft); color: var(--ple-accent-strong); font-weight: 680; }
  .question-response-control .format-status.ready { border-color: var(--ple-success); background: color-mix(in srgb, var(--ple-success) 7%, white); color: var(--ple-success); }
  .question-response-control .format-status.error { border-color: var(--ple-danger); background: color-mix(in srgb, var(--ple-danger) 7%, white); color: var(--ple-danger); font-weight: 700; }
  .question-response-control .status-spinner { display: inline-block; width: 0.9rem; height: 0.9rem; margin-right: 0.35rem; border: 2px solid currentcolor; border-right-color: transparent; border-radius: 50%; animation: spin 0.8s linear infinite; }
  .question-response-control .response-actions, .question-response-control .imathas-question-backend-actions { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .question-response-control .imathas-question-backend-frame { width: 100%; min-height: 24rem; border: 1px solid currentColor; }
  @media (max-width: 360px) {
    .question-response-control .ordering-row { grid-template-columns: minmax(0, 1fr) 44px 44px; }
    .question-response-control .choice-card { padding-inline: 0.4rem; }
  }
`;
