// assessment_attempt_summary_styles.ts - read-only Attempt history layout only.

export const ASSESSMENT_ATTEMPT_SUMMARY_STYLES = `
  .attempt-history.attempt-summary {
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
  }
  .attempt-history .attempt-history__score,
  .attempt-history .attempt-history__question-header,
  .attempt-history .attempt-history__result {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--ple-space-1) var(--ple-space-3);
  }
  .attempt-history .attempt-history__score > *,
  .attempt-history .attempt-history__question-header > * {
    margin: 0;
  }
  .attempt-history .attempt-summary__question {
    display: grid;
    gap: var(--ple-space-2);
    min-inline-size: 0;
    padding-block: var(--ple-space-3);
    border-block-start: 1px solid var(--ple-border);
    overflow-wrap: anywhere;
  }
  .attempt-history .attempt-history__response,
  .attempt-history .attempt-summary__disclosure {
    display: grid;
    gap: var(--ple-space-1);
    min-inline-size: 0;
  }
  .attempt-history h4,
  .attempt-history .attempt-history__response > p {
    margin: 0;
  }
  .attempt-history .student-feedback-panel__blocks {
    display: grid;
    gap: var(--ple-space-2);
    min-inline-size: 0;
  }
  .attempt-history .student-feedback-panel__blocks > * { margin: 0; }
  .attempt-history .student-feedback-panel__image {
    display: block;
    max-inline-size: 100%;
    block-size: auto;
  }
  .attempt-history .student-feedback-panel__math { font-family: var(--ple-font-mono); }
  .attempt-history .student-feedback-panel__code {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .attempt-history .student-feedback-panel__table-wrap {
    min-inline-size: 0;
    overflow-x: auto;
  }
  .attempt-history .student-feedback-panel__table { border-collapse: collapse; }
  .attempt-history .student-feedback-panel__table th,
  .attempt-history .student-feedback-panel__table td {
    padding: var(--ple-space-1) var(--ple-space-2);
    border: 1px solid var(--ple-border);
    text-align: start;
    vertical-align: top;
  }
`;
