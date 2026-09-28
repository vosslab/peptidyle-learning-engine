// course_appearance_styles.ts - responsive Instructor theme selection presentation.

export const COURSE_APPEARANCE_STYLES = `
.course-appearance-form {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: var(--ple-compact-gap, 0.5rem);
  min-inline-size: 0;
  max-inline-size: 72rem;
}

.course-appearance-section {
  min-inline-size: 0;
  padding-block-start: var(--ple-compact-gap, 0.5rem);
  border-block-start: 1px solid var(--ple-border);
}

.course-appearance-fieldset {
  min-inline-size: 0;
  margin: 0;
  padding: 0;
  border: 0;
}

.course-appearance-fieldset legend {
  margin-block-end: 0.45rem;
  font-size: 1.15rem;
  font-weight: 800;
}

.course-appearance-help,
.course-appearance-save-note {
  color: var(--ple-muted);
}

.course-appearance-save-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ple-compact-gap, 0.5rem);
  align-items: center;
  margin-block-start: var(--ple-compact-gap, 0.5rem);
}

.course-appearance-error {
  margin: 0;
  padding: 0.45rem 0.6rem;
  border-inline-start: 0.25rem solid var(--ple-danger, #9f1c1c);
  background: var(--ple-surface-soft);
  color: var(--ple-ink);
}

.course-appearance-file-label,
.course-appearance-alternative-text label {
  display: grid;
  gap: 0.35rem;
  margin-block-start: var(--ple-compact-gap, 0.5rem);
  font-weight: 700;
}

.course-appearance-file-label input,
.course-appearance-alternative-text input[type="text"] {
  max-inline-size: 42rem;
}

.course-appearance-alternative-text {
  display: grid;
  gap: 0.35rem;
  margin-block: 0.8rem 0;
  padding: 0.65rem;
  border: 1px solid var(--ple-border);
}

.course-appearance-alternative-text > label {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  margin-block-start: 0;
}

.course-appearance-alternative-text > label:has(input[type="text"]) {
  display: grid;
}

.course-appearance-banner-preview-group {
  container-type: inline-size;
  display: grid;
  gap: 0.4rem;
  inline-size: 100%;
  min-inline-size: 0;
  max-inline-size: 75rem;
  margin: 0.9rem auto 0;
}

.course-appearance-banner-preview-group > p {
  margin: 0;
}

.course-appearance-banner-image {
  display: block;
  min-inline-size: 0;
  inline-size: 100%;
  object-fit: contain;
  object-position: center;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-control, 0.25rem);
}

.course-appearance-banner-course-name {
  color: var(--ple-muted);
  font-weight: 750;
  text-align: center;
}

@media (forced-colors: active) {
  .course-appearance-section,
  .course-appearance-error,
  .course-appearance-alternative-text,
  .course-appearance-banner-image {
    border-color: CanvasText;
    background: Canvas;
    color: CanvasText;
  }

}
`;
