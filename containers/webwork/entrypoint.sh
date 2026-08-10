#!/usr/bin/env bash
set -euo pipefail

required_secret() {
	secret_file="$1"
	[ -r "$secret_file" ] || { echo "WebWork bootstrap secret file is unreadable" >&2; exit 1; }
	secret_value="$(cat "$secret_file")"
	printf '%s' "$secret_value" | grep -Eq '^[A-Za-z0-9_-]{43,}$' || {
		echo "WebWork bootstrap secret has an invalid format" >&2
		exit 1
	}
	printf '%s' "$secret_value"
}

[ -n "${PLE_WEBWORK_MOJO_SECRET_FILE:-}" ] || { echo "WebWork Mojolicious secret file is required" >&2; exit 1; }
mojo_secret="$(required_secret "$PLE_WEBWORK_MOJO_SECRET_FILE")"
mojo_listen="$(awk -F': ' '$1 == "listen" { print $2 }' /opt/ple-webwork/webwork2.mojolicious.yml)"
mojo_workers="$(awk -F': ' '$1 == "workers" { print $2 }' /opt/ple-webwork/webwork2.mojolicious.yml)"
grep -qx 'allow_unsecured_rpc: 0' /opt/ple-webwork/webwork2.mojolicious.yml
[ "$mojo_listen" = "http://0.0.0.0:8080" ] || { echo "WebWork listen override is invalid" >&2; exit 1; }
[ "$mojo_workers" = "2" ] || { echo "WebWork worker override is invalid" >&2; exit 1; }

# Start from the complete pinned upstream configuration. The PLE fragment is
# intentionally appended only after the distribution configuration is present.
install -m 0640 -o www-data -g www-data /opt/webwork/webwork2/conf/site.conf.dist /opt/webwork/webwork2/conf/site.conf
cat /opt/ple-webwork/site.conf >> /opt/webwork/webwork2/conf/site.conf
install -m 0640 -o www-data -g www-data /opt/webwork/webwork2/conf/webwork2.mojolicious.dist.yml /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
sed -i "0,/^  - /s//  - ${mojo_secret}/" /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
sed -i "s|^    - http://\*:8080$|    - ${mojo_listen}|" /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
sed -i "s/^  workers: 25$/  workers: ${mojo_workers}/" /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
if grep -q '^allow_unsecured_rpc:' /opt/webwork/webwork2/conf/webwork2.mojolicious.yml; then
	sed -i 's/^allow_unsecured_rpc:.*/allow_unsecured_rpc: 0/' /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
else
	printf '\nallow_unsecured_rpc: 0\n' >> /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
fi
grep -q '^secrets:$' /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
grep -q '^pg_dir: /opt/webwork/pg$' /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
grep -q '^allow_unsecured_rpc: 0$' /opt/webwork/webwork2/conf/webwork2.mojolicious.yml
if [ ! -f /opt/webwork/webwork2/conf/localOverrides.conf ]; then
	install -m 0640 -o www-data -g www-data /opt/webwork/webwork2/conf/localOverrides.conf.dist /opt/webwork/webwork2/conf/localOverrides.conf
fi
/usr/local/bin/init_render_course.sh
exec "$@"
