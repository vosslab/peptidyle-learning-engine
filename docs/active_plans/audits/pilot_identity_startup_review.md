# Local identity bootstrap compatibility review

## Verdict

**ACCEPTED.** The startup compatibility repair now has a focused permanent
host-side regression that covers the exact stale-projection failure, its
credential-preservation boundary, and its refusal behavior. It authorizes a
repeat of the previously blocked bounded live launcher run; it does not claim
that real-stack evidence has already been obtained.

This verdict includes the follow-up path-spelling repair. The previous live
failure used the absolute spelling of the repository-default env file, which
had bypassed the literal `containers/env.local` guard and therefore skipped the
bootstrap. The launcher now maps only those two exact default spellings to the
canonical local path before every local-only guard. A same-basename custom path
remains operator-owned and is not normalized, seeded, or given credentials.

## Confirmed behavior

- `bootstrap_local_identities` reads an existing mode-0600 regular credential
  file and never rotates it. It refuses a symlink, unreadable/nonregular file,
  any mode other than `0600`, duplicate roles, an unknown/blank record, a
  missing instructor/student pair, identical credentials, and noncanonical
  base64url data before it calls the projection writer. Those failure paths
  leave `containers/local-identities.json` unchanged.
- It validates the exact 43-character unpadded base64url spelling, decodes the
  canonical 32-byte value, and hashes those decoded bytes. This agrees with
  `LocalFileIdentityProvider::verify`, which base64url-decodes the presented
  value and applies SHA-256 to the 32 raw bytes rather than to text.
- The regenerated JSON uses the server-required `learner_alias` fields
  `instructor-local` and `student-local`, fixed local tenant/user IDs, and the
  expected instructor/administrator and student role sets. The identity file
  contains hashes only; bearer values remain only in the mode-0600 credential
  file.
- The projection is generated to a same-directory `mktemp` file, mode-set to
  `0644`, then renamed over the old name. That prevents an API launch from
  observing a partially written JSON file. Repeated launchers can only publish
  complete projections derived from credentials they validated.
- The `stat -f '%Lp'` then `stat -c '%a'` branch supports macOS/BSD and GNU
  `stat`. The used `openssl base64 -d -A`, `openssl dgst -sha256 -r`, `tr`, and
  shell parsing forms are available in the project-supported macOS and GNU
  environments. `set -o pipefail` makes decode/hash failures fail closed.
- The normal local env is the only path that creates or refreshes these files.
  A custom `--env-file` is still operator-owned: it is neither populated with
  local credentials nor rewritten by this compatibility path.

`containers/local-identities.json` itself is intentionally a generated
projection: an old-mode, malformed, or symlinked projection is replaced by a
new regular mode-0644 projection only after the private credential source has
passed validation. The refusal contract above applies to that private source,
which is the durable identity owner.

## Durable regression now present

`tests/e2e/e2e_local_identity_bootstrap.sh` is the focused host-side test. It
uses only fixed non-secret fixture values in a temporary directory and does not
start Podman or read `env.local`. It proves that an aliasless stale projection
is replaced through a new inode, while both existing mode-0600 credentials
remain byte-identical. The replacement is a regular mode-0644 file with exact
aliases, local IDs, display names, role arrays, and SHA-256 values derived from
decoded credential bytes. It also proves empty successful stdout/stderr and
that neither bearer string appears in the projection.

Malformed, wrong-mode, duplicate-role, symlink, and missing credential inputs
each fail with captured output and leave a sentinel projection byte-identical.
The source's same-directory temporary-file rename is the appropriate atomic
publication primitive: concurrent readers see the complete previous projection
or the complete replacement, never a partial write.

The same test calls the normalization helper with the relative default,
absolute repository-default, and a temporary custom `env.local` path. Only the
first two become `containers/env.local`; this covers the exact runner spelling
that exposed the live startup defect without broadening the custom-env trust
boundary.

## Commands run

| Command                                                                                          | Result         |
| ------------------------------------------------------------------------------------------------ | -------------- |
| `bash -n launch_local_stack.sh`                                                                  | PASS           |
| `bash -n containers/local_identity_bootstrap.sh tests/e2e/e2e_local_identity_bootstrap.sh`       | PASS           |
| `bash tests/e2e/e2e_local_identity_bootstrap.sh`                                                 | PASS           |
| `python3 -m pytest -q tests/test_replica_compose_topology.py tests/test_local_auth_container.py` | PASS: 8 passed |
| `git diff --check`                                                                               | PASS           |

No Podman command was run and no local credential or identity-file value was
read or printed during this review.
