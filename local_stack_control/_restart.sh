# shellcheck shell=bash
# Restart-only helpers. This file is sourced by local_stack_control/launch.sh after
# its shared Compose, environment, and readiness helpers are available.

restart_refusal() {
	die "cannot restart ${RESTART_SERVICE}: $*; run 'source source_me.sh && python3 local_stack.py start --no-open' to reconcile the stack"
}

assert_ready_for_restart() {
	local service_name container_id status exit_code running health_status smtp_count
	local one_shots=(local-data-volume-permissions createbuckets identity-secret-init)
	local required_services=(postgres minio webwork-renderer api worker gateway)

	smtp_count="$(compose_service_container_count_any_state smtp-secret-init)"
	if [ "$WITH_SMTP" -eq 1 ]; then
		[ "$smtp_count" = "1" ] || restart_refusal "the selected SMTP overlay is not initialized"
		one_shots+=(smtp-secret-init)
	elif [ "$smtp_count" != "0" ]; then
		restart_refusal "the running project uses the SMTP overlay; repeat the command with --with-smtp"
	fi
	for service_name in "${one_shots[@]}"; do
		container_id="$(compose_service_container_id_any_state "$service_name")" \
			|| restart_refusal "required one-shot ${service_name} is missing or ambiguous"
		status="$(podman container inspect --format '{{.State.Status}}' "$container_id")"
		exit_code="$(podman container inspect --format '{{.State.ExitCode}}' "$container_id")"
		[ "$status" = "exited" ] && [ "$exit_code" = "0" ] \
			|| restart_refusal "required one-shot ${service_name} has not completed successfully"
	done
	for service_name in "${required_services[@]}"; do
		[ "$service_name" != "$RESTART_SERVICE" ] || continue
		container_id="$(compose_service_container_id_any_state "$service_name")" \
			|| restart_refusal "required service ${service_name} is missing or ambiguous"
		running="$(podman container inspect --format '{{.State.Running}}' "$container_id")"
		[ "$running" = true ] || restart_refusal "required service ${service_name} is not running"
		if [ "$service_name" != worker ]; then
			health_status="$(podman container inspect --format '{{.State.Health.Status}}' "$container_id")"
			[ "$health_status" = healthy ] || restart_refusal "required service ${service_name} is not healthy"
		fi
	done
	if [ "$RESTART_SERVICE" != gateway ]; then
		gateway_port="$(effective_gateway_port)"
		curl --fail --silent --show-error --max-time 2 --output /dev/null \
			"http://127.0.0.1:${gateway_port}/health" 2>/dev/null \
			|| restart_refusal "the published gateway health route is not ready"
	fi
}

container_env_value() {
	local container_id="$1" setting_name="$2"
	podman container inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$container_id" \
		| awk -F= -v setting_name="$setting_name" '$1 == setting_name { print substr($0, index($0, "=") + 1) }'
}

prepare_renderer_identity_for_restart() {
	local renderer_container_id running_renderer_image_id provenance_ref provenance_image_id
	local api_container_id api_renderer_version provenance_line_count

	# shellcheck disable=SC2154
	renderer_image_id="$(podman image inspect --format '{{.Id}}' "$renderer_image_ref")"
	[ -n "$renderer_image_id" ] || restart_refusal "the configured renderer has no OCI image ID"
	renderer_container_id="$(compose_service_container_id_any_state webwork-renderer)" \
		|| restart_refusal "the running renderer is missing or ambiguous"
	running_renderer_image_id="$(podman container inspect --format '{{.Image}}' "$renderer_container_id")"
	[ "$running_renderer_image_id" = "$renderer_image_id" ] \
		|| restart_refusal "the configured and running renderer image identities differ"
	renderer_provenance_path="$REPO_ROOT/$LOCAL_WEBWORK_PROVENANCE_FILE"
	[ ! -L "$renderer_provenance_path" ] && [ -f "$renderer_provenance_path" ] && [ -r "$renderer_provenance_path" ] \
		|| restart_refusal "the renderer provenance record is missing, unreadable, or not regular"
	[ "$(local_file_mode "$renderer_provenance_path")" = 600 ] \
		|| restart_refusal "the renderer provenance record does not have mode 0600"
	provenance_line_count="$(awk 'END { print NR + 0 }' "$renderer_provenance_path")"
	provenance_ref="$(awk -F= '$1 == "image_ref" { print substr($0, index($0, "=") + 1) }' "$renderer_provenance_path")"
	provenance_image_id="$(awk -F= '$1 == "image_id" { print substr($0, index($0, "=") + 1) }' "$renderer_provenance_path")"
	[ "$provenance_line_count" = 2 ] && [ "$provenance_ref" = "$renderer_image_ref" ] \
		&& [ "$provenance_image_id" = "$renderer_image_id" ] \
		|| restart_refusal "the renderer provenance record does not match the selected image"
	PLE_WEBWORK_RENDERER_VERSION="${renderer_image_id#sha256:}"
	export PLE_WEBWORK_RENDERER_VERSION
	if [ "$RESTART_SERVICE" != api ]; then
		api_container_id="$(compose_service_container_id_any_state api)" \
			|| restart_refusal "the API is missing or ambiguous"
		api_renderer_version="$(container_env_value "$api_container_id" PLE_WEBWORK_RENDERER_VERSION)"
		[ "$api_renderer_version" = "$PLE_WEBWORK_RENDERER_VERSION" ] \
			|| restart_refusal "the API and selected renderer identities differ"
	fi
}
