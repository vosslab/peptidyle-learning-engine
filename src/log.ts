// log.ts - the central browser logging surface.
//
// Why this module exists: eslint.config.js sets `no-console: warn` and
// check_codebase.sh runs ESLint with --max-warnings 0, so a bare console call
// in src/ fails the gate. Routing through here gives one place to add a level
// filter, a transport, or redaction later without touching call sites.
//
// Redaction matters for this product specifically: an answer key or a grading
// decision must never reach a browser log. Anything logged here is visible to
// the student, so treat it as public output.

/* eslint-disable no-console */

type LogArgs = readonly unknown[];

export const log = {
  /** Routine progress a developer wants while working. */
  info(message: string, ...args: LogArgs): void {
    console.info(`[ple] ${message}`, ...args);
  },

  /** A recoverable problem the user may still be able to work around. */
  warn(message: string, ...args: LogArgs): void {
    console.warn(`[ple] ${message}`, ...args);
  },

  /** A failure the user needs to know about. */
  error(message: string, ...args: LogArgs): void {
    console.error(`[ple] ${message}`, ...args);
  },
};
