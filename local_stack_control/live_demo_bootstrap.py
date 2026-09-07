"""Install the fixed Live Demo objects and database baseline."""

import base64

import local_stack_control.compose
import local_stack_control.disposable_stack_adapter
import local_stack_control.lifecycle_commands
import local_stack_control.live_demo_gateway
import local_stack_control.live_demo_seed
import local_stack_control.models
import local_stack_control.process


LIVE_DEMO_ACCOUNT_SEEDS = (
	(account.setting, account.account_id, account.product_role)
	for account in local_stack_control.live_demo_seed.SEEDED_ACCOUNTS
)


#============================================
def seed_live_demo_source_objects(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Put immutable private Question Source bytes before recording their facts."""
	for question in local_stack_control.live_demo_seed.SEEDED_PUBLISHED_QUESTIONS:
		object_path = local_stack_control.live_demo_seed.source_object_path(question)
		stat_argv = local_stack_control.compose.compose_argv(
			target,
			[
				"exec", "-T", "minio", "/bin/sh", "-ec",
				"mc alias set seeded http://127.0.0.1:9000 \"$MINIO_ROOT_USER\" \"$MINIO_ROOT_PASSWORD\" >/dev/null; exec mc stat \"$1\"",
				"seeded-stat", f"seeded/private-content/{object_path}",
			],
		)
		stat = runner.run(
			stat_argv,
			local_stack_control.lifecycle_commands.child_environment(target),
			target.repo_root,
		)
		if stat.returncode == 0:
			continue
		if stat.returncode != 1:
			raise local_stack_control.models.ControllerError(
				"live-demo Question Source storage inspection did not complete"
			)
		metadata = local_stack_control.live_demo_seed.object_record_metadata(target.repo_root, question)
		put_argv = local_stack_control.compose.compose_argv(
			target,
			[
				"run", "--rm", "--no-deps", "-T", "--entrypoint", "/bin/sh",
				"createbuckets", "-ec",
				"mc alias set seeded http://minio:9000 \"$MINIO_ROOT_USER\" \"$MINIO_ROOT_PASSWORD\" >/dev/null; object_file=; trap 'rm -f \"$object_file\"' EXIT; object_file=$(mktemp /tmp/live-demo-question-source.XXXXXX); chmod 600 \"$object_file\"; cat >\"$object_file\"; mc cp --disable-multipart --custom-header \"Content-Type: $2\" --attr \"ple-record-v1=$1\" \"$object_file\" \"$3\" >/dev/null",
				"seeded-copy",
				metadata,
				local_stack_control.live_demo_seed.PLE_QUESTION_JSON_MEDIA_TYPE,
				f"seeded/private-content/{object_path}",
			],
		)
		local_stack_control.lifecycle_commands.require_command(
			runner.run(
				put_argv,
				local_stack_control.lifecycle_commands.child_environment(target),
				target.repo_root,
				local_stack_control.live_demo_seed.source_bytes(target.repo_root, question).decode("utf-8"),
			),
			"live-demo Question Source initialization",
			local_stack_control.disposable_stack_adapter.private_environment_values(target.env_file),
		)
	local_stack_control.live_demo_seed.require_question_asset_baseline(target.repo_root)
	asset_path = local_stack_control.live_demo_seed.restricted_question_asset_object_path()
	stat_argv = local_stack_control.compose.compose_argv(
		target,
		[
			"exec", "-T", "minio", "/bin/sh", "-ec",
			"mc alias set seeded http://127.0.0.1:9000 \"$MINIO_ROOT_USER\" \"$MINIO_ROOT_PASSWORD\" >/dev/null; exec mc stat \"$1\"",
			"seeded-asset-stat", f"seeded/private-content/{asset_path}",
		],
	)
	stat = runner.run(
		stat_argv,
		local_stack_control.lifecycle_commands.child_environment(target),
		target.repo_root,
	)
	if stat.returncode == 0:
		return
	if stat.returncode != 1:
		raise local_stack_control.models.ControllerError(
			"live-demo Question Asset storage inspection did not complete"
		)
	metadata = local_stack_control.live_demo_seed.question_asset_record_metadata(target.repo_root)
	put_argv = local_stack_control.compose.compose_argv(
		target,
		[
			"run", "--rm", "--no-deps", "-T", "--entrypoint", "/bin/sh",
			"createbuckets", "-ec",
			"mc alias set seeded http://minio:9000 \"$MINIO_ROOT_USER\" \"$MINIO_ROOT_PASSWORD\" >/dev/null; object_file=; trap 'rm -f \"$object_file\"' EXIT; object_file=$(mktemp /tmp/live-demo-question-asset.XXXXXX); chmod 600 \"$object_file\"; base64 -d >\"$object_file\"; mc cp --disable-multipart --custom-header \"Content-Type: $2\" --attr \"ple-record-v1=$1\" \"$object_file\" \"$3\" >/dev/null",
			"seeded-asset-copy",
			metadata,
			local_stack_control.live_demo_seed.PNG_MEDIA_TYPE,
			f"seeded/private-content/{asset_path}",
		],
	)
	local_stack_control.lifecycle_commands.require_command(
		runner.run(
			put_argv,
			local_stack_control.lifecycle_commands.child_environment(target),
			target.repo_root,
			base64.b64encode(
				local_stack_control.live_demo_seed.question_asset_bytes(target.repo_root)
			).decode("ascii"),
		),
		"live-demo Question Asset initialization",
		local_stack_control.disposable_stack_adapter.private_environment_values(target.env_file),
	)


#============================================
def seed_live_demo_baseline(
	target: local_stack_control.models.ComposeTarget,
	runner: local_stack_control.process.CommandRunner,
	values: dict[str, str],
) -> None:
	"""Install the fixed Account and Published Question baseline after buckets exist."""
	if not local_stack_control.live_demo_gateway.is_tls_target(target):
		return
	if any(values.get(setting) != account_id for setting, account_id, _role in LIVE_DEMO_ACCOUNT_SEEDS):
		raise local_stack_control.models.ControllerError("live-demo seeded Account mapping is invalid")
	seed_live_demo_source_objects(target, runner)
	child = local_stack_control.lifecycle_commands.child_environment(target)
	child["PGPASSWORD"] = values["POSTGRES_PASSWORD"]
	argv = local_stack_control.compose.compose_argv(
		target,
		[
			"exec", "-T", "postgres", "psql", "-X", "-v", "ON_ERROR_STOP=1",
			"-U", values["POSTGRES_USER"], "-d", values["POSTGRES_DB"],
		],
	)
	local_stack_control.lifecycle_commands.require_command(
		runner.run(
			argv, child, target.repo_root,
			local_stack_control.live_demo_seed.seed_sql(target.repo_root),
		),
		"live-demo baseline initialization",
		(values["POSTGRES_PASSWORD"],),
	)
