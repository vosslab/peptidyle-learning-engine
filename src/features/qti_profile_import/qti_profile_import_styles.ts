// qti_profile_import_styles.ts - scoped responsive styles for author-only QTI review.

export const QTI_PROFILE_IMPORT_STYLES = `
.qti-profile-import { min-width:0; border-block:1px solid var(--ple-border); border-radius:0; background:transparent; }
.qti-profile-import > summary { display:flex; flex-wrap:wrap; gap:.25rem .5rem; align-items:baseline; min-height:var(--ple-control-min-height,2.25rem); padding:.35rem 0; cursor:pointer; font-weight:780; }
.qti-profile-import > summary::marker { color:var(--ple-accent); }
.qti-profile-import__summary-help { color:var(--ple-muted); font-size:.9rem; font-weight:500; }
.qti-profile-import__body { display:grid; gap:.85rem; min-width:0; padding:0 0 .85rem; border-top:1px solid var(--ple-border); }
.qti-profile-import__intro { margin:.75rem 0 0; color:var(--ple-muted); }
.qti-profile-import__archive-context { margin:0; padding:.45rem .7rem; border:0; border-inline-start:3px solid var(--ple-border); border-radius:0; background:var(--ple-surface-soft); overflow-wrap:anywhere; }
.qti-profile-import__field { display:grid; gap:.4rem; min-width:0; font-weight:700; }
.qti-profile-import__field input[type="file"] { width:100%; min-width:0; min-height:var(--ple-control-min-height,2.25rem); padding:.25rem .4rem; border:1px solid var(--ple-border); border-radius:var(--ple-radius-control,.25rem); background:var(--ple-surface-soft); color:var(--ple-ink); font:inherit; }
.qti-profile-import__field-help { margin:0; color:var(--ple-muted); font-size:.9rem; font-weight:500; overflow-wrap:anywhere; }
.qti-profile-import__actions { display:flex; flex-wrap:wrap; gap:.25rem; align-items:center; }
.qti-profile-import__status, .qti-profile-import__alert { margin:0; padding:.7rem .8rem; border-left:4px solid var(--ple-accent); background:var(--ple-surface-soft); overflow-wrap:anywhere; }
.qti-profile-import__alert { border-left-color:var(--ple-danger); background:color-mix(in srgb, var(--ple-danger) 7%, white); }
.qti-profile-import__report { display:grid; gap:.85rem; min-width:0; }
.qti-profile-import__report h2, .qti-profile-import__report h3 { margin-bottom:.35rem; }
.qti-profile-import__profile { display:grid; grid-template-columns:repeat(3, minmax(0, 1fr)); gap:.5rem; margin:0; }
.qti-profile-import__profile div { min-width:0; padding:.5rem 0; border:0; border-top:1px solid var(--ple-border); border-radius:0; background:transparent; }
.qti-profile-import__profile dt { color:var(--ple-muted); font-size:.85rem; font-weight:700; }
.qti-profile-import__profile dd { margin:.2rem 0 0; overflow-wrap:anywhere; }
.qti-profile-import__notice-list, .qti-profile-import__item-notices { display:grid; gap:.45rem; margin:.4rem 0 0; padding-left:1.35rem; }
.qti-profile-import__notice-list li, .qti-profile-import__item-notices li { overflow-wrap:anywhere; }
.qti-profile-import__items { display:grid; gap:.65rem; margin:0; padding:0; list-style:none; }
.qti-profile-import__item { min-width:0; padding:.7rem .8rem; border:0; border-inline-start:3px solid var(--ple-border); border-radius:0; background:var(--ple-surface-soft); overflow-wrap:anywhere; }
.qti-profile-import__item--rejected { border-left:4px solid var(--ple-danger); }
.qti-profile-import__item--accepted { border-left:4px solid var(--ple-accent); }
.qti-profile-import__item-heading { display:flex; gap:.55rem; align-items:flex-start; }
.qti-profile-import__item-heading h4 { margin:0; min-width:0; overflow-wrap:anywhere; }
.qti-profile-import__item-icon { display:inline-grid; flex:0 0 1.45rem; width:1.45rem; height:1.45rem; place-items:center; border-radius:50%; background:var(--ple-surface); font-weight:900; }
.qti-profile-import__item-identity { margin:.35rem 0; color:var(--ple-muted); font-family:ui-monospace, SFMono-Regular, Menlo, monospace; font-size:.86rem; overflow-wrap:anywhere; }
.qti-profile-import__choice { display:flex; gap:.55rem; align-items:flex-start; margin-top:.65rem; font-weight:700; }
.qti-profile-import__review { display:grid; gap:.6rem; padding:.7rem .8rem; border:0; border-inline-start:3px solid var(--ple-border); border-radius:0; background:var(--ple-surface-soft); }
.qti-profile-import__review p { margin:0; overflow-wrap:anywhere; }
.qti-profile-import__acknowledgement { display:flex; gap:.6rem; align-items:flex-start; font-weight:700; }
.qti-profile-import__acknowledgement input { margin-top:.2rem; }
@media (max-width: 42rem) {
  .qti-profile-import__profile { grid-template-columns:1fr; }
  .qti-profile-import > summary { align-items:flex-start; }
}
`;
