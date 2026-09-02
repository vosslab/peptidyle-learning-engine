// student_feedback_panel_styles.ts - styles mounted only by the server-projected Student Feedback panel.

export const STUDENT_FEEDBACK_PANEL_STYLES = `
  .student-feedback-panel { display: grid; gap: var(--ple-space-2, 0.5rem); padding: 0.5rem 0.625rem; border: 0; border-left: 4px solid var(--ple-success); border-radius: 0; background: color-mix(in srgb, var(--ple-success) 7%, white); }
  .student-feedback-panel__heading { margin: 0; }
  .student-feedback-panel__status, .student-feedback-panel__score, .student-feedback-panel__empty { margin: 0; }
  .student-feedback-panel__section { display: grid; gap: var(--ple-space-2, 0.5rem); padding-top: var(--ple-space-2, 0.5rem); border-top: 1px solid var(--ple-border); }
  .student-feedback-panel__section h3 { margin: 0; }
  .student-feedback-panel__blocks { display: grid; gap: var(--ple-space-2, 0.5rem); }
  .student-feedback-panel__blocks > * { margin: 0; }
  .student-feedback-panel__math { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  .student-feedback-panel__image { display: block; max-width: 100%; height: auto; border-radius: var(--ple-radius-inset, 0.5rem); }
  .student-feedback-panel__code { overflow-x: auto; padding: 0.65rem; border-radius: var(--ple-radius-control, 0.375rem); background: var(--ple-surface-soft); }
  .student-feedback-panel__table-wrap { overflow-x: auto; }
  .student-feedback-panel__table { width: 100%; border-collapse: collapse; }
  .student-feedback-panel__table th, .student-feedback-panel__table td { padding: 0.5rem; border: 1px solid var(--ple-border); text-align: left; vertical-align: top; }
  .student-feedback-panel__advance { min-height: var(--ple-response-min-height, 2.75rem); width: auto; justify-self: start; padding: 0.35rem 0.65rem; }
  @media (max-width: 30rem) { .student-feedback-panel { padding: 0.5rem; } }
  @media (prefers-reduced-motion: reduce) { .student-feedback-panel { scroll-behavior: auto; } }
`;
