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

### Inspected work names its Student and Assessment

**Decision.** The authorized Student-work inspection result includes
server-resolved Student and Assessment labels, never placing those labels in
cursors, URLs, or browser storage. Current `assignment` wire or source names are
implementation gaps and do not change the product term.

### PLE-owned wire names use direct Serde DTOs

**Decision.** PLE-owned serialized fields and portable discriminants use `snake_case`; direct generated DTOs reflect Serde while registered protocols retain their owner spelling.

### Blueprint-operation authorization

**Decision.** Course creation may create an empty Course Instance or adopt a
Public Blueprint. A Blueprint is created Private with Revision 1. Only its
owner may Save content or change its Private, Public, or Archived lifecycle.
The transport resolves its authenticated Account only through `SessionRecord`;
current operation counts and route names are implementation details rather than
product lifecycle authority.
