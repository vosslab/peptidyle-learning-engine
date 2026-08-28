// eslint.config.local.js - consumer-owned ESLint overrides.
//
// Add repo-specific ESLint config objects here: extra browser-context globs,
// per-tool globals, or local rule tweaks. This file ships once via the noexist
// bucket and is never overwritten by propagation, so your edits survive. The
// canonical eslint.config.js imports and spreads this array AFTER its own config,
// so entries here refine or override the canonical rules.
//
// Example: give two named node tools browser globals for page.evaluate() use,
// without loosening no-undef across all tools.
//
//   import globals from "globals";
//   export default [
//     {
//       files: ["tools/scene_to_png.mjs", "tools/svg_picker/**"],
//       languageOptions: { globals: { ...globals.browser } },
//     },
//   ];

export default [
  {
    // The Rust workspace is not TypeScript source. Nothing under crates/ is
    // part of the browser client, and typed linting fails on any .ts found
    // there because those files are outside every tsconfig project.
    //
    // This matters because ts-rs writes generated TypeScript, and its default
    // output directory is inside the crate that declares the type. This repo
    // writes browser boundary types to root generated/, so a .ts file under
    // crates/ is stale output from a different generator invocation. Either
    // way it is not client code and must not gate the build.
    //
    // Container build context, Rust artifacts, and disposable wasm-bindgen
    // export-inspection glue are excluded for the same reason: they are not
    // authored app code. Root generated/api remains linted after tsgen.
    ignores: [
      ".venv/**",
      "crates/**",
      "target/**",
      "dist_wasm/**",
      "containers/**",
      "generated/wasm-export-check/**",
    ],
  },
];
