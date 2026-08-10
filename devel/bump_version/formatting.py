# local repo modules
import bump_version.contracts

def format_version(details: dict) -> str:
	"""Format a version string from parts.

	Args:
		details (dict): Version parts.

	Returns:
		str: Formatted version.
	"""
	major = format_number(details["major"], details.get("major_width"))
	minor = format_number(details["minor"], details.get("minor_width"))
	patch = format_number(details["patch"], details.get("patch_width"))
	base = f"{major}.{minor}.{patch}"
	if not details["pre_tag"]:
		return base

	pre_num = details["pre_num"] if details["pre_num"] is not None else 1
	if details["style"] == "pep440":
		return f"{base}{bump_version.contracts.PRE_TAG_SHORT[details['pre_tag']]}{pre_num}"

	return f"{base}-{details['pre_tag']}.{pre_num}"

#============================================

def format_number(value: int, width: int | None) -> str:
	"""Format a number with optional zero padding.

	Args:
		value (int): Numeric value.
		width (int | None): Minimum width to preserve.

	Returns:
		str: Formatted number.
	"""
	text = str(value)
	if width and len(text) < width:
		return text.zfill(width)
	return text

#============================================
