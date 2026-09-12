# Database authorization

This document describes the PostgreSQL authority boundary implemented by the
canonical base schema. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) defines product
authority, and [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the
meaning of product terms. This document explains how PostgreSQL enforces those
decisions.

## Principle

PLE application accounts are not PostgreSQL roles. A request authenticates an
Account, then its protected transaction installs that trusted account identity
in `ple.session_account_id`. Database functions resolve the account's active
state and exact current relationship before they read or change protected data.
Browser input, URLs, queue payloads, object addresses, and worker payloads are
untrusted scope claims, not authority.

The database is default-deny. Protected tables use forced row-level security
(RLS); schemas, tables, sequences, and functions receive explicit grants; and
the runtime has no owner, superuser, database-creation, role-creation, or
`BYPASSRLS` capability. A missing session identity or current relationship
denies the operation in the same transaction as the data access.

## Bootstrap, ownership, and migration

The platform bootstrap is deliberately outside the application. One privileged
bootstrap transaction creates every PLE PostgreSQL role: the non-login
`ple_database_owner`; four non-login schema owners
(`ple_data_owner`, `ple_private_owner`, `ple_audit_owner`, and
`ple_api_owner`); the ordinary application capabilities; the typed worker
capabilities; the no-login Unrelease executor; and the non-inheriting
`ple_migrator` login.

The base validates that complete role graph, then creates its schemas, objects,
and explicit grants. `ple_migrator` is `NOCREATEROLE` and has exactly six
direct `SET`-only memberships: the database owner, the four schema owners,
and `ple_unrelease_executor`. It has no membership in `ple_app`, `ple_auth`,
or `ple_student`.

`ple_migrator` installs the base through the small `base_schema/install.sql`
manifest and is the only principal that can write the schema-qualified SQLx
ledger, `ple_migration._sqlx_migrations`. It has no database-wide `CREATE`
privilege: its SQLx `CREATE` privilege is confined to `ple_migration` because
SQLx checks its ledger on every forward-migration run. The migration schema and
ledger are unavailable to `PUBLIC`.

The application does not read the ledger directly. `ple_api.ple_schema_state`
is a read-only, security-barrier projection of the current base release
identity and forward SQLx ledger. It reports `pre-production` today. At the
first human-approved production cutover, the SQL projection and Rust
`BASE_RELEASE_IDENTITY` change together to one immutable production-baseline
identifier recorded in [CHANGELOG.md](CHANGELOG.md). `ple_app` selects the
projection for application verification, and `ple_migrator` selects it for the
coordinator's post-install verification. The direct migrator read is limited to
that projection; it does not give a runtime capability ledger-write or DDL
authority. `ple_app` cannot alter the projection, ledger, or any schema object.

Before the first human-approved production deployment, structural corrections
belong in their owning base-schema module. That cutover freezes the base; later
structural changes are immutable SQLx forward migrations. This lifecycle rule
keeps authorization simple rather than adding another authorization mechanism.

## Runtime principals

The base provides three ordinary no-login capabilities:

| Capability | Purpose |
| --- | --- |
| `ple_app` | Authenticated application operations and read-only schema verification. |
| `ple_auth` | Session resolution and authentication operations. |
| `ple_student` | Bounded student-facing operations where a separate capability is required. |

Platform provisioning creates separate `LOGIN NOINHERIT` service identities.
Each has only the direct `SET` memberships required by its process. In
particular, `ple_api_login` can assume `ple_app` and `ple_auth`; the individual
grading and publisher logins can assume only their matching worker capability.
They are non-administrative and have neither object ownership nor unrelated
memberships. A process connects as its login and explicitly assumes its one
operation capability; it does not gain authority from a broad shared database
role.

Worker capabilities receive only the registered claim, lease, read, and commit
functions for their typed work. For example, the iMathAS grading capability
executes its two claim/commit procedures but has no direct protected-table
access. A worker locks a durable, typed lease before acting; the function checks
that the job kind, target type, and exact target agree. This prevents a queue
message or backend response from widening its scope.

## Application authority

`ple_api` functions resolve the transaction-local Account and use current
database facts for authorization:

- an active Instructor plus current Instructor membership authorizes teaching
  operations for that exact course;
- a Student owns only their current course record and derived Attempt data;
- authoring operations require their current workspace relationship, and
  Blueprint Draft operations require the current Blueprint owner relationship;
  and
- a Sysadmin is a product role, not ambient Student-record or teaching
  authority. Support access remains a separately scoped, audited capability.

Course membership, account state, availability, workspace, and Blueprint owner
facts are checked when the protected operation runs. Revoking a relationship or
deactivating an account therefore closes the relevant capability without relying
on an earlier route-level decision. The API uses non-enumerating failures for
targets that the session may not resolve.

Question and Blueprint revisions are immutable evidence. Their stable
lineages hold availability, and an archived lineage is excluded from new
selection while exact historical references continue to resolve. Assignment
state is current and protected by its qualified Assignment Edit Number; Student
Work retains the evidence needed to interpret an existing Attempt after a later
Assignment change.

## RLS and trusted function seams

Every protected relation enables and forces RLS. Policies are role-specific and
use the current Account, membership, ownership, workspace, or lease predicate
that applies to the operation. Table owners do not bypass these policies merely
because they own the table.

Some operations need a small privileged seam: immutable-event triggers,
cross-table invariants, session resolution, scoped API operations, and typed
worker commits. Those functions use `SECURITY DEFINER` only for their declared
capability, have a fixed trusted `search_path` beginning with `pg_catalog`, and
are revoked from `PUBLIC`. The calling role receives execution only where the
base grants that exact function. This keeps a necessary invariant close to the
data without turning a schema owner into a runtime identity.

## Student Work and Assignment Unrelease

Student Work is immutable to ordinary runtime roles. Its root is an Assignment
Attempt; dependent Issued Questions, Question Attempts, responses,
presentations, submissions, grading evidence, backend exchanges, pool
selections, observation receipts, and related records follow root-oriented
foreign-key cascades. Shared Questions, assets, current Assignment structure,
and course membership are outside that closure.

`ple_unrelease_executor` is a dedicated no-login capability. It owns the
single `SECURITY DEFINER` Unrelease operation and has the narrow grants needed
to lock the Assignment, read redacted impact counts, update its lifecycle
state, delete the Attempt root, rebuild affected Question Revision statistics,
and append the audit event. It is not a service login and no API or worker
capability inherits its authority.

Unrelease first locks the Assignment using the same ordering as Attempt start,
save, submission, and grading. It then verifies current teaching authority,
Released state, exact Assignment Edit Number, and exact title confirmation.
The status transition, complete Student Work closure deletion, statistics
rebuild, and redacted audit event commit together. The audit event contains the
actor, Assignment, aggregate counts, outcome, and time; it excludes Student
identities, responses, and grades. A rejected precondition leaves all of those
facts unchanged.

## Deployment boundary

The short-lived database-migrator image contains the PostgreSQL 17 client, the
base manifest, optional installation-data manifest, and forward migrations. It
is the only shipped image that carries `psql` or schema-installation material.
API and worker runtime images contain neither `psql` nor DDL authority.

Production provisioning occurs from a short-lived, audited administration
environment in the private network. It bootstraps the platform identities,
installs and verifies the base, provisions the narrowly scoped service logins,
and then deploys application processes with their own TLS-verified credentials.
The application pool verifies its login and capability contract at startup;
successful infrastructure provisioning alone is not authorization evidence.

## Verification scope

Permanent checks cover the durable security properties: default-deny grants,
forced RLS, owner and runtime-role separation, fixed-path privileged functions,
worker capability membership, restricted schema-state access, non-enumeration,
and the Unrelease closure. Fresh-installation, failed-install rollback, and
connected service exercises remain integration evidence rather than a catalog
inventory frozen into this document.
