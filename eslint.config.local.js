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
    // redirects that output to src/api/generated/ via .cargo/config.toml, so
    // a .ts file under crates/ is stale output from a run that predates that
    // setting, or from a crate built with the variable unset. Either way it is
    // not client code and must not gate the build.
    //
    // Container build context and Rust artifacts are excluded for the same
    // reason: they are not the app.
    ignores: ["crates/**", "target/**", "dist_wasm/**", "containers/**"],
  },
];
