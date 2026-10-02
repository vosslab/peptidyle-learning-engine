// log.ts - the central browser logging surface.
//
// eslint.config.js sets `no-console: warn` and check_codebase.sh runs ESLint
// with --max-warnings 0, so a bare console call in src/ fails the gate.
// This module is the only browser diagnostic. It prints a closed public
// phrase. An unrecognized event is withheld, and extra arguments are ignored,
// so an answer key, grade, response, or Student record cannot reach the console.

/* eslint-disable no-console */

const PUBLIC_DIAGNOSTICS = {
  clientBooting: "peptidyle client booting",
} as const;

type PublicDiagnostic = keyof typeof PUBLIC_DIAGNOSTICS;

function emit(method: "info" | "warn" | "error", event: string): void {
  const message = PUBLIC_DIAGNOSTICS[event as PublicDiagnostic];
  console[method](message === undefined ? "[ple] diagnostic withheld" : `[ple] ${message}`);
}

export const log = {
  /** Routine progress a developer wants while working. */
  info(event: PublicDiagnostic): void {
    emit("info", event);
  },

  /** A recoverable problem the user may still be able to work around. */
  warn(event: PublicDiagnostic): void {
    emit("warn", event);
  },

  /** A failure the user needs to know about. */
  error(event: PublicDiagnostic): void {
    emit("error", event);
  },
};
