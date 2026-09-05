// e2e_ribbon_destination_ledger.mjs - generated ledger delivery evidence.

import { execFileSync } from "node:child_process";
import path from "node:path";

import { getRepoRoot } from "../../devel/repo_root.mjs";

const repoRoot = getRepoRoot();
const ledgerGenerator = path.join(repoRoot, "devel/generate_ribbon_destination_ledger.mjs");

execFileSync("node", ["--import", "tsx", ledgerGenerator, "--check"], {
  cwd: repoRoot,
  stdio: "inherit",
});
