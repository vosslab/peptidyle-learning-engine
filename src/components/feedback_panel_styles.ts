// feedback_panel_styles.ts - styles mounted only by the server-projected feedback panel.

export const FEEDBACK_PANEL_STYLES = `
  .feedback-panel { display: grid; gap: var(--ple-space-3, 0.75rem); padding: clamp(0.875rem, 2vw, 1.25rem); border: 2px solid var(--ple-success); border-radius: var(--ple-radius-surface, 0.625rem); background: color-mix(in srgb, var(--ple-success) 7%, white); }
  .feedback-panel__heading { margin: 0; }
  .feedback-panel__status, .feedback-panel__score, .feedback-panel__empty { margin: 0; }
  .feedback-panel__section { display: grid; gap: var(--ple-space-2, 0.5rem); padding-top: var(--ple-space-3, 0.75rem); border-top: 1px solid var(--ple-border); }
  .feedback-panel__section h3 { margin: 0; }
  .feedback-panel__blocks { display: grid; gap: var(--ple-space-2, 0.5rem); }
  .feedback-panel__blocks > * { margin: 0; }
  .feedback-panel__math { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  .feedback-panel__image { display: block; max-width: 100%; height: auto; border-radius: var(--ple-radius-group, 0.5rem); }
  .feedback-panel__code { overflow-x: auto; padding: 0.65rem; border-radius: var(--ple-radius-control, 0.375rem); background: var(--ple-surface-soft); }
  .feedback-panel__table-wrap { overflow-x: auto; }
  .feedback-panel__table { width: 100%; border-collapse: collapse; }
  .feedback-panel__table th, .feedback-panel__table td { padding: 0.5rem; border: 1px solid var(--ple-border); text-align: left; vertical-align: top; }
  .feedback-panel__advance { min-height: var(--ple-response-min-height, 3.5rem); width: 100%; }
  @media (max-width: 30rem) { .feedback-panel { padding: 0.75rem; } }
  @media (prefers-reduced-motion: reduce) { .feedback-panel { scroll-behavior: auto; } }
`;
