> **Historical review input, not current instructions.** Current authority is
> [implementation_plan.md](implementation_plan.md),
> [release_completion_plan.md](active/release_completion_plan.md), and
> [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md). [m0-results.md](m0-results.md) is concluded evidence.

Recommended 2026 architecture

Start with a containerized modular monolith, not a collection of microservices.

Internet
   |
AWS Application Load Balancer
   |
Web application container
   |-- TypeScript frontend
   |-- TypeScript API server
   |-- Rust native library
   |
PostgreSQL on Amazon RDS
   |
Object storage on Amazon S3
Separate service:
WeBWorK rendering and grading container

AWS Fargate can run containers without managing EC2 hosts, and ECS services can sit behind an Application Load Balancer for HTTP and HTTPS routing.

A modern replacement for LAMP

I would translate LAMP as:

Traditional LAMP	Peptidyle equivalent
Linux	Linux containers
Apache	AWS Application Load Balancer
MySQL	PostgreSQL on RDS
PHP	TypeScript API plus Rust libraries
JavaScript pages	TypeScript web application
Cron	ECS scheduled tasks or a worker container
Local files	S3 object storage

Apache is not obsolete. It is simply unnecessary for the main application unless a dependency specifically requires Apache.

The AWS load balancer can terminate HTTPS and route requests directly to the application container. The TypeScript server can serve the compiled frontend assets and API routes. Adding Apache or Nginx between them would initially create another configuration layer without providing much value.

Application container

I would use TypeScript for both the frontend and the HTTP server.

A reasonable structure:

apps/
   web/             React, SvelteKit, or similar
   api/             TypeScript HTTP API
packages/
   domain-types/    Shared TypeScript types
   ui/
crates/
   domain/          Rust policy and grading logic
   generators/      Algorithmic question generation
   wasm/            Browser-compatible subset

Possible TypeScript server choices:

* Fastify for a conventional API server
* Hono for a smaller web-standard API layer
* SvelteKit if the frontend and server should remain closely integrated
* Next.js if its ecosystem is valuable to the project

For your project, I would lean toward SvelteKit or Fastify, rather than making the architecture dependent on a large full-stack framework.

Where Rust belongs

Rust should not replace the entire web server at the beginning.

Use Rust for code that benefits from:

* deterministic execution
* strict type safety
* algorithmic question generation
* answer normalization
* assignment state transitions
* timing-policy calculations
* parsers and import validation
* native and WebAssembly reuse

The TypeScript server can call the compiled Rust library through a native Node binding, a separate process, or WebAssembly.

A practical division is:

TypeScript:
    HTTP
    authentication
    database access
    user interface
    course management
    backend orchestration
Rust:
    question generation
    grading policies
    attempt state machines
    deterministic randomization
    QTI parsing and validation
    shared browser/server execution

WebAssembly

Use WebAssembly selectively.

Good browser-side WASM tasks:

* previewing algorithmic questions
* deterministic parameter generation
* mathematical expression parsing
* answer-format validation
* reproducing assignment state logic

Do not place secret answers or authoritative grading logic exclusively in browser WASM. Students can download and inspect any code sent to the browser.

The server must remain authoritative for:

* final grading
* timer expiration
* assignment completion
* authorization
* access to secure question pools

Database

Use PostgreSQL, preferably Amazon RDS PostgreSQL.

I would not use SQLite for the production student-record database. SQLite can be secure on a single machine, but it becomes awkward for concurrent application containers, failover, centralized backups, access auditing, and horizontal scaling.

PostgreSQL provides:

* transactions
* concurrent access
* mature backup tooling
* structured JSON where useful
* robust permissions
* row-level security

PostgreSQL row-level security can restrict which rows a database role may read or modify. This can provide a second authorization layer beneath the application.

RDS manages routine database operations and can encrypt storage, logs, backups, read replicas, and snapshots using AES-256. TLS should also be required for database connections.

Use SQLite only for:

* local development
* tests
* command-line utilities
* temporary import processing
* isolated caches that contain no authoritative student records

FERPA

FERPA does not require a particular database or programming language. Security depends on the complete system design and operating practices.

At minimum:

* encrypt data in transit and at rest
* keep PostgreSQL in private subnets
* use narrowly scoped IAM roles
* separate instructor, student, and administrator permissions
* maintain audit logs for access and grade changes
* avoid storing unnecessary student information
* define retention and deletion policies
* use encrypted backups
* protect production access with multifactor authentication
* never write student responses or grades into ordinary application logs

AWS provides services and controls that can support a FERPA-compliant implementation, but AWS does not make an application compliant automatically. The organization remains responsible for its configuration and data practices.

WeBWorK service

A separate WeBWorK container or service is sensible.

WeBWorK has its own runtime, problem libraries, rendering behavior, and security considerations. Keep that environment isolated from the main application:

Peptidyle API
   |
Private authenticated request
   |
WeBWorK renderer/grader service
   |
Rendered problem or normalized grade result

The WeBWorK service should:

* have no public endpoint
* accept only signed internal requests
* receive a question reference, seed, and permitted context
* return rendered content or a normalized grading result
* avoid direct access to the Peptidyle database
* run with strict CPU, memory, and request-time limits

Other backends can later follow the same adapter pattern.

Initial AWS deployment

I would begin with:

1 ECS/Fargate web service
    Peptidyle TypeScript application
1 ECS/Fargate WeBWorK service
    isolated renderer and grader
1 RDS PostgreSQL database
    private and encrypted
1 S3 bucket
    exports, figures, and generated files
1 Application Load Balancer
    HTTPS entry point
CloudWatch
    operational logs and metrics
Secrets Manager
    database and backend credentials

The database should be a managed RDS service, not a database container. Containers are disposable. The primary database should have managed storage, automated backups, restore procedures, and controlled failover.

Suggested stack

Operating environment: Linux containers
Container platform:     AWS ECS with Fargate
Public HTTP entry:      AWS Application Load Balancer
Frontend:               TypeScript with SvelteKit or React
API server:             TypeScript with Fastify or SvelteKit
Domain engine:          Rust
Browser computation:    Rust compiled to WASM
Database:               PostgreSQL on Amazon RDS
Object storage:         Amazon S3
Backend services:       Separate WeBWorK container
Infrastructure:         Terraform or AWS CDK
Local development:      Docker Compose

In acronym form, your modern LAMP is effectively:

Linux containers, Application Load Balancer, managed PostgreSQL, and TypeScript plus Rust.

The most important early choice is not Apache versus Nginx. It is keeping the web application, domain engine, database,
and external question renderers behind clear boundaries while initially deploying the main application as one manageable
service.

Object storage should be designed as a core subsystem, not added later. QTI packages, images, rendered assets, exported exams, and imported archives will otherwise create awkward database and deployment problems.

I would separate three kinds of data.

1. Database records

PostgreSQL stores metadata and relationships:

* question ID
* version
* source backend
* ownership
* course and assignment references
* MIME type
* checksum
* object-storage key
* file size
* import status
* licensing metadata
* access policy
* creation and modification dates

The database should not usually store the large binary payload.

2. Object storage

S3-compatible storage holds:

* QTI ZIP packages
* images
* audio and video
* H5P packages
* DOCX and PDF exports
* imported source files
* generated previews
* archived question versions

Simple text questions can remain directly in PostgreSQL. WeBWorK problems may also remain as text or source references, depending on licensing and integration rules.

3. Derived artifacts

Generated files should be treated separately from source files:

* rendered images
* sanitized HTML
* extracted QTI resources
* thumbnails
* preview PDFs
* student-specific exam exports

These can usually be regenerated. That affects retention, backup, and cache policy.

Recommended object model

Do not use user-provided filenames as storage keys.

Use immutable keys based on stable IDs and versions:

questions/{question_id}/versions/{version_id}/source/qti-package.zip
questions/{question_id}/versions/{version_id}/assets/{asset_id}.png
questions/{question_id}/versions/{version_id}/derived/preview.html
exports/{organization_id}/{export_id}/exam.pdf

Each object record should include:

interface StoredObject {
	id: string
	bucket: string
	key: string
	sha256: string
	sizeBytes: number
	mediaType: string
	category: "source" | "asset" | "derived" | "export"
	createdAt: string
}

The checksum is important for deduplication, corruption checks, and reproducibility.

QTI packages

A QTI ZIP should remain preserved as the original source package.

During import:

1. Store the original ZIP unchanged.
2. Validate the ZIP structure.
3. Scan for unsafe paths and unexpected files.
4. Extract into a temporary isolated workspace.
5. Parse the manifest.
6. Store each referenced asset as a separate object.
7. Convert supported question content into the internal model.
8. Record unsupported features without silently discarding them.
9. Preserve the original package for re-export or later parser improvements.

Do not serve files directly from extracted ZIP paths. ZIP packages can contain path traversal attacks, duplicate filenames, malformed XML, oversized decompression output, or executable content.

Images and embedded resources

Every image should have a stable internal asset ID. Question HTML should reference platform asset URLs rather than raw S3 URLs.

For example:

<img src="/api/assets/01J...">

The API can then:

* check authorization
* issue a short-lived signed URL
* log access
* apply content headers
* prevent direct bucket exposure

For public open-content questions, a CDN path may eventually be appropriate. Secure instructor-only content should remain behind authorization.

Versioning

Question content should be immutable after publication.

Editing a question should create a new version:

question
  version 1
  version 2
  version 3

Each version points to its exact assets and source package. This allows old student attempts to remain reproducible.

An image replacement should not overwrite the old image. It should create a new asset object with a new checksum and key.

Deduplication

Many QTI packages may contain identical images. Content-addressed storage can reduce duplication:

objects/sha256/ab/cd/abcdef...

The database can map several asset records to the same physical object.

I would not make content-addressed storage visible to the application everywhere. Keep a stable logical asset ID and hide the physical storage strategy behind an object service.

Security boundaries

The object bucket should be private.

Use:

* server-side encryption
* strict IAM roles
* short-lived signed URLs
* separate buckets or prefixes for source, derived, and exports
* malware scanning for uploaded archives
* MIME validation
* maximum archive size
* maximum expanded size
* file-count limits
* extension and content checks
* lifecycle rules
* audit logs

Never trust the MIME type supplied by the browser or QTI package.

FERPA considerations

Question content itself may not be FERPA data. Student-specific exports, uploaded responses, annotated exams, and grading artifacts may contain educational records.

I would separate those from reusable content:

content bucket
student-record bucket
temporary-processing bucket

That allows different retention, access, and logging policies.

Student-specific generated files should have explicit expiration and deletion policies.

Local development

Use an S3-compatible service locally, such as MinIO, rather than writing files to arbitrary local directories. This keeps development behavior close to production.

Docker Compose
  web
  api
  postgres
  minio
  webwork

The application should access storage through one internal interface:

interface ObjectStore {
	put(input: PutObjectInput): Promise<StoredObject>
	get(id: string): Promise<ReadableStream>
	delete(id: string): Promise<void>
	createSignedUrl(id: string, expiresIn: number): Promise<string>
}

This keeps AWS S3, MinIO, or another provider replaceable.

Decisions to make before implementation

The important design questions are:

1. Which content stays directly in PostgreSQL?
2. Which formats are preserved as original archives?
3. Are extracted package assets immutable?
4. How are asset references rewritten during import?
5. How are question versions linked to assets?
6. Which files are authoritative and which are derived?
7. Which objects may be public, signed, or instructor-only?
8. What gets backed up and for how long?
9. How are orphaned objects detected and removed?
10. How are licensing and attribution stored with each asset?

The strongest default is:

PostgreSQL stores meaning and relationships. Object storage stores files. Every published question version points to immutable objects by ID and checksum.

That decision should be fixed before building the QTI importer, question editor, or exam exporter.
