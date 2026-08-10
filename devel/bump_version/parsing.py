# Standard Library
import re

# local repo modules
import bump_version.contracts as contracts
import bump_version.formatting as formatting

def is_version_candidate(text: str) -> bool:
	"""Check whether a string looks like a version.

	Args:
		text (str): Version candidate.

	Returns:
		bool: True if it looks like a version.
	"""
	value = text.strip()
	if not value:
		return False
	try:
		parse_version_details(value)
		return True
	except ValueError:
		pass
	if contracts.SIMPLE_VERSION_PATTERN.fullmatch(value):
		return True
	return False

#============================================

#============================================

def bump_version(version: str, bump: str, pre_style: str) -> str:
	"""Bump a semantic version.

	Args:
		version (str): Current version string.
		bump (str): major, minor, patch, alpha, beta, or rc.
		pre_style (str): pep440 or dash.

	Returns:
		str: New version string.
	"""
	details = parse_version_details(version)
	if bump in ("major", "minor", "patch"):
		if details["pre_tag"] or details["pre_num"] is not None:
			raise ValueError(f"Remove prerelease suffix before bumping: {version}")
		if bump == "major":
			details["major"] += 1
			details["minor"] = 0
			details["patch"] = 0
		elif bump == "minor":
			details["minor"] += 1
			details["patch"] = 0
		else:
			details["patch"] += 1
		details["pre_tag"] = None
		details["pre_num"] = None
		return formatting.format_version(details)

	if bump in ("alpha", "beta", "rc"):
		return bump_prerelease(details, bump, pre_style)

	raise ValueError(f"Unsupported bump mode: {bump}")

#============================================

def version_number_parts(
	major_text: str,
	minor_text: str,
	patch_text: str | None=None,
) -> dict:
	"""Convert version segment text into numeric values plus original widths.

	The width fields are the zero-padding filter: they record that "08" was
	two characters so 26.08 can be rebuilt as 26.08 rather than 26.8, while
	the numeric fields drive arithmetic and the unpadded Cargo form. A version
	with no patch segment reports patch 0 at width 1.

	Args:
		major_text (str): Major segment as written.
		minor_text (str): Minor segment as written.
		patch_text (str | None): Patch segment as written, or None when absent.

	Returns:
		dict: Numeric major/minor/patch and their source widths.
	"""
	parts = {
		"major": int(major_text),
		"minor": int(minor_text),
		"patch": int(patch_text) if patch_text is not None else 0,
		"major_width": len(major_text),
		"minor_width": len(minor_text),
		"patch_width": len(patch_text) if patch_text is not None else 1,
	}
	return parts

#============================================

def parse_version_details(version: str) -> dict:
	"""Parse a version string into parts.

	Args:
		version (str): Version string.

	Returns:
		dict: Parsed version parts.
	"""
	match = contracts.PEP440_PATTERN.match(version)
	if match:
		details = version_number_parts(
			match.group("major"),
			match.group("minor"),
			match.group("patch"),
		)
		details.update({
			"pre_tag": contracts.PRE_TAG_NAMES[match.group("tag")],
			"pre_num": int(match.group("num")),
			"style": "pep440",
		})
		return details

	match = contracts.SHORT_PEP440_PATTERN.match(version)
	if match:
		details = version_number_parts(match.group("major"), match.group("minor"))
		details.update({
			"pre_tag": contracts.PRE_TAG_NAMES[match.group("tag")],
			"pre_num": int(match.group("num")),
			"style": "pep440",
			"patch_optional": True,
		})
		return details

	match = contracts.DASH_PATTERN.match(version)
	if match:
		num_text = match.group("num")
		details = version_number_parts(
			match.group("major"),
			match.group("minor"),
			match.group("patch"),
		)
		details.update({
			"pre_tag": match.group("tag"),
			"pre_num": int(num_text) if num_text else 0,
			"style": "dash",
		})
		return details

	match = contracts.YY_MM_PATCH_PATTERN.match(version)
	if match:
		num_text = match.group("num")
		details = version_number_parts(
			match.group("major"),
			match.group("minor"),
			match.group("patch"),
		)
		details.update({
			"pre_tag": match.group("tag"),
			"pre_num": int(num_text) if num_text else None,
			"style": "pep440",
			"patch_optional": False,
		})
		return details

	match = contracts.YY_MM_SHORT_PATTERN.match(version)
	if match:
		details = version_number_parts(match.group("major"), match.group("minor"))
		details.update({
			"pre_tag": match.group("tag"),
			"pre_num": int(match.group("num")),
			"style": "pep440",
			"patch_optional": True,
		})
		return details

	match = contracts.YY_MM_BARE_PATTERN.match(version)
	if match:
		details = version_number_parts(match.group("major"), match.group("minor"))
		details.update({
			"pre_tag": None,
			"pre_num": None,
			"style": "pep440",
			"patch_optional": True,
		})
		return details

	match = contracts.BASE_VERSION_PATTERN.match(version)
	if match:
		details = version_number_parts(
			match.group("major"),
			match.group("minor"),
			match.group("patch"),
		)
		details.update({
			"pre_tag": None,
			"pre_num": None,
			"style": "none",
			"patch_optional": False,
		})
		return details

	raise ValueError(f"Unsupported version format: {version}")

#============================================

def validate_yy_mm_patch(version: str) -> None:
	"""Validate YY.MM.PATCH format with optional PEP 440 prerelease suffix.

	Args:
		version (str): Version string.
	"""
	match = contracts.YY_MM_PATCH_PATTERN.match(version)
	match_short = contracts.YY_MM_SHORT_PATTERN.match(version)
	match_bare = contracts.YY_MM_BARE_PATTERN.match(version)
	if not match and not match_short and not match_bare:
		raise ValueError(
			f"Version must be YY.MM, YY.MM.PATCH, or YY.MM prerelease: {version}"
		)

	month_text = (match or match_short or match_bare).group("minor")
	month = int(month_text)
	if month < 1 or month > 12:
		raise ValueError(f"Invalid month in version: {version}")

#============================================

def bump_prerelease(details: dict, tag: str, pre_style: str) -> str:
	"""Bump or add a prerelease suffix.

	Args:
		details (dict): Parsed version details.
		tag (str): alpha, beta, or rc.
		pre_style (str): pep440 or dash.

	Returns:
		str: Updated version.
	"""
	style = details["style"]
	if style == "none":
		style = pre_style
	details = dict(details)
	details["style"] = style
	if details["pre_tag"] == tag:
		if details["pre_num"] is None:
			details["pre_num"] = 1
		else:
			details["pre_num"] += 1
	else:
		details["pre_tag"] = tag
		details["pre_num"] = 1
	return formatting.format_version(details)

#============================================
