// assessment_attempt_summary_styles.ts - read-only Attempt summary layout.
// The page content class is attempt-summary. BEM names stay attempt-history__*.

export const ASSESSMENT_ATTEMPT_SUMMARY_STYLES = `
  .attempt-summary {
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
  }
  .attempt-summary .attempt-history__score,
  .attempt-summary .attempt-history__question-header,
  .attempt-summary .attempt-history__result {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--ple-space-1) var(--ple-space-3);
  }
  .attempt-summary .attempt-history__score > *,
  .attempt-summary .attempt-history__question-header > * {
    margin: 0;
  }
  .attempt-summary .attempt-summary__question {
    display: grid;
    gap: var(--ple-space-2);
    min-inline-size: 0;
    padding-block: var(--ple-space-3);
    border-block-start: 1px solid var(--ple-border);
    overflow-wrap: anywhere;
  }
  .attempt-summary .attempt-history__response,
  .attempt-summary .attempt-summary__disclosure {
    display: grid;
    gap: var(--ple-space-1);
    min-inline-size: 0;
  }
  .attempt-summary h4,
  .attempt-summary .attempt-history__response > p {
    margin: 0;
  }
  .attempt-summary .student-feedback-panel__blocks {
    display: grid;
    gap: var(--ple-space-2);
    min-inline-size: 0;
  }
  .attempt-summary .student-feedback-panel__blocks > * { margin: 0; }
  .attempt-summary .student-feedback-panel__image {
    display: block;
    max-inline-size: 100%;
    block-size: auto;
  }
  .attempt-summary .student-feedback-panel__math { font-family: var(--ple-font-mono); }
  .attempt-summary .attempt-history__answer-review {
    display: block;
    inline-size: 100%;
    border: 0;
  }
  .attempt-summary .student-feedback-panel__code {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .attempt-summary .student-feedback-panel__table-wrap {
    min-inline-size: 0;
    overflow-x: auto;
  }
  .attempt-summary .student-feedback-panel__table { border-collapse: collapse; }
  .attempt-summary .student-feedback-panel__table th,
  .attempt-summary .student-feedback-panel__table td {
    padding: var(--ple-space-1) var(--ple-space-2);
    border: 1px solid var(--ple-border-strong);
    text-align: start;
    vertical-align: top;
  }
`;
