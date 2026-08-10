#!/usr/bin/env python3

# Stable command facade. Implementation modules remain directly importable.
import bump_version.cli
import bump_version.discovery
import bump_version.parsing
import bump_version.rewrite

#============================================

def main() -> None:
	args = bump_version.cli.parse_args()
	base_dir = bump_version.discovery.normalize_base_dir(args.base_dir)
	entries = bump_version.discovery.parse_versions(base_dir, args.max_depth)

	base_version_override = ""
	explicit_version = ""
	if args.bump and args.set_version:
		base_version_override = bump_version.discovery.normalize_base_version_override(
			args.set_version
		)
		if not base_version_override:
			raise SystemExit("--set-version requires a non-empty value.")
	elif args.set_version:
		explicit_version = args.set_version.strip()
		if not explicit_version:
			raise SystemExit("--set-version requires a non-empty value.")

	if explicit_version:
		entries = bump_version.discovery.ensure_version_file_entry(entries, base_dir)

	if not entries:
		raise SystemExit("No version sources found.")

	print("Discovered versions:")
	for entry in entries:
		entry_label = bump_version.discovery.format_entry_label(entry, base_dir)
		version_display = entry["version"] if entry["version"] else "(empty)"
		if entry.get("create"):
			version_display = "(missing)"
		print(f"- {entry_label}: {version_display} ({entry['kind']})")

	base_version_display = ""
	if base_version_override:
		base_version = base_version_override
		base_version_display = base_version
	elif explicit_version:
		versions = sorted(set(entry["version"] for entry in entries))
		if len(versions) == 1:
			base_version = versions[0]
			base_version_display = base_version
		else:
			base_version = ""
			base_version_display = "multiple"
	else:
		base_version = bump_version.discovery.choose_base_version(entries, args.source)
		base_version_display = base_version

	if base_version_override and not args.update_all:
		known_versions = {
			bump_version.discovery.normalize_base_version_override(entry["version"])
			for entry in entries
		}
		if base_version not in known_versions:
			raise SystemExit(
				f"Base version not found: {base_version}. Use --update-all to override."
			)

	if args.enforce_yy_mm and base_version:
		bump_version.parsing.validate_yy_mm_patch(base_version)
	if explicit_version:
		new_version = explicit_version
	else:
		new_version = bump_version.parsing.bump_version(
			base_version,
			args.bump,
			args.pre_style,
		)
	if args.enforce_yy_mm:
		bump_version.parsing.validate_yy_mm_patch(new_version)

	print(f"Base version: {base_version_display}")
	print(f"New version: {new_version}")

	if explicit_version or args.update_all:
		if all(
			bump_version.rewrite.entry_matches_target(entry, new_version)
			for entry in entries
		):
			print("New version matches current version. Nothing to do.")
			return
	else:
		if base_version == new_version:
			print("New version matches current version. Nothing to do.")
			return

	if explicit_version or args.update_all:
		# Entries already at the target need no rewrite and stay out of the plan.
		selected = [
			entry
			for entry in entries
			if not bump_version.rewrite.entry_matches_target(entry, new_version)
		]
		skipped = []
	else:
		if base_version_override:
			selected = [
				entry
				for entry in entries
				if bump_version.discovery.normalize_base_version_override(
					entry["version"]
				) == base_version
			]
			skipped = [
				entry
				for entry in entries
				if bump_version.discovery.normalize_base_version_override(
					entry["version"]
				) != base_version
			]
		else:
			selected = [entry for entry in entries if entry["version"] == base_version]
			skipped = [entry for entry in entries if entry["version"] != base_version]

	if skipped:
		print("Skipping entries with different versions:")
		for entry in skipped:
			entry_label = bump_version.discovery.format_entry_label(entry, base_dir)
			print(f"- {entry_label}: {entry['version']}")

	print("Planned updates:")
	for entry in selected:
		entry_label = bump_version.discovery.format_entry_label(entry, base_dir)
		print(f"- {entry_label}")

	# Count distinct files: one Cargo.lock holds many package stanzas.
	changed_paths = set()
	for entry in selected:
		result = bump_version.rewrite.update_entry(entry, new_version, args.apply)
		if result["changed"]:
			changed_paths.add(result["path"])

	if args.apply:
		print(f"Updated {len(changed_paths)} file(s).")
	else:
		print("Dry run only. Use --apply to write changes.")


#============================================
if __name__ == "__main__":
	main()
