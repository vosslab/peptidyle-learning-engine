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

.course-appearance-theme-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 14rem), 1fr));
  gap: var(--ple-compact-gap, 0.5rem);
  margin-block-start: var(--ple-compact-gap, 0.5rem);
}

.course-appearance-theme-card {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.45rem 0.6rem;
  align-items: center;
  min-block-size: var(--ple-control-min-height, 2.25rem);
  padding: 0.55rem;
  border: 2px solid color-mix(in srgb, var(--ple-border) 72%, transparent);
  border-radius: var(--ple-radius-control, 0.25rem);
  background:
    linear-gradient(90deg, var(--ple-theme-secondary), var(--ple-theme-accent)) top / 100% 0.25rem
      no-repeat,
    var(--ple-theme-canvas);
  color: var(--ple-ink);
  cursor: pointer;
}

.course-appearance-theme-card:has(input:checked) {
  border-color: var(--ple-accent-strong);
  box-shadow: 0 0 0 3px var(--ple-theme-canvas);
}

.course-appearance-theme-card:focus-within {
  outline: 2px solid var(--ple-focus);
  outline-offset: 2px;
}

.course-appearance-theme-card input {
  inline-size: 1.25rem;
  block-size: 1.25rem;
  margin: 0;
}

.course-appearance-theme-label {
  font-weight: 750;
}

.course-appearance-palette-preview {
  display: grid;
  grid-column: 1 / -1;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--ple-ink) 24%, transparent);
  border-radius: var(--ple-radius-control, 0.25rem);
  font-size: 0.75rem;
  font-weight: 700;
  text-align: center;
}

.course-appearance-palette-preview span {
  min-inline-size: 0;
  padding: 0.2rem;
}

.course-appearance-palette-canvas {
  background: var(--ple-theme-canvas);
  color: var(--ple-ink);
}

.course-appearance-palette-secondary {
  background: var(--ple-theme-secondary);
  color: var(--ple-theme-on-secondary);
}

.course-appearance-palette-accent {
  /* Raw habitat accents are decorative anchors, not guaranteed text surfaces. */
  background: color-mix(in srgb, var(--ple-theme-accent) 12%, var(--ple-theme-canvas));
  color: var(--ple-ink);
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
  object-fit: cover;
  object-position: center;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-control, 0.25rem);
}

/* The hero stays small and centered, matching the 1200 by 200 rendition ratio. */
.course-appearance-banner-hero {
  aspect-ratio: 6 / 1;
  block-size: clamp(120px, calc(100cqi / 6), 200px);
}

.course-appearance-banner-card {
  aspect-ratio: 5 / 2;
}

.course-appearance-banner-course-name {
  color: var(--ple-muted);
  font-weight: 750;
  text-align: center;
}

@media (max-width: 30rem) {
  .course-appearance-theme-grid {
    grid-template-columns: 1fr;
  }
}

@media (forced-colors: active) {
  .course-appearance-theme-card,
  .course-appearance-section,
  .course-appearance-error,
  .course-appearance-alternative-text,
  .course-appearance-banner-image {
    border-color: CanvasText;
    background: Canvas;
    color: CanvasText;
  }

  .course-appearance-theme-card:has(input:checked) {
    border: 4px double Highlight;
    box-shadow: none;
  }
}
`;
