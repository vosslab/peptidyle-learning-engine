// assignment_editor_styles.ts - scoped layout for the instructor assignment editor.

export const ASSIGNMENT_EDITOR_STYLES = `
.assignment-editor-grid { display:grid; grid-template-columns:minmax(0,1.35fr) minmax(17rem,.8fr); gap:var(--ple-space-3,.75rem); align-items:start; }
.assignment-editor-panel { padding:var(--ple-space-4,1rem); border:1px solid var(--ple-border); border-radius:var(--ple-radius-surface,.625rem); background:var(--ple-surface); }
.assignment-editor-panel h2 { margin-top:0; }
.assignment-editor-field { display:grid; gap:.35rem; margin:.7rem 0; font-weight:720; }
.assignment-editor-field input, .assignment-editor-field select, .assignment-editor-field textarea { min-height:var(--ple-control-min-height,3rem); padding:.5rem .6rem; border:1px solid var(--ple-border); border-radius:var(--ple-radius-control,.375rem); background:white; color:var(--ple-ink); font:inherit; }
.assignment-editor-actions { display:flex; flex-wrap:wrap; gap:var(--ple-space-2,.5rem); align-items:center; margin-top:var(--ple-space-3,.75rem); }
.assignment-editor-list { display:grid; gap:var(--ple-space-2,.5rem); margin:0; padding:0; list-style:none; }
.assignment-editor-row { padding:.65rem .75rem; border:1px solid var(--ple-border); border-radius:var(--ple-radius-group,.5rem); background:white; }
.assignment-editor-row h3 { margin:0; font-size:1rem; }
.assignment-editor-row p { margin:.35rem 0; color:var(--ple-muted); font-size:.92rem; overflow-wrap:anywhere; }
.assignment-editor-problem-identity { align-items:center; display:flex; flex-wrap:wrap; gap:.45rem .75rem; margin:.35rem 0; color:var(--ple-muted); font-size:.92rem; }
.assignment-editor-direct-import { margin-top:var(--ple-space-3,.75rem); padding:.65rem 0 .65rem .75rem; border:0; border-left:3px solid var(--ple-accent); background:color-mix(in srgb, var(--ple-accent) 4%, white); }
.assignment-editor-direct-import h3 { margin-top:0; }
.assignment-editor-import-success { color:var(--ple-success, #176b3a); font-weight:700; }
.assignment-editor-row-actions { display:flex; flex-wrap:wrap; gap:var(--ple-space-2,.5rem); }
.assignment-editor-row-actions .quiet-action { min-height:2.75rem; padding:.4rem .65rem; }
.assignment-editor-policy-set { margin:.75rem 0; padding:.65rem 0 0; border:0; border-top:1px solid var(--ple-border); }
.assignment-editor-policy-set legend { padding:0 .3rem; font-weight:760; }
.assignment-editor-radio { display:flex; align-items:center; gap:.5rem; min-height:2.75rem; font-weight:720; }
.assignment-editor-run-timing .assignment-editor-field { margin:.25rem 0 .7rem 1.8rem; }
.assignment-editor-violations { margin:.75rem 0; padding:.65rem .75rem; border-left:4px solid var(--ple-danger); background:color-mix(in srgb, var(--ple-danger) 7%, white); }
.assignment-editor-violations h2 { margin:.1rem 0 .5rem; font-size:1.15rem; }
.assignment-editor-violations ul { margin:.5rem 0 0; padding-left:1.25rem; }
.assignment-editor-catalog-results { margin-top:.75rem; }
.assignment-editor-note { color:var(--ple-muted); font-size:.92rem; }
@media (max-width:48rem) { .assignment-editor-grid { grid-template-columns:1fr; } }
`;
