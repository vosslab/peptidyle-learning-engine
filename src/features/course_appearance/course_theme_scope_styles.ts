// course_theme_scope_styles.ts - route-local course-theme presentation.

export const COURSE_THEME_SCOPE_STYLES = `
.course-theme-scope {
  padding-top: 0.875rem;
  border-radius: 0;
  background:
    linear-gradient(
        90deg,
        var(--ple-theme-secondary) 0 68%,
        var(--ple-theme-accent) 68% 100%
      )
      top / 100% 0.25rem no-repeat;
  background-color: var(--ple-surface);
  color: var(--ple-ink);
}

@media (max-width: 30rem) {
  .course-theme-scope {
    padding-top: 0.75rem;
  }
}

@media (forced-colors: active) {
  .course-theme-scope {
    border-top: 2px solid CanvasText;
    background: Canvas;
    color: CanvasText;
  }
}
`;
