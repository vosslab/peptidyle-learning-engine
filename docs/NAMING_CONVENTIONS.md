# Naming conventions

This file is the canonical naming policy for Peptidyle Learning Engine. The vendored language and
repository style guides remain authoritative for their general rules; this file resolves PLE's
cross-language and cross-runtime boundaries.

## Core rule

Name an identifier for the boundary that owns it, and convert it once at that boundary. Preserve a
registered or frozen external name exactly. When PLE owns a portable or server-side identifier and
no stronger boundary convention applies, prefer readable lowercase `snake_case`.

Human-role names follow [USER_ROLES.md](USER_ROLES.md): **Student**, **Instructor**, and
**Sysadmin**. PLE-owned identifiers use `student`, `instructor`, and `sysadmin` for those people and
their role-bound work. `user` names a generic authenticated identity before course authority is
known. `learning` remains valid for educational-system concepts; `learner` is not a role alias in
new PLE-owned names. Temporary Instructor-roadmap coordination keys use `WP-INST-*`. They exist only
while the owning plan is active and can disappear when that plan closes. Product, API, evidence,
and persistence identifiers use domain names instead.

`UpperCamelCase` is reserved for type-like and component objects. Ordinary TypeScript and browser
names use `lowerCamelCase`; the initial lowercase letter distinguishes that form from the reserved
type-like form. This browser convention follows the naming used by DOM Web APIs, SolidJS, TypeScript,
and generated clients. PLE uses `snake_case` everywhere that ecosystem convention does not provide
a stronger reason.

## Naming matrix

| Boundary                                              | Convention                             | PLE example                       |
| ----------------------------------------------------- | -------------------------------------- | --------------------------------- |
| Rust modules, functions, fields, and locals           | `snake_case`                           | `preview_assignment_fast_forward` |
| Rust types, traits, and enum variants                 | `UpperCamelCase`                       | `AssignmentFastForwardDecision`   |
| TypeScript functions, locals, properties, and signals | `lowerCamelCase`                       | `prepareFastForward`              |
| TypeScript types, interfaces, classes, and components | `UpperCamelCase`                       | `CurriculumAdoptionPage`          |
| Python modules, functions, locals, and fixtures       | `snake_case`                           | `origin_receipt_from_file`        |
| Python classes                                        | `UpperCamelCase`                       | `ScenarioRunReceipt`              |
| PostgreSQL identifiers                                | `snake_case`                           | `curriculum_adoption_receipt`     |
| Browser JSON fields and union discriminants           | `lowerCamelCase`                       | `importRevision`, `fastForward`   |
| Cross-runtime symbolic values and JSON map keys       | `snake_case`                           | `fresh_elena`                     |
| Static URL segments and CSS class names               | lowercase kebab case                   | `course-blueprints`               |
| Environment variables and constants                   | `SCREAMING_SNAKE_CASE`                 | `PLE_LIVE_DEMO_BROWSER_INPUT`     |
| Public domain references                              | registered uppercase prefix plus value | `C-11`, `AC-2`                    |
| Temporary work-package labels                         | plan-scoped uppercase hyphen form      | `WP-INST-B2`, `WP-R0`             |

## Why these conventions

- `snake_case` is PLE's readable default and already aligns Rust, Python, PostgreSQL, migrations,
  evidence IDs, and operational scripts.
- `lowerCamelCase` keeps browser code interoperable with DOM Web APIs, SolidJS, TypeScript libraries,
  and generated clients without a second local dialect.
- `UpperCamelCase` makes type-like objects and UI components visibly distinct from runtime values.
- Lowercase kebab case follows URL, HTML, CSS, and Compose conventions where punctuation separates
  words more naturally than underscores.
- `SCREAMING_SNAKE_CASE` makes process-level configuration and compile-time constants conspicuous.
- One conversion at an owning boundary prevents handwritten aliases and casing drift between
  runtimes.
- Registered public IDs retain their exact forms because stability is more valuable than local
  stylistic uniformity. Work-package keys follow their active plan namespace and may be renamed
  atomically while they remain temporary planning metadata.

## Boundary distinctions

- A JSON object's field name follows the browser wire rule. A symbolic identifier carried as a
  field value or map key follows the cross-runtime rule. For example, the field is
  `observedContexts`, while one context key is `fresh_elena`.
- Serde owns Rust-to-browser field and discriminant conversion. Browser consumers use generated
  lower-camel contracts directly. Consumers do not maintain handwritten casing aliases.
- Generated contract field names match the wire exactly. Rust source fields remain snake case.
- Frozen contract changes follow the atomic change rule in [CONTRACTS.md](CONTRACTS.md).
- Acronyms follow the owning language's normal word rules. Use `Uuid`, not `UUID`, in an
  `UpperCamelCase` Rust or TypeScript type name unless an external API freezes another form.

## Human-role vocabulary convergence

Use dependency order to move existing PLE-owned role names to the canonical vocabulary:

1. New contracts use `Student`, `Instructor`, and `Sysadmin` immediately.
2. Question-model and browser assignment types converge on `StudentAssignment*` together.
3. Disclosure contracts converge on `StudentDisclosure*` as one model-to-client change.
4. Work-routing and submission-status Stores converge on `StudentWork*` together with their server
   consumers.
5. PostgreSQL role-bearing names converge through forward migrations before the first schema
   freeze; accepted migrations remain immutable evidence of the schema path.

Each step owns its source, generated contracts, focused validation, and documentation as one atomic
change. `learning` remains the correct adjective for system concepts such as learning data and
learning outcomes.

## TypeScript and browser

- Use `lowerCamelCase` for ordinary functions, methods, variables, properties, props, signals, and
  browser-owned query parameters.
- Use `UpperCamelCase` only for components, classes, interfaces, types, and type-like domain
  constructors.
- Use lowercase kebab case for static URL path segments, CSS classes, CSS custom properties, and
  authored HTML `data-*` attribute names.
- Use `lowerCamelCase` for PLE-owned JSON fields and tagged-union discriminants. Keep symbolic
  values and map keys in `snake_case` when they cross runtimes or enter durable evidence.
- Native DOM, Web API, framework, and dependency names retain their upstream spelling.

## Rust

- Use `snake_case` for crates, modules, functions, methods, fields, and locals.
- Use `UpperCamelCase` for structs, enums, enum variants, traits, and type aliases.
- Use `SCREAMING_SNAKE_CASE` for constants and statics.
- Put browser wire conversion on the owning Serde type with one explicit rename policy.
- Keep raw `wasm-bindgen` snake-case exports behind the TypeScript Wasm boundary.

## Python and shell

- Use `snake_case` for Python modules, functions, methods, locals, fixtures, and command names.
- Use `UpperCamelCase` for Python classes and typed record objects.
- Use `SCREAMING_SNAKE_CASE` for Python constants and exported shell environment variables.
- Use lowercase `snake_case` for PLE-owned shell variables that remain inside one script.

## PostgreSQL

- Use unquoted lowercase `snake_case` for schemas, tables, columns, views, functions, triggers,
  policies, constraints, indexes, and roles. PostgreSQL case folding must not create a second
  identifier form.
- Name entity and relationship keys for their domain object, such as `tenant_id`, `course_id`, and
  `assignment_id`. Use `public_id` for a separately exposed public locator.
- Name timestamps for their event or state transition with an `_at` suffix, such as `created_at`,
  `updated_at`, `occurred_at`, `expires_at`, and `revoked_at`.
- Name serialized documents with a `_payload` suffix and their SHA-256 companions with
  `_payload_sha256`, such as `report_payload` and `report_payload_sha256`.
- Use `revision` for one row or aggregate's optimistic revision. Qualify other revision and
  generation counters by subject, such as `schedule_revision` and `scoring_generation`.
- Lead tenant-owned primary keys, foreign keys, and important indexes with `tenant_id` when the
  relationship permits it.
- Prefix constraint and index names with the owning relation. Use the established PostgreSQL
  suffixes `_pkey`, `_fkey`, `_key`, `_check`, and `_idx`.
- Prefix PLE-owned functions and roles with `ple_`. Contract-versioned functions end in `_v1` or
  their registered version.
- A JSON or JSONB column name follows the PostgreSQL rule. Keys inside the document follow the
  contract that owns that document; browser and Rust bridge documents retain their registered
  lower-camel field names.

## Files and operations

- Durable Markdown reference files directly under `docs/` use `SCREAMING_SNAKE_CASE.md`.
- Working documents under `docs/active_plans/` use lowercase `snake_case.md`.
- Authored non-Markdown filenames use lowercase ASCII `snake_case`. Generated TypeScript files under
  `generated/api/` retain generator-owned `UpperCamelCase.ts` names.
- Migration filenames use a sortable numeric allocation followed by a lowercase snake-case
  description, such as `2026081847_curriculum_adoption_public_bridge.sql`.
- Compose project, service, network, and container-facing names use lowercase kebab case when the
  owning Compose field permits it, such as `ple-live-demo-browser`.
- PLE scenario IDs, evidence context IDs, durable namespaces, and similar machine-selected names use
  lowercase `snake_case`. Human-visible labels remain plain language.

## Review checklist

Before adding or changing a name, verify:

1. The owning boundary is clear.
2. One canonical spelling crosses that boundary.
3. Generated code derives from its owner.
4. Portable symbolic values use `snake_case`.
5. Registered public references and frozen external names remain exact.
