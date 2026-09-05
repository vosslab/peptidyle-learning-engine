// course_theme_scope_styles.ts - route-local course-theme presentation.

export const COURSE_THEME_SCOPE_STYLES = `
.course-theme-scope {
  min-height: var(--ple-course-scope-min-block-size, calc(100vh - 5rem));
  margin: var(--ple-course-scope-edge-offset, -0.25rem);
  padding: var(--ple-course-scope-padding, 1rem);
  border-radius: var(--ple-radius-surface, 0.9rem);
  background-color: var(--ple-theme-canvas);
  background-image:
    radial-gradient(
      circle at 8% 0%,
      color-mix(
        in srgb,
        var(--ple-theme-secondary) var(--ple-course-theme-secondary-wash, 18%),
        transparent
      ),
      transparent 26rem
    ),
    radial-gradient(
      circle at 92% 0%,
      color-mix(
        in srgb,
        var(--ple-theme-accent) var(--ple-course-theme-accent-wash, 16%),
        transparent
      ),
      transparent 24rem
    ),
    linear-gradient(90deg, var(--ple-theme-secondary), var(--ple-theme-accent)),
    linear-gradient(
      180deg,
      transparent var(--ple-course-theme-fade-start, 18rem),
      var(--ple-surface) 100%
    );
  background-position: center, center, top, center;
  background-size: auto, auto, 100% var(--ple-course-theme-rail-size, 0.32rem), 100% 100%;
  background-repeat: no-repeat;
  color: var(--ple-ink);
}

.course-theme-scope > .page,
.course-theme-scope > [data-current-path] > .page {
  position: relative;
}

.course-theme-scope .course-card {
  border-inline-start: 0.28rem solid var(--ple-theme-accent);
  background: color-mix(in srgb, var(--ple-card-surface) 88%, transparent);
}

.course-theme-scope .course-card:hover {
  background: var(--ple-card-surface);
}

@media (max-width: 30rem) {
  .course-theme-scope {
    --ple-course-scope-padding: 0.75rem;
    --ple-course-theme-secondary-wash: 12%;
    --ple-course-theme-accent-wash: 12%;
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
