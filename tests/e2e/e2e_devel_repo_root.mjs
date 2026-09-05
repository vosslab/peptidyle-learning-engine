// e2e_devel_repo_root.mjs - Git-worktree root resolution for developer commands.

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const { getRepoRoot } = await import(pathToFileURL(path.join(repoRoot, "devel/repo_root.mjs")));

assert.equal(
  getRepoRoot(),
  repoRoot,
  "the developer root resolver returns this Git worktree's top-level directory",
);

const originalFailure = new Error("not a Git repository");
assert.throws(
  () =>
    getRepoRoot({
      executeGit: () => {
        throw originalFailure;
      },
    }),
  (error) => {
    assert.ok(error instanceof Error);
    assert.match(error.message, /Run this command from a Git worktree\./u);
    assert.strictEqual(error.cause, originalFailure);
    return true;
  },
  "a Git failure retains its cause and gives developers an actionable next step",
);

const nonWorktreeDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "ple-repo-root-e2e-"));
try {
  const resolverModule = pathToFileURL(path.join(repoRoot, "devel/repo_root.mjs")).href;
  const probe = await import("node:child_process");
  const program = [
    `import { getRepoRoot } from ${JSON.stringify(resolverModule)};`,
    "try { getRepoRoot(); } catch (error) {",
    "  console.error(error.message);",
    "  process.exit(1);",
    "}",
  ].join("\n");
  const result = probe.spawnSync(process.execPath, ["--input-type=module", "--eval", program], {
    cwd: nonWorktreeDirectory,
    encoding: "utf8",
    env: Object.fromEntries(
      Object.entries(process.env).filter(
        ([name]) => name !== "GIT_DIR" && name !== "GIT_WORK_TREE",
      ),
    ),
  });
  assert.equal(result.status, 1, "a directory outside any Git worktree rejects root resolution");
  assert.match(
    result.stderr,
    /Cannot determine the repository root.*Run this command from a Git worktree\./su,
  );
} finally {
  fs.rmSync(nonWorktreeDirectory, { recursive: true, force: true });
}

console.log(
  "PASS: developer Git root resolution succeeds in a worktree and fails clearly outside one",
);
