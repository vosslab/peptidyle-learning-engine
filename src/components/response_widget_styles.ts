// response_widget_styles.ts - styles mounted only by the response-widget dispatcher.

export const RESPONSE_WIDGET_STYLES = `
  .response-widget { display: grid; gap: 1rem; }
  .response-widget fieldset { display: grid; gap: 0.75rem; min-width: 0; border: 0; margin: 0; padding: 0; }
  .response-widget legend { font-weight: 700; }
  .response-widget .response-control, .response-widget .choice-card, .response-widget .order-action,
  .response-widget .primary-action, .response-widget .quiet-action { min-height: 56px; }
  .response-widget .response-control { box-sizing: border-box; width: 100%; padding: 0.75rem; }
  .response-widget .choice-list, .response-widget .ordering-list { display: grid; gap: 0.5rem; }
  .response-widget .choice-card { display: flex; align-items: center; gap: 0.75rem; box-sizing: border-box; padding: 0.75rem; cursor: pointer; }
  .response-widget .choice-card.selected { outline: 3px solid currentColor; }
  .response-widget .choice-number, .response-widget legend { font-weight: 700; }
  .response-widget .ordering-row { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; align-items: center; gap: 0.5rem; }
  .response-widget .order-action { min-width: 56px; }
  .response-widget .format-status, .response-widget .keyboard-hint, .response-widget .field-help { margin: 0; }
  .response-widget .format-status.error { font-weight: 700; }
  .response-widget .external-tool-actions { display: flex; flex-wrap: wrap; gap: 0.75rem; }
  .response-widget .external-tool-frame { width: 100%; min-height: 24rem; border: 1px solid currentColor; }
  @media (max-width: 360px) { .response-widget .ordering-row { grid-template-columns: minmax(0, 1fr) 56px 56px; } }
`;
