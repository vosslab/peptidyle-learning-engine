# Focused operational design decisions

This companion to [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) keeps implementation-specific settled decisions concise.

### Dependency manifests permit current secure releases

**Decision.** Registry dependencies use an open reviewed minimum; exceptions are documented and lockfiles record reviewed resolutions.

### Repository Python has one selected interpreter

**Decision.** Repository Python commands source `source_me.sh` and invoke `python3` directly.
`pip_requirements.txt` and `pip_requirements-dev.txt` declare runtime and developer dependencies;
installation targets the selected Python 3.12 environment.

### Generated output has tracked authority

**Decision.** Ignored generated output is rebuilt from tracked authority; reviewed goldens remain tracked only when they define durable evidence.

### Local-stack replacement is scoped and inspectable

**Decision.** The Python controller owns labelled lifecycle, readiness, and bounded cleanup for the selected project.

### Gradebook Summary and Student-work inspection have one authority each

**Decision.** The Gradebook Summary is server-derived. Authorized Student-work inspection validates the exact course composite, writes its audit fact atomically, and returns an answer-free `no-store` Student-work inspection result with only the Student response and issued presentation needed for teaching.

### Inspected work names its Student and Assignment

**Decision.** The authorized Student-work inspection result includes server-resolved Student and Assignment labels, never placing those labels in cursors, URLs, or browser storage.

### PLE-owned wire names use direct Serde DTOs

**Decision.** PLE-owned serialized fields and portable discriminants use `snake_case`; direct generated DTOs reflect Serde while registered protocols retain their owner spelling.

### Blueprint-operation authorization

**Decision.** Course creation owns normal minimal Blueprint creation. The Blueprint-operation transport is closed to its six defined operations and resolves its authenticated Account only through `SessionRecord`.
