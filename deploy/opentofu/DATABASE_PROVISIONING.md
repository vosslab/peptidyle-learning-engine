# Production database provisioning contract

OpenTofu creates the encrypted, private RDS instance. It never receives a
migration password, application password, or CA bundle. Run the following
workflow from a short-lived, audited administration environment in the private
VPC; do not run it from an ECS API or worker task.

1. Obtain the RDS-managed master secret through break-glass access, create the
   migration login, and run `cargo tools database migrate` with only
   `PLE_MIGRATION_DATABASE_URL`.
2. Apply the repository migration role/grant baseline. Create exactly
   `ple_api_login`, `ple_worker_login`, `ple_accepted_submission_recovery_login`,
   `ple_accepted_submission_fast_path_login`, `ple_publisher_login`, and
   `ple_grading_reader` with the
   memberships and attributes the production pool verifier attests. `ple_api_login` has direct
   `SET`-only membership in `ple_app` and `ple_auth`. `ple_worker_login` has direct `SET`-only
   membership in `ple_app` for ordinary worker jobs. `ple_accepted_submission_recovery_login`
   has only direct `SET`-only membership in `ple_accepted_submission_execution`; the execution
   capability is worker-only and grants the sealed accepted-submission loader without direct
   private-table access. `ple_publisher_login` has only `SET` membership in
   `ple_public_asset_publisher`. `ple_accepted_submission_fast_path_login` has only
   `SET`-only membership in `ple_accepted_submission_execution_fast_path`; this capability
   grants exact-target accepted-submission execution to the API fast path.
   Every service login is `LOGIN`, `NOINHERIT`, non-administrative, and lacks `BYPASSRLS`, schema
   creation, table ownership, and unrelated memberships. ECS uses IAM only for AWS APIs and
   PostgreSQL logins are separate credentials.
3. Create five Secrets Manager JSON values: the API value holds only the names
   selected by `local.api_required_secret_keys` and enabled feature groups;
   the worker value holds only `PLE_WORKER_DATABASE_URL` and `PLE_AUTOMATED_GRADING_DATABASE_URL`.
   The recovery value holds only `PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL` for
   `ple_accepted_submission_recovery_login`. The fast-path value holds only
   `PLE_ACCEPTED_SUBMISSION_FAST_PATH_DATABASE_URL` for
   `ple_accepted_submission_fast_path_login`. The publisher value holds only
   `PLE_PUBLISHER_DATABASE_URL`. Each JSON value has its own CMK; its key policy permits
   only the matching ECS execution role through Secrets Manager (`kms:ViaService` and the
   exact secret encryption context), never the RDS master-secret CMK. The recovery and
   fast-path secrets are distinct from their ordinary process secrets even though the owning
   task reads both. Each URL uses `sslmode=verify-full`, exact login name, RDS endpoint hostname,
   and the reviewed RDS CA root bundled into the application image.
4. Before service rollout, probe TLS certificate validation, execute the
   application pool capability checks, prove RLS isolation with each task and capability
   login URL, and rotate the RDS master secret independently of every application secret.
5. Establish human authority only from the same short-lived audited
   administration environment. After real-person validation and account-email
   verification, an operator may set one exact account's `platform_roles` to
   `["sysadmin"]` or add one direct `Instructor` course membership. Record the
   operator, verified person, reason, target account/course, timestamp, and
   resulting database identities in the deployment change record. Never
   grant either role from the API login, an email string alone, an invitation,
   or browser input. The application grants on `ple_account` deliberately
   exclude `platform_roles` updates. The canonical human role boundary is
   [USER_ROLES.md](../../docs/USER_ROLES.md).

The deployment must stop if any step fails. A successful `tofu apply` is not
database authorization evidence.
