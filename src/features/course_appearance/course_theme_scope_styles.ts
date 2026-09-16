// course_theme_scope_styles.ts - route-local course-theme presentation.

export const COURSE_THEME_SCOPE_STYLES = `
.course-theme-scope {
  margin: var(--ple-course-scope-edge-offset, -0.25rem);
  padding: var(--ple-course-scope-padding, 1rem);
  border-radius: 0;
  background-color: var(--ple-surface);
  background-image: linear-gradient(90deg, var(--ple-theme-secondary), var(--ple-theme-accent));
  background-position: top;
  background-size: 100% var(--ple-course-theme-rail-size, 0.32rem);
  background-repeat: no-repeat;
  color: var(--ple-ink);
}

.course-theme-scope > .page,
.course-theme-scope > [data-current-path] > .page {
  position: relative;
}

.course-theme-scope .course-card {
  border-inline-start: 0.28rem solid var(--ple-theme-accent);
  background: var(--ple-card-surface);
}

.course-theme-scope .course-card:hover {
  background: var(--ple-card-surface);
}

@media (max-width: 30rem) {
  .course-theme-scope {
    --ple-course-scope-padding: 0.75rem;
  }
}

@media (forced-colors: active) {
  .course-theme-scope {
    border: 2px solid CanvasText;
    background: Canvas;
    color: CanvasText;
  }
}
`;
