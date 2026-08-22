// assignment_editor_styles.ts - compact desktop-first instructor workspace.

export const ASSIGNMENT_EDITOR_STYLES = `
.assignment-editor-save-result {
  display: grid;
  gap: 0.25rem;
  margin: 0 0 var(--ple-section-gap, 0.75rem);
  padding: var(--ple-panel-padding, 0.85rem);
  border-inline-start: 0.35rem solid var(--ple-success, #237447);
  border-radius: var(--ple-radius-group, 0.7rem);
  background: color-mix(in srgb, var(--ple-success, #237447) 12%, var(--ple-card-surface));
  box-shadow: var(--ple-shadow);
}

.assignment-editor-save-result h2,
.assignment-editor-save-result p {
  margin: 0;
}

.assignment-editor-grid {
  display: grid;
  grid-template-columns:
    minmax(var(--ple-instructor-primary-min-inline, 30rem), 1.3fr)
    minmax(var(--ple-instructor-secondary-min-inline, 24rem), 1fr);
  gap: var(--ple-layout-gap, 1rem);
  align-items: start;
}

.assignment-editor-panel {
  min-width: 0;
  padding: var(--ple-panel-padding, 0.85rem);
  border-radius: var(--ple-radius-group, 0.7rem);
  background: color-mix(in srgb, var(--ple-card-surface) 84%, transparent);
  box-shadow: var(--ple-shadow);
}

.assignment-editor-panel h2 {
  margin: 0 0 0.5rem;
}

.assignment-editor-policy-panel {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--ple-compact-gap, 0.35rem);
  margin-bottom: var(--ple-section-gap, 0.75rem);
}

.assignment-editor-policy-panel > h2,
.assignment-editor-policy-panel > .assignment-editor-run-timing,
.assignment-editor-policy-panel > .assignment-editor-note {
  grid-column: 1 / -1;
}

.assignment-editor-policy-panel > h2 {
  margin-bottom: 0;
}

.assignment-editor-panel > h3 {
  margin: 0.6rem 0 0.35rem;
}

.assignment-editor-field {
  display: grid;
  gap: var(--ple-assignment-field-gap);
  margin: var(--ple-assignment-field-margin-block) 0;
  font-size: var(--ple-assignment-field-font-size);
  font-weight: 680;
}

.assignment-editor-field input,
.assignment-editor-field select,
.assignment-editor-field textarea {
  width: 100%;
  min-height: var(--ple-control-min-height, 2.25rem);
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--ple-border-strong, var(--ple-border));
  border-radius: var(--ple-radius-control, 0.45rem);
  background: var(--ple-card-surface);
  color: var(--ple-ink);
  font: inherit;
}

.assignment-editor-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ple-compact-gap, 0.35rem);
  align-items: center;
  min-block-size: var(--ple-assignment-actions-reserved-block-size);
  margin: 0 0 var(--ple-section-gap, 0.75rem);
  padding: calc(var(--ple-panel-padding, 0.85rem) - 0.3rem);
  border-radius: var(--ple-radius-group, 0.7rem);
  background: color-mix(in srgb, var(--ple-card-surface) 94%, transparent);
  box-shadow: var(--ple-shadow);
}

.assignment-editor-list {
  display: grid;
  gap: calc(var(--ple-compact-gap, 0.35rem) - 0.15rem);
  margin: 0;
  padding: 0;
  list-style: none;
  counter-reset: question;
}

.assignment-editor-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.15rem var(--ple-row-padding-inline, 0.62rem);
  align-items: center;
  min-height: var(--ple-dense-row-min-height, 3.6rem);
  padding:
    var(--ple-row-padding-block, 0.45rem)
    calc(var(--ple-row-padding-inline, 0.62rem) - 0.12rem);
  border-radius: var(--ple-radius-control, 0.45rem);
  background: color-mix(in srgb, var(--ple-surface-soft) 52%, transparent);
  counter-increment: question;
}

.assignment-editor-row h3 {
  margin: 0;
  font-size: 0.96rem;
}

.assignment-editor-list .assignment-editor-row h3::before {
  margin-right: 0.45rem;
  color: var(--ple-muted);
  content: counter(question) ".";
  font-variant-numeric: tabular-nums;
}

.assignment-editor-row p {
  grid-column: 1 / -1;
  margin: 0.2rem 0;
  color: var(--ple-muted);
  font-size: 0.86rem;
  overflow-wrap: anywhere;
}

.assignment-editor-problem-identity {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem 0.65rem;
  margin: 0.12rem 0;
  color: var(--ple-muted);
  font-size: 0.82rem;
}

.assignment-editor-row-actions {
  display: flex;
  grid-row: 1 / span 2;
  grid-column: 2;
  gap: var(--ple-assignment-row-action-gap);
}

.assignment-editor-row-actions .quiet-action {
  min-width: var(--ple-assignment-row-action-min-size);
  min-height: var(--ple-assignment-row-action-min-size);
  padding: 0.2rem 0.42rem;
  font-size: 0.86rem;
}

.assignment-editor-row-actions .quiet-action:last-child {
  color: var(--ple-danger);
}

.assignment-editor-direct-import {
  margin-top: 0.6rem;
  border-radius: var(--ple-radius-control, 0.45rem);
  background: var(--ple-surface-soft);
}

.assignment-editor-direct-import summary {
  padding:
    var(--ple-assignment-disclosure-padding-block)
    var(--ple-assignment-disclosure-padding-inline);
  color: var(--ple-accent-strong);
  cursor: pointer;
  font-size: 0.9rem;
  font-weight: 680;
}

.assignment-editor-direct-import > div {
  padding: 0 var(--ple-assignment-disclosure-padding-inline)
    var(--ple-assignment-disclosure-padding-inline);
}

.assignment-editor-reuse {
  margin: 0.55rem 0;
  border-radius: var(--ple-radius-control, 0.45rem);
  background: color-mix(in srgb, var(--ple-theme-primary) 7%, var(--ple-card-surface));
}

.assignment-editor-reuse summary {
  padding:
    var(--ple-assignment-disclosure-padding-block)
    var(--ple-assignment-disclosure-padding-inline);
  color: var(--ple-accent-strong);
  cursor: pointer;
  font-size: 0.9rem;
  font-weight: 700;
}

.assignment-editor-reuse > div {
  padding: 0 var(--ple-assignment-disclosure-padding-inline)
    var(--ple-assignment-disclosure-padding-inline);
}

.assignment-editor-reuse-checklist {
  display: grid;
  gap: 0.15rem;
  max-height: var(--ple-reuse-checklist-block-size, 12rem);
  margin: 0.35rem 0;
  overflow: auto;
}

.assignment-editor-reuse-checklist label {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.45rem;
  align-items: start;
  padding: 0.35rem 0.4rem;
  border-radius: var(--ple-radius-control, 0.45rem);
  background: color-mix(in srgb, var(--ple-card-surface) 75%, transparent);
  cursor: pointer;
}

.assignment-editor-reuse-checklist label > span {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 0.2rem 0.65rem;
}

.assignment-editor-reuse-checklist strong {
  font-size: 0.86rem;
}

.assignment-editor-reuse-checklist small {
  color: var(--ple-muted);
  font-family: var(--ple-font-mono, ui-monospace);
}

.assignment-editor-reuse-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ple-compact-gap, 0.35rem);
  margin-top: var(--ple-assignment-field-margin-block);
}

.assignment-editor-import-success {
  color: var(--ple-success, #237447);
  font-weight: 680;
}

.assignment-editor-policy-set {
  margin: 0;
  padding: 0.5rem 0.6rem;
  border: 0;
  border-radius: var(--ple-radius-control, 0.45rem);
  background: color-mix(in srgb, var(--ple-surface-soft) 64%, transparent);
}

.assignment-editor-policy-set legend {
  padding: 0 0.2rem;
  font-size: 0.86rem;
  font-weight: 700;
}

.assignment-editor-policy-set .assignment-editor-field {
  grid-template-columns: minmax(0, 1fr);
  min-width: 0;
  margin: 0;
}

.assignment-editor-radio {
  display: inline-flex;
  align-items: center;
  gap: var(--ple-compact-gap, 0.35rem);
  min-height: var(--ple-assignment-row-action-min-size);
  margin-right: 0.8rem;
  font-size: 0.88rem;
  font-weight: 640;
}

.assignment-editor-run-timing .assignment-editor-field {
  grid-template-columns: 1fr;
  margin: 0.2rem 0 0.3rem 1.4rem;
}

.assignment-editor-violations {
  margin: 0.6rem 0;
  padding: 0.6rem 0.75rem;
  border-left: 3px solid var(--ple-danger);
  border-radius: 0 var(--ple-radius-control) var(--ple-radius-control) 0;
  background: color-mix(in srgb, var(--ple-danger) 7%, white);
}

.assignment-editor-violations h2 {
  margin: 0.1rem 0 0.4rem;
  font-size: 1.05rem;
}

.assignment-editor-violations ul {
  margin: 0.35rem 0 0;
  padding-left: 1.15rem;
}

.assignment-editor-catalog-results {
  display: grid;
  gap: var(--ple-assignment-field-gap);
  max-height: var(--ple-assignment-catalog-block-size, 15rem);
  margin-top: var(--ple-assignment-disclosure-padding-block);
  overflow: auto;
  overscroll-behavior: contain;
}

.assignment-editor-catalog-results .assignment-editor-row {
  grid-template-columns: minmax(0, 1fr) auto;
}

.assignment-editor-note {
  margin: 0.35rem 0;
  color: var(--ple-muted);
  font-size: 0.82rem;
}

@media (max-width: 60rem) {
  .assignment-editor-grid,
  .assignment-editor-policy-panel {
    grid-template-columns: 1fr;
  }

  .assignment-editor-catalog-results {
    max-height: none;
  }
}
`;
