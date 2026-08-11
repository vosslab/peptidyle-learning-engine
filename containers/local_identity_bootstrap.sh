# Local identity bootstrap helpers sourced by launch_local_stack.sh.
#
# Required caller contract: `die`, `local_credential_record`, LOCAL_IDENTITY_FILE,
# LOCAL_CREDENTIAL_FILE, LOCAL_TENANT_ID, LOCAL_INSTRUCTOR_ID, and
# LOCAL_STUDENT_ID are defined before this file is sourced. REPO_ROOT and
# LOCAL_ENV_FILE are also required for default local-stack path normalization.

normalize_default_local_env_file() {
	env_file="$1"
	case "$env_file" in
	"$LOCAL_ENV_FILE"|"$REPO_ROOT/$LOCAL_ENV_FILE") printf '%s\n' "$LOCAL_ENV_FILE" ;;
	*) printf '%s\n' "$env_file" ;;
	esac
}

local_file_mode() {
	file_path="$1"
	if stat -f '%Lp' "$file_path" >/dev/null 2>&1; then
		stat -f '%Lp' "$file_path"
	else
		stat -c '%a' "$file_path"
	fi
}

validate_local_credential() {
	credential="$1"
	[ "${#credential}" -eq 43 ] || die "local login credential must use canonical 32-byte base64url"
	case "$credential" in
		*[!A-Za-z0-9_-]*) die "local login credential must use canonical 32-byte base64url" ;;
	esac
	canonical_credential="$(printf '%s=' "$credential" | tr '_-' '/+' | openssl base64 -d -A 2>/dev/null | openssl base64 -A | tr '+/' '-_' | tr -d '=')" \
		|| die "local login credential must use canonical 32-byte base64url"
	[ "$canonical_credential" = "$credential" ] \
		|| die "local login credential must use canonical 32-byte base64url"
}

local_credential_hash() {
	credential="$1"
	printf '%s=' "$credential" | tr '_-' '/+' | openssl base64 -d -A 2>/dev/null | openssl dgst -sha256 -r | awk '{print $1}'
}

load_local_credentials() {
	[ ! -L "$LOCAL_CREDENTIAL_FILE" ] \
		|| die "$LOCAL_CREDENTIAL_FILE must not be a symbolic link"
	[ -f "$LOCAL_CREDENTIAL_FILE" ] && [ -r "$LOCAL_CREDENTIAL_FILE" ] \
		|| die "$LOCAL_CREDENTIAL_FILE is missing, unreadable, or not regular"
	[ "$(local_file_mode "$LOCAL_CREDENTIAL_FILE")" = "600" ] \
		|| die "$LOCAL_CREDENTIAL_FILE must have mode 0600"

	instructor_credential=""
	student_credential=""
	while IFS='=' read -r credential_role credential_value || [ -n "$credential_role$credential_value" ]; do
		case "$credential_role" in
		instructor)
			[ -z "$instructor_credential" ] || die "$LOCAL_CREDENTIAL_FILE has duplicate instructor credentials"
			instructor_credential="$credential_value"
			;;
		student)
			[ -z "$student_credential" ] || die "$LOCAL_CREDENTIAL_FILE has duplicate student credentials"
			student_credential="$credential_value"
			;;
		*) die "$LOCAL_CREDENTIAL_FILE must contain only instructor and student credentials" ;;
		esac
	done <"$LOCAL_CREDENTIAL_FILE"
	[ -n "$instructor_credential" ] && [ -n "$student_credential" ] \
		|| die "$LOCAL_CREDENTIAL_FILE must contain instructor and student credentials"
	validate_local_credential "$instructor_credential"
	validate_local_credential "$student_credential"
	[ "$instructor_credential" != "$student_credential" ] \
		|| die "$LOCAL_CREDENTIAL_FILE must use distinct instructor and student credentials"
	instructor_hash="$(local_credential_hash "$instructor_credential")" \
		|| die "$LOCAL_CREDENTIAL_FILE could not be decoded"
	student_hash="$(local_credential_hash "$student_credential")" \
		|| die "$LOCAL_CREDENTIAL_FILE could not be decoded"
}

write_local_identity_file() {
	temporary_identity_file="$(mktemp "${LOCAL_IDENTITY_FILE}.XXXXXX")"
	if ! printf '{"credentials":[{"credential_sha256":"%s","learner_alias":"instructor-local","tenant_id":"%s","user_id":"%s","display_name":"Local Instructor","roles":["instructor","administrator"]},{"credential_sha256":"%s","learner_alias":"student-local","tenant_id":"%s","user_id":"%s","display_name":"Local Student","roles":["student"]}]}\n' \
		"$instructor_hash" "$LOCAL_TENANT_ID" "$LOCAL_INSTRUCTOR_ID" \
		"$student_hash" "$LOCAL_TENANT_ID" "$LOCAL_STUDENT_ID" >"$temporary_identity_file"; then
		rm -f "$temporary_identity_file"
		die "could not write $LOCAL_IDENTITY_FILE"
	fi
	chmod 644 "$temporary_identity_file" || {
		rm -f "$temporary_identity_file"
		die "could not set mode 0644 on $LOCAL_IDENTITY_FILE"
	}
	mv "$temporary_identity_file" "$LOCAL_IDENTITY_FILE" || {
		rm -f "$temporary_identity_file"
		die "could not replace $LOCAL_IDENTITY_FILE"
	}
}

bootstrap_local_identities() {
	if [ -e "$LOCAL_CREDENTIAL_FILE" ] || [ -L "$LOCAL_CREDENTIAL_FILE" ]; then
		load_local_credentials
	else
		[ ! -e "$LOCAL_IDENTITY_FILE" ] && [ ! -L "$LOCAL_IDENTITY_FILE" ] \
			|| die "$LOCAL_CREDENTIAL_FILE is missing; refusing to rotate existing local identities"
		instructor_record="$(local_credential_record)"
		student_record="$(local_credential_record)"
		instructor_credential="${instructor_record%%	*}"
		student_credential="${student_record%%	*}"
		umask 077
		printf 'instructor=%s\nstudent=%s\n' \
			"$instructor_credential" "$student_credential" >"$LOCAL_CREDENTIAL_FILE"
		chmod 600 "$LOCAL_CREDENTIAL_FILE"
		load_local_credentials
	fi

	# The container reads a generated hash-only identity projection. Rebuild it
	# atomically from the private credentials on every launch so new required
	# metadata is never blocked by a stale ignored file.
	write_local_identity_file
}
