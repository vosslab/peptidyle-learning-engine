// course_appearance_styles.ts - responsive instructor settings presentation.

export const COURSE_APPEARANCE_STYLES = `
.course-appearance-form {
  display: grid;
  gap: 0.5rem;
  max-width: 72rem;
}

.course-appearance-section {
  min-width: 0;
  padding: 0.6rem 0 0;
  border: 0;
  border-top: 1px solid var(--ple-border);
  border-radius: 0;
  background: transparent;
}

.course-appearance-section > :first-child {
  margin-top: 0;
}

.course-appearance-section > :last-child {
  margin-bottom: 0;
}

.course-appearance-fieldset {
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
}

.course-appearance-fieldset legend {
  margin-bottom: 0.45rem;
  font-size: 1.15rem;
  font-weight: 800;
}

.course-appearance-help,
.course-appearance-file-summary,
.course-appearance-save-note {
  color: var(--ple-muted);
}

.course-appearance-theme-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 13.5rem), 1fr));
  gap: 0.3rem;
  margin-top: 0.5rem;
}

.course-appearance-theme-card {
  position: relative;
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: 0.5rem;
  min-height: var(--ple-response-min-height, 2.75rem);
  padding: 0.52rem 0.5rem 0.35rem;
  overflow: hidden;
  border: 2px solid color-mix(in srgb, var(--ple-border) 72%, transparent);
  border-radius: var(--ple-radius-control, 0.25rem);
  background:
    linear-gradient(90deg, var(--ple-theme-secondary), var(--ple-theme-accent)) top / 100% 0.22rem
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
  outline-offset: 1px;
  box-shadow: none;
}

.course-appearance-theme-card input {
  inline-size: 1.25rem;
  block-size: 1.25rem;
  margin: 0;
}

.course-appearance-theme-label {
  display: grid;
  gap: 0.2rem;
  font-weight: 750;
}

.course-appearance-swatches {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 0.2rem;
  min-height: 0.8rem;
}

.course-appearance-swatch {
  border: 1px solid color-mix(in srgb, var(--ple-ink) 24%, transparent);
  border-radius: 0.125rem;
}

.course-appearance-file {
  display: grid;
  gap: 0.3rem;
  min-width: 0;
  max-width: 38rem;
  font-weight: 750;
}

.course-appearance-file input[type="file"] {
  width: 100%;
  min-width: 0;
  min-height: var(--ple-control-min-height, 2.25rem);
  max-width: 100%;
  padding: 0.25rem;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-control, 0.25rem);
  background: var(--ple-card-surface);
  color: var(--ple-ink);
}

.course-appearance-file-actions,
.course-appearance-save-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.3rem;
  margin-top: 0.5rem;
}

.course-appearance-alt-options {
  display: grid;
  gap: 0.25rem;
  margin-top: 0.5rem;
}

.course-appearance-alt-options label {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  min-height: var(--ple-control-min-height, 2.25rem);
  padding: 0.2rem 0;
  border-radius: 0;
}

.course-appearance-alt-options input[type="radio"] {
  inline-size: 1.25rem;
  block-size: 1.25rem;
}

.course-appearance-alt-text {
  display: grid;
  gap: 0.3rem;
  max-width: 42rem;
  margin-top: 0.5rem;
  font-weight: 750;
}

.course-appearance-alt-text input {
  width: 100%;
  min-height: var(--ple-control-min-height, 2.25rem);
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-control, 0.25rem);
  background: var(--ple-card-surface);
  color: var(--ple-ink);
}

.course-appearance-field-error {
  margin: 0.35rem 0 0;
  color: var(--ple-danger, #9f1c1c);
  font-weight: 750;
}

.course-appearance-preview-theme {
  display: grid;
  gap: 0.5rem;
  min-width: 0;
  padding: 0.75rem;
  border: 0;
  border-radius: var(--ple-radius-inset, 0.7rem);
  background:
    radial-gradient(
      circle at 92% 0%,
      color-mix(in srgb, var(--ple-theme-accent) 14%, transparent),
      transparent 20rem
    ),
    linear-gradient(90deg, var(--ple-theme-secondary), var(--ple-theme-accent)) top / 100% 0.3rem
      no-repeat,
    var(--ple-theme-canvas);
  color: var(--ple-ink);
}

.course-appearance-preview {
  display: grid;
  gap: 0.55rem;
  min-width: 0;
  margin: 0;
}

.course-appearance-preview--wide {
  width: min(100%, 60rem);
}

.course-appearance-preview--narrow {
  width: min(100%, 22rem);
}

.course-appearance-preview figcaption {
  font-weight: 800;
}

.course-appearance-preview-title {
  margin: 0;
  padding: 0.35rem 0.5rem;
  border-inline-start: 0.45rem solid var(--ple-theme-accent);
  background: var(--ple-card-surface);
  color: var(--ple-ink);
  font-size: clamp(1rem, 3vw, 1.35rem);
}

.course-appearance-banner {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 1200 / 328;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-inset, 0.25rem);
  object-fit: cover;
}

.course-appearance-no-banner {
  margin: 0;
  padding: 0.5rem 0.625rem;
  border-inline-start: 0.35rem solid var(--ple-theme-secondary);
  background: var(--ple-card-surface);
}

.course-appearance-conflict,
.course-appearance-error,
.course-appearance-success {
  padding: 0.5rem 0.625rem;
  border: 0;
  border-inline-start: 4px solid var(--ple-border);
  border-radius: 0;
  background: var(--ple-surface-soft);
}

.course-appearance-conflict h2,
.course-appearance-error h2 {
  margin-top: 0;
}

@media (max-width: 30rem) {
  .course-appearance-theme-grid {
    grid-template-columns: 1fr;
  }
}

@media (forced-colors: active) {
  .course-appearance-section,
  .course-appearance-theme-card,
  .course-appearance-preview-theme,
  .course-appearance-conflict,
  .course-appearance-error,
  .course-appearance-success {
    border: 2px solid CanvasText;
    background: Canvas;
    color: CanvasText;
    forced-color-adjust: auto;
  }

  .course-appearance-theme-card:has(input:checked) {
    border: 4px double Highlight;
    box-shadow: none;
  }

  .course-appearance-swatch {
    background: Canvas !important;
    border-color: CanvasText;
  }
}

@media (prefers-reduced-motion: reduce) {
  .course-appearance-form *,
  .course-appearance-form *::before,
  .course-appearance-form *::after {
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}
`;
