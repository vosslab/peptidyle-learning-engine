// course_theme_scope_styles.ts - route-local course-theme presentation.

export const COURSE_THEME_SCOPE_STYLES = `
.course-theme-scope {
  min-height: calc(100vh - 14rem);
  padding: clamp(1rem, 3vw, 2rem);
  border-radius: 1.2rem;
  background:
    linear-gradient(
        90deg,
        var(--ple-theme-secondary) 0 68%,
        var(--ple-theme-accent) 68% 100%
      )
      top / 100% 0.5rem no-repeat,
    var(--ple-surface);
  color: var(--ple-ink);
}

@media (max-width: 30rem) {
  .course-theme-scope {
    padding: 1rem 0.75rem;
    border-radius: 0.85rem;
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
