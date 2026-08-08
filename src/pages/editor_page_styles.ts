// editor_page_styles.ts - local styles so the mock editor does not alter the application shell.

export const EDITOR_PAGE_STYLES = `
.editor-grid { display:grid; gap:1.25rem; grid-template-columns:minmax(0, 1fr) minmax(18rem, .7fr); }
.editor-panel { padding:1.25rem; border:1px solid var(--ple-border); border-radius:1rem; background:var(--ple-surface); box-shadow:var(--ple-shadow); }
.editor-panel h2 { margin-top:0; }
.editor-field { display:grid; gap:.4rem; margin:1rem 0; font-weight:720; }
.editor-field input, .editor-field textarea, .editor-field select { width:100%; padding:.7rem; border:1px solid var(--ple-border); border-radius:.55rem; background:var(--ple-surface); color:var(--ple-ink); }
.editor-field textarea { min-height:8rem; resize:vertical; }
.editor-actions { display:flex; flex-wrap:wrap; gap:.75rem; align-items:center; margin-top:1rem; }
.editor-capabilities { display:grid; gap:.6rem; margin:1rem 0; }
.editor-capability { display:flex; gap:.55rem; align-items:center; font-weight:650; }
.editor-violation { margin:.55rem 0; padding:.7rem; border-left:4px solid var(--ple-danger); background:color-mix(in srgb, var(--ple-danger) 7%, white); }
.editor-preview { min-width:0; }
.editor-preview .question-card { margin:0; }
.editor-guidance { padding:.9rem 1rem; border-left:4px solid var(--ple-accent); background:var(--ple-surface-soft); }
.editor-draft-list { display:grid; gap:.5rem; padding:0; list-style:none; }
.editor-draft-list button { width:100%; padding:.75rem; border:1px solid var(--ple-border); border-radius:.6rem; background:var(--ple-surface); color:var(--ple-ink); text-align:left; }
.editor-draft-list button[aria-current="page"] { border-color:var(--ple-accent); background:var(--ple-surface-soft); }
.editor-diff { padding-left:1.25rem; }
.instructor-preview { display:grid; gap:.75rem; margin-top:1.5rem; padding-top:1.25rem; border-top:1px solid var(--ple-border); }
.instructor-preview h3, .instructor-preview h4 { margin:0; }
.instructor-preview__card { display:grid; gap:1rem; }
@media (max-width: 48rem) { .editor-grid { grid-template-columns:1fr; } }
`;
