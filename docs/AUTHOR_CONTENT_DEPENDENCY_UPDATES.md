# Author-content dependency updates

`rdkit` is a closed author-content library identifier. It is not a package
specifier, URL, CDN choice, or author-controlled path. The current reviewed
official package, local bytes, and generated server registry are the only
runtime authority.

## Current-release refresh

Human Guidance requires current dependencies for security fixes. A refresh is
a reviewed source change that resolves the current official `@rdkit/rdkit`
release and replaces PLE's current local runtime. It is not a historical
version catalog and does not bind a package version or file digest to an
Attempt, Student Work record, or author-content descriptor.

Before accepting a refresh, the maintainer records the official provenance and
BSD-3-Clause license, verifies npm integrity and the two allowed JS/WASM file
hashes from a clean download, reviews browser compatibility, and runs the
isolated-frame proof against locally served bytes. The proof must show no CDN,
API, or other fallback request.

The vendoring command then replaces the current local pair and generated
registry. It rejects a source that does not match the reviewed package,
license, integrity, or exact two-file runtime surface. The normal offline
check is:

```bash
node devel/sync_author_content_dependency.mjs --check
```

The runtime fails closed when the current artifact is unavailable or malformed.
It never falls back to a CDN, author URL, old dependency release, or generic
asset path.
