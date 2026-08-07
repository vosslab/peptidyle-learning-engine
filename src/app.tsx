// app.tsx - the application shell (MOD-UI-SHELL).
//
// Placeholder shell for M0. The route tree, session context, error boundaries,
// and focus conventions land in M3 against the frontend architecture contract
// frozen in M1 (WP-C9). Keep this component honest about build state rather
// than mocking screens that do not exist yet.

import type { JSX } from "solid-js";

export function App(): JSX.Element {
  return (
    <main class="shell">
      <h1>Peptidyle Learning Engine</h1>
      <p>
        Foundation build. The question model, adapters, and attempt loop are not implemented yet.
      </p>
    </main>
  );
}
