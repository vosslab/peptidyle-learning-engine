// repo_root.mjs - resolve the repository root for developer commands.

import { execFileSync } from "node:child_process";

/**
 * Resolve the Git worktree root rather than inferring it from a script location.
 */
export function getRepoRoot({ executeGit = execFileSync } = {}) {
  let output;
  try {
    output = executeGit("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(
      "Cannot determine the repository root with " +
        "`git rev-parse --show-toplevel`. Run this command from a Git worktree. " +
        `Git reported: ${detail}`,
      { cause: error },
    );
  }
  const repoRoot = output.trim();
  if (repoRoot.length === 0) {
    throw new Error("Git returned an empty repository root.");
  }
  return repoRoot;
}
