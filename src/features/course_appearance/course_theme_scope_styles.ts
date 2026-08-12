// course_theme_scope_styles.ts - route-local course-theme presentation.

export const COURSE_THEME_SCOPE_STYLES = `
.course-theme-scope {
  padding: clamp(0.75rem, 2vw, 1.25rem);
  border-radius: var(--ple-radius-surface, 0.625rem);
  background:
    linear-gradient(
        90deg,
        var(--ple-theme-secondary) 0 68%,
        var(--ple-theme-accent) 68% 100%
      )
      top / 100% 0.375rem no-repeat,
    var(--ple-surface);
  color: var(--ple-ink);
}

@media (max-width: 30rem) {
  .course-theme-scope {
    padding: 0.75rem 0.5rem;
    border-radius: var(--ple-radius-group, 0.5rem);
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
