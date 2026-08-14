// response_widget_styles.ts - styles mounted only by the response-widget dispatcher.

export const RESPONSE_WIDGET_STYLES = `
  .response-widget { display: grid; gap: var(--ple-space-2, 0.5rem); }
  .response-widget fieldset { display: grid; gap: 0.15rem; min-width: 0; border: 0; margin: 0; padding: 0; }
  .response-widget legend { padding: 0; font-size: 1rem; font-weight: 720; }
  .response-widget .keyboard-hint { margin: 0; color: var(--ple-muted); font-size: 0.92rem; }
  .response-widget .completion-progress { margin: 0; color: var(--ple-muted); font-size: 0.92rem; font-weight: 700; }
  .response-widget .response-control, .response-widget .choice-card, .response-widget .order-action,
  .response-widget .primary-action, .response-widget .quiet-action { min-height: var(--ple-response-min-height, 44px); padding: 0.35rem 0.65rem; }
  .response-widget .response-control { box-sizing: border-box; width: 100%; padding: 0.4rem 0.55rem; }
  .response-widget .choice-list, .response-widget .ordering-list { display: grid; gap: 0.18rem; }
  .response-widget .choice-card { display: flex; align-items: center; gap: 0.55rem; box-sizing: border-box; padding: 0.38rem 0.55rem; border: 0; border-left: 3px solid transparent; border-radius: var(--ple-radius-control, 0.45rem); background: color-mix(in srgb, var(--ple-surface-soft) 58%, transparent); color: inherit; cursor: pointer; font-weight: 580; }
  .response-widget .choice-card:hover { background: var(--ple-surface-soft); }
  .response-widget .choice-card.selected { border-left-color: var(--ple-accent-strong); background: var(--ple-surface-soft); }
  .response-widget .choice-card:has(input:focus-visible) { outline: 2px solid var(--ple-focus); outline-offset: 2px; }
  .response-widget .choice-card input { width: 1.125rem; height: 1.125rem; margin: 0; accent-color: var(--ple-accent-strong); }
  .response-widget .choice-card input:focus-visible { outline: 0; }
  .response-widget .choice-number { display: inline-grid; width: 1.45rem; height: 1.45rem; place-items: center; border-radius: 999px; background: color-mix(in srgb, var(--ple-accent) 10%, white); color: var(--ple-accent-strong); font-size: 0.76rem; font-weight: 760; }
  .response-widget .matching-progress { margin: 0; font-weight: 700; }
  .response-widget .matching-choice-content { display: grid; gap: 0.2rem; }
  .response-widget .matching-choice-state { font-size: 0.875rem; font-weight: 700; }
  .response-widget .matching-choice-card { width: 100%; color: inherit; font: inherit; text-align: left; }
  .response-widget .matching-choice-card:focus-visible { outline: 2px solid var(--ple-focus); outline-offset: 2px; }
  .response-widget .choice-card.unavailable { cursor: not-allowed; opacity: 0.7; }
  .response-widget .ordering-row { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; align-items: center; gap: 0.25rem; }
  .response-widget .order-action { min-width: 44px; }
  .response-widget .format-status, .response-widget .field-help { margin: 0; }
  .response-widget .format-status { min-height: 2.25rem; padding: 0.35rem 0.5rem; border-left: 3px solid var(--ple-accent); background: var(--ple-surface-soft); color: var(--ple-accent-strong); font-weight: 680; }
  .response-widget .format-status.ready { border-color: var(--ple-success); background: color-mix(in srgb, var(--ple-success) 7%, white); color: var(--ple-success); }
  .response-widget .format-status.error { border-color: var(--ple-danger); background: color-mix(in srgb, var(--ple-danger) 7%, white); color: var(--ple-danger); font-weight: 700; }
  .response-widget .status-spinner { display: inline-block; width: 0.9rem; height: 0.9rem; margin-right: 0.35rem; border: 2px solid currentcolor; border-right-color: transparent; border-radius: 50%; animation: spin 0.8s linear infinite; }
  .response-widget .response-actions, .response-widget .external-tool-actions { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .response-widget .external-tool-frame { width: 100%; min-height: 24rem; border: 1px solid currentColor; }
  @media (max-width: 360px) {
    .response-widget .ordering-row { grid-template-columns: minmax(0, 1fr) 44px 44px; }
    .response-widget .choice-card { padding-inline: 0.4rem; }
  }
`;
