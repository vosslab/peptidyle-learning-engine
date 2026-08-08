// assignment_editor_styles.ts - scoped layout for the instructor assignment editor.

export const ASSIGNMENT_EDITOR_STYLES = `
.assignment-editor-grid { display:grid; grid-template-columns:minmax(0,1.35fr) minmax(17rem,.8fr); gap:1rem; align-items:start; }
.assignment-editor-panel { padding:1.25rem; border:1px solid var(--ple-border); border-radius:1rem; background:var(--ple-surface); box-shadow:var(--ple-shadow); }
.assignment-editor-panel h2 { margin-top:0; }
.assignment-editor-field { display:grid; gap:.4rem; margin:.85rem 0; font-weight:720; }
.assignment-editor-field input, .assignment-editor-field select { min-height:3rem; padding:.55rem .65rem; border:1px solid var(--ple-border); border-radius:.55rem; background:white; color:var(--ple-ink); }
.assignment-editor-actions { display:flex; flex-wrap:wrap; gap:.75rem; align-items:center; margin-top:1rem; }
.assignment-editor-list { display:grid; gap:.75rem; margin:0; padding:0; list-style:none; }
.assignment-editor-row { padding:.85rem; border:1px solid var(--ple-border); border-radius:.7rem; background:white; }
.assignment-editor-row h3 { margin:0; font-size:1rem; }
.assignment-editor-row p { margin:.35rem 0; color:var(--ple-muted); font-size:.92rem; overflow-wrap:anywhere; }
.assignment-editor-row-actions { display:flex; flex-wrap:wrap; gap:.5rem; }
.assignment-editor-row-actions .quiet-action { min-height:2.7rem; padding:.45rem .7rem; }
.assignment-editor-policy-set { margin:.9rem 0; padding:.75rem; border:1px solid var(--ple-border); border-radius:.7rem; }
.assignment-editor-policy-set legend { padding:0 .3rem; font-weight:760; }
.assignment-editor-violations { margin:1rem 0; padding:1rem 1.1rem; border-left:4px solid var(--ple-danger); background:color-mix(in srgb, var(--ple-danger) 7%, white); }
.assignment-editor-violations h2 { margin:.1rem 0 .5rem; font-size:1.15rem; }
.assignment-editor-violations ul { margin:.5rem 0 0; padding-left:1.25rem; }
.assignment-editor-catalog-results { margin-top:1rem; }
.assignment-editor-note { color:var(--ple-muted); font-size:.92rem; }
@media (max-width:48rem) { .assignment-editor-grid { grid-template-columns:1fr; } }
`;
