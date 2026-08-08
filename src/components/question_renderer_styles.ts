// question_renderer_styles.ts - layout rules owned by the question renderer.

export const QUESTION_RENDERER_STYLES = `
  .question-renderer { display: grid; gap: clamp(1rem, 2vw, 2rem); min-width: 0; }
  .question-renderer__status { margin: 0; color: #344054; }
  .question-renderer__prompt { display: grid; gap: 1rem; min-width: 0; }
  .question-renderer__block { margin: 0; min-width: 0; overflow-wrap: anywhere; }
  .question-renderer__figure { display: grid; gap: 0.5rem; margin: 0; }
  .question-renderer__image { display: block; max-width: 100%; height: auto; }
  .question-renderer__math { display: inline-block; max-width: 100%; overflow-x: auto; }
  .question-renderer__code { overflow-x: auto; padding: 1rem; border-radius: 0.5rem; background: #101828; color: #f9fafb; }
  .question-renderer__table-wrap { overflow-x: auto; }
  .question-renderer table { width: 100%; border-collapse: collapse; }
  .question-renderer th, .question-renderer td { padding: 0.625rem; border: 1px solid #98a2b3; text-align: left; vertical-align: top; }
  .question-renderer__error { display: grid; gap: 0.75rem; padding: 1rem; border-left: 4px solid #b42318; background: #fef3f2; }
  .question-renderer__retry { min-height: 56px; justify-self: start; padding: 0.75rem 1rem; }
  @media (max-width: 480px) {
    .question-renderer { gap: 1rem; }
    .question-renderer th, .question-renderer td { min-width: 9rem; }
  }
  @media (min-width: 768px) {
    .question-renderer { gap: 1.5rem; }
    .question-renderer__prompt { gap: 1.25rem; }
  }
  @media (min-width: 1920px) {
    .question-renderer { max-width: 90rem; }
    .question-renderer__prompt { max-width: 76rem; }
  }
  @media (prefers-reduced-motion: reduce) {
    .question-renderer *, .question-renderer *::before, .question-renderer *::after { animation-duration: 0.01ms !important; transition-duration: 0.01ms !important; scroll-behavior: auto !important; }
  }
`;
