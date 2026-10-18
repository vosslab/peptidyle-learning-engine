// repo_root.mjs - resolve the repository root for developer commands.

import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

/**
 * Resolve the repository root from this module's location, independent of cwd.
 */
export function getRepoRoot() {
  return REPO_ROOT;
}
