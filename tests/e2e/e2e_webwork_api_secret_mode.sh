#!/usr/bin/env bash
# Prove the API's strict-0600 secret handoff without building WebWork.
set -euo pipefail

proof_root="$(mktemp -d "${TMPDIR:-/tmp}/ple-webwork-api-secret.XXXXXX")"
proof_volume="ple-webwork-api-secret-proof-$$"
alpine_digest="$(awk -F= '$1 == "PLE_SECRET_INIT_IMAGE_SHA256" { print $2 }' containers/env.example)"
case "$alpine_digest" in ????????*) ;; *) echo "missing pinned Alpine initializer digest" >&2; exit 1 ;; esac
alpine_image="docker.io/library/alpine@sha256:${alpine_digest}"
trap 'podman volume rm -f "$proof_volume" >/dev/null 2>&1 || true; rm -rf "$proof_root"' EXIT
umask 077
openssl rand -base64 48 | tr '+/' '-_' | tr -d '\n=' >"$proof_root/source"
chmod 600 "$proof_root/source"

podman volume create "$proof_volume" >/dev/null
podman run --rm --network none --read-only --cap-drop ALL --cap-add CHOWN --cap-add DAC_OVERRIDE --security-opt no-new-privileges \
	-v "$proof_root/source:/run/source/webwork_render_password:ro" \
	-v "$proof_volume:/run/ple-secrets" \
	"$alpine_image" /bin/sh -ec \
	'cp /run/source/webwork_render_password /run/ple-secrets/webwork_render_password; chmod 600 /run/ple-secrets/webwork_render_password; chown 10001:10001 /run/ple-secrets/webwork_render_password'
podman run --rm --user 10001:10001 -v "$proof_volume:/run/ple-secrets:ro" "$alpine_image" /bin/sh -ec \
	'[ "$(stat -c %a /run/ple-secrets/webwork_render_password)" = 600 ]; cat /run/ple-secrets/webwork_render_password >/dev/null'
openssl rand -base64 48 | tr '+/' '-_' | tr -d '\n=' >"$proof_root/source"
chmod 600 "$proof_root/source"
expected_hash="$(sha256sum "$proof_root/source" | awk '{print $1}')"
podman run --rm --network none --read-only --cap-drop ALL --cap-add CHOWN --cap-add DAC_OVERRIDE --security-opt no-new-privileges \
	-v "$proof_root/source:/run/source/webwork_render_password:ro" -v "$proof_volume:/run/ple-secrets" "$alpine_image" /bin/sh -ec \
	'cp /run/source/webwork_render_password /run/ple-secrets/webwork_render_password; chmod 600 /run/ple-secrets/webwork_render_password; chown 10001:10001 /run/ple-secrets/webwork_render_password'
actual_hash="$(podman run --rm --user 10001:10001 -v "$proof_volume:/run/ple-secrets:ro" "$alpine_image" sha256sum /run/ple-secrets/webwork_render_password | awk '{print $1}')"
[ "$actual_hash" = "$expected_hash" ] || { echo "FAIL: rotated host secret did not refresh API runtime volume" >&2; exit 1; }
if podman run --rm --user 10002:10002 -v "$proof_volume:/run/ple-secrets:ro" "$alpine_image" cat /run/ple-secrets/webwork_render_password >/dev/null 2>&1; then
	echo "FAIL: unrelated UID read the API secret" >&2
	exit 1
fi
echo "PASS: strict API secret is readable only by UID 10001."
