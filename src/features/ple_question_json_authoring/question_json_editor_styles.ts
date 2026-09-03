// question_json_editor_styles.ts - scoped, responsive styles for ple-question-json authoring controls.

export const PLE_QUESTION_JSON_EDITOR_STYLES = `
.ple-question-json-authoring { display:grid; gap:.85rem; min-width:0; }
.ple-question-json-authoring fieldset { min-width:0; margin:0; padding:.8rem 0 0; border:0; border-top:1px solid var(--ple-border); }
.ple-question-json-authoring legend { padding:0 .35rem; font-weight:780; }
.ple-question-json-authoring__field { display:grid; gap:.35rem; margin:.7rem 0; font-weight:700; }
.ple-question-json-authoring__field input:not([type="checkbox"]):not([type="radio"]), .ple-question-json-authoring__field textarea, .ple-question-json-authoring__field select { width:100%; min-width:0; min-height:var(--ple-control-min-height,2.25rem); padding:.35rem .5rem; border:1px solid var(--ple-border); border-radius:var(--ple-radius-control,.25rem); background:var(--ple-surface); color:var(--ple-ink); font:inherit; }
.ple-question-json-authoring__field input[type="checkbox"], .ple-question-json-authoring__field input[type="radio"] { inline-size:1.25rem; block-size:1.25rem; margin:0 .35rem 0 0; accent-color:var(--ple-accent-strong); }
.ple-question-json-authoring__field textarea { min-height:5.5rem; resize:vertical; }
.ple-question-json-authoring__help { margin:.25rem 0; color:var(--ple-muted); font-size:.92rem; font-weight:500; }
.ple-question-json-authoring__error { margin:.35rem 0; padding:.6rem .7rem; border-left:4px solid var(--ple-danger); background:color-mix(in srgb, var(--ple-danger) 7%, white); color:var(--ple-ink); font-weight:650; }
.ple-question-json-authoring__choice-list { display:grid; gap:.35rem; margin:0; padding:0; list-style:none; }
.ple-question-json-authoring__choice { min-width:0; padding:.45rem .55rem; border:0; border-inline-start:3px solid var(--ple-border); border-radius:0; background:var(--ple-surface-soft); }
.ple-question-json-authoring__choice-header, .ple-question-json-authoring__row-actions, .ple-question-json-authoring__actions { display:flex; flex-wrap:wrap; gap:.25rem; align-items:center; }
.ple-question-json-authoring__choice-header { justify-content:space-between; }
.ple-question-json-authoring__choice-title { margin:0; font-size:1rem; }
.ple-question-json-authoring__identity { color:var(--ple-muted); font-family:ui-monospace, SFMono-Regular, Menlo, monospace; overflow-wrap:anywhere; }
.ple-question-json-authoring__grid { display:grid; grid-template-columns:repeat(2, minmax(0, 1fr)); gap:.65rem; }
.ple-question-json-authoring__preview { padding:.85rem 0 0; border:0; border-top:1px solid var(--ple-border); border-radius:0; background:transparent; }
.ple-question-json-authoring__preview h3, .ple-question-json-authoring__preview h4 { margin-top:0; }
.ple-question-json-authoring__preview-choice { display:flex; gap:.4rem; align-items:flex-start; min-height:var(--ple-control-min-height,2.25rem); margin:0; padding:.35rem 0; border:0; border-bottom:1px solid var(--ple-border); border-radius:0; }
.ple-question-json-authoring__instructor-check { margin-top:.85rem; padding-top:.85rem; border-top:1px solid var(--ple-border); }
@media (max-width: 42rem) { .ple-question-json-authoring__grid { grid-template-columns:1fr; } .ple-question-json-authoring__choice-header { align-items:flex-start; } }
`;
