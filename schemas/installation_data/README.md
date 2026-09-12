# PLE installation data

Installation data creates the ordinary complete Live Demo teaching environment
on the canonical PLE schema. It is selected by default for a new local or
production installation; an installation owner can opt out before
provisioning. It is not a demo schema, marker, report, special role, or
teardown subsystem. Once present, its records follow the same product lifecycle
as other teaching data.

Run the complete default operation after the base schema and its owning
services are ready:

```bash
cargo tools installation-data provision
```

`provision` runs `apply`, then creates cross-system Student Work and grading
effects through their ordinary application paths. `apply` is the narrower,
convergent database-owned operation: it creates a temporary publication
context, publishes the eight Pilot Questions through the ordinary publisher,
then runs `install.sql`. `live_demo.sql` adds ordinary Accounts, an Authoring
Workspace, Blueprint Draft and Revision, Course, roster claims, and a released
Assignment with exact Question Revision pins. Repeating `apply` converges on
that same database-owned graph.

Direct SQL is used only for state PostgreSQL owns completely. Attempts,
presentation, responses, submission, grading, statistics, and other
cross-system effects remain with their ordinary owning paths; the manifest does
not recreate those effects.

To opt out before any Live Demo data is provisioned:

```bash
cargo tools installation-data provision --without-live-demo
```

See the [Live Demo specification](../../docs/LIVE_DEMO_SPEC.md) for the
resulting product data and [installation guide](../../docs/INSTALL.md) for the
required publisher and storage environment. The local controller exposes the
same opt-out through `local_stack.py start --headless --without-live-demo`.
