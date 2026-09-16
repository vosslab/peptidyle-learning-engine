# Starter repository proposed changes

## Status

Proposed, not approved. This document records one demonstrated formatting problem for later review.
It does not authorize a PLE or starter-repository change.

## Observed problem

PLE stores a reviewed RDKit browser runtime at
`assets/author-content-dependencies/rdkit/RDKit_minimal.js`. The file is a third-party dependency,
not PLE-authored JavaScript. The vendored `check_codebase.sh` runs Prettier over every matching
JavaScript and TypeScript path. Because the vendored `.prettierignore` has no repository-specific
RDKit entry, the aggregate formatting step reports this file as needing changes.

The failure is specific and reproducible:

```text
npx prettier --check assets/author-content-dependencies/rdkit/RDKit_minimal.js
[warn] assets/author-content-dependencies/rdkit/RDKit_minimal.js
[warn] Code style issues found in the above file. Run Prettier with --write to fix.
```

Running the repository formatting check with one explicit exclusion passes:

```sh
npx prettier --check '**/*.{ts,tsx,mts,cts,js,mjs,cjs}' \
  '!assets/author-content-dependencies/**'
```

The repository-owned `package.json` applies the same exclusion to `format:write`. That prevents the
ordinary write command from rewriting RDKit, but it does not change the vendored aggregate check.

## Why not reformat RDKit

Reformatting would replace the reviewed dependency bytes with a PLE-local derivative. It would add
a large, unrelated diff and weaken direct comparison with the supplied runtime without improving
PLE behavior. The correct formatting scope is PLE-authored source, not this stored dependency.

## Proposed canonical change

First inspect the starter repository's propagation implementation. Two small canonical options
should be compared:

- Preserve an explicitly repository-owned group in `.prettierignore` while propagation seeds and
  updates the shared defaults.
- Let the canonical check accept one optional repo-owned ignore file. Prettier supports repeated
  `--ignore-path` options, so the check can name its existing defaults and the additional input
  directly without parsing or merging files.

If propagation cannot already preserve local `.prettierignore` entries safely, prefer the second
option: PLE would place `assets/author-content-dependencies/**` in the repo-owned input, and both
check and write commands would name the same inputs.

This is a proposed direction, not an implementation prescription. The propagation code has not
been inspected for this document. The exact representation should remain small and should not add
a new configuration framework.

## Tradeoffs

- Preserving repository-owned `.prettierignore` entries keeps one standard Prettier input and lets
  each repository name only demonstrated local exceptions. Propagation must distinguish shared
  defaults from local entries without retaining obsolete shared defaults.
- Adding a second ignore input makes ownership explicit and uses Prettier's supported repeated
  `--ignore-path` options. It adds one small starter convention and requires check and write
  commands to keep the same input list.
- A generic convention that excludes every vendor or dependency directory is simple, but directory
  names and review boundaries vary by repository. It could silently exempt PLE-authored code.
- Hard-coding the PLE path in the shared starter check would solve only PLE and would put a product
  detail in generic tooling.

## Review decision

The user should decide between preserved repository-owned `.prettierignore` entries and an
additional repo-owned ignore input after the starter propagation code is inspected. Until then,
the explicit negated glob is the demonstrated individual-gate workaround, and `package.json`
protects the repository's formatting write command. The vendored aggregate formatter remains
unresolved by PLE-owned files alone.
