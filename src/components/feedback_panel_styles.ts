// feedback_panel_styles.ts - styles mounted only by the server-projected feedback panel.

export const FEEDBACK_PANEL_STYLES = `
  .feedback-panel { display: grid; gap: 1.25rem; padding: clamp(1.25rem, 3vw, 2rem); border: 2px solid var(--ple-success); border-radius: 1rem; background: color-mix(in srgb, #176b3a 7%, white); }
  .feedback-panel__heading { margin: 0; }
  .feedback-panel__status, .feedback-panel__score, .feedback-panel__empty { margin: 0; }
  .feedback-panel__section { display: grid; gap: 0.75rem; padding-top: 1rem; border-top: 1px solid var(--ple-border); }
  .feedback-panel__section h3 { margin: 0; }
  .feedback-panel__blocks { display: grid; gap: 0.75rem; }
  .feedback-panel__blocks > * { margin: 0; }
  .feedback-panel__math { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  .feedback-panel__image { display: block; max-width: 100%; height: auto; border-radius: 0.5rem; }
  .feedback-panel__code { overflow-x: auto; padding: 0.75rem; border-radius: 0.5rem; background: var(--ple-surface-soft); }
  .feedback-panel__table-wrap { overflow-x: auto; }
  .feedback-panel__table { width: 100%; border-collapse: collapse; }
  .feedback-panel__table th, .feedback-panel__table td { padding: 0.5rem; border: 1px solid var(--ple-border); text-align: left; vertical-align: top; }
  .feedback-panel__advance { min-height: 3.5rem; width: 100%; }
  @media (max-width: 30rem) { .feedback-panel { padding: 1rem; } }
  @media (prefers-reduced-motion: reduce) { .feedback-panel { scroll-behavior: auto; } }
`;
