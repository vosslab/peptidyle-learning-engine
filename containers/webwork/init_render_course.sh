#!/usr/bin/env bash
set -euo pipefail
required_setting() {
	setting_name="$1"
	[ -n "${!setting_name:-}" ] || { echo "WebWork bootstrap requires ${setting_name}" >&2; exit 1; }
}
for setting_name in WEBWORK_DB_HOST WEBWORK_DB_PORT WEBWORK_DB_NAME WEBWORK_DB_USER WEBWORK_DB_PASSWORD PLE_WEBWORK_RENDER_COURSE_ID PLE_WEBWORK_RENDER_USER PLE_WEBWORK_RENDER_PASSWORD_FILE; do
	required_setting "$setting_name"
done
[ -r "$PLE_WEBWORK_RENDER_PASSWORD_FILE" ] || { echo "WebWork bootstrap password file is unreadable" >&2; exit 1; }
for upstream_identifier in "$PLE_WEBWORK_RENDER_COURSE_ID" "$PLE_WEBWORK_RENDER_USER"; do
	case "$upstream_identifier" in
		*[!A-Za-z0-9_-]*|'') echo "WebWork course and service-user IDs must use ASCII letters, digits, underscores, or hyphens" >&2; exit 1 ;;
	esac
done
until mariadb-admin ping --silent --host="$WEBWORK_DB_HOST" --port="$WEBWORK_DB_PORT" --user="$WEBWORK_DB_USER" --password="$WEBWORK_DB_PASSWORD"; do
	sleep 1
done
course_dir="/opt/webwork/courses/${PLE_WEBWORK_RENDER_COURSE_ID}"
course_table_count="$(mariadb --silent --skip-column-names --host="$WEBWORK_DB_HOST" --port="$WEBWORK_DB_PORT" --user="$WEBWORK_DB_USER" --password="$WEBWORK_DB_PASSWORD" "$WEBWORK_DB_NAME" -e "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name LIKE '${PLE_WEBWORK_RENDER_COURSE_ID}%';")"
case "$course_table_count" in
	''|*[!0-9]*) echo "could not inspect WebWork course tables" >&2; exit 1 ;;
esac
if [ -d "$course_dir" ] && [ "$course_table_count" -eq 0 ]; then
	# The database was reset while the separate courses volume survived. Preserve
	# the unusable tree for diagnosis, then let upstream addcourse rebuild it.
	orphan_dir="$(mktemp -d "/opt/webwork/courses/.orphaned-${PLE_WEBWORK_RENDER_COURSE_ID}-XXXXXX")"
	mv "$course_dir" "$orphan_dir/course"
fi
if [ ! -d "$course_dir" ]; then
	classlist_file="$(mktemp)"
	trap 'rm -f "$classlist_file"' EXIT
	password_hash="$(openssl passwd -6 "$(cat "$PLE_WEBWORK_RENDER_PASSWORD_FILE")")"
	printf 'render,PLE,Renderer,C,,,,,%s,%s,2\n' "$PLE_WEBWORK_RENDER_USER" "$password_hash" >"$classlist_file"
	umask 027
	/opt/webwork/webwork2/bin/addcourse --users="$classlist_file" "$PLE_WEBWORK_RENDER_COURSE_ID"
fi
rm -f "$course_dir/course.conf"
install -m 0640 /opt/ple-webwork/course.conf "$course_dir/course.conf"
/usr/local/bin/reconcile_render_account.pl "$PLE_WEBWORK_RENDER_COURSE_ID" "$PLE_WEBWORK_RENDER_USER" "$PLE_WEBWORK_RENDER_PASSWORD_FILE"
chown -R www-data:www-data "$course_dir"
