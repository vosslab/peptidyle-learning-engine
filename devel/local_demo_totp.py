#!/usr/bin/env python3
"""Separate local-demo authenticator; emit one TOTP only to a private process pipe."""

import argparse
import base64
import hashlib
import hmac
import json
import os
import pathlib
import re
import stat
import sys
import time
import urllib.parse

import local_stack_control.models
import local_stack_control.private_files


#============================================
def read_setup_seed(path: pathlib.Path) -> bytes:
	"""Require the controller's private, bounded, exact Morgan setup URI shape."""
	# ASVS 13.3.2, 2.2.1: reuse descriptor-validated current-user mode-0600 reads.
	if not path.is_absolute():
		raise ValueError("absolute local setup path required")
	content = local_stack_control.private_files.read_current_user_private_file(path, 2_048)
	uri = urllib.parse.urlsplit(content.decode("ascii").strip())
	issuer = "Peptidyle Learning Engine local demo"
	parameters = urllib.parse.parse_qs(uri.query, strict_parsing=True)
	if (
		uri.scheme != "otpauth" or uri.netloc != "totp" or uri.fragment
		or urllib.parse.unquote(uri.path) != f"/{issuer}:Morgan Delgado"
		or set(parameters) != {"secret", "issuer", "algorithm", "digits", "period"}
		or any(len(values) != 1 for values in parameters.values())
		or parameters["issuer"] != [issuer] or parameters["algorithm"] != ["SHA1"]
		or parameters["digits"] != ["6"] or parameters["period"] != ["30"]
	):
		raise ValueError("local setup URI is invalid")
	encoded = parameters["secret"][0]
	if re.fullmatch(r"[A-Z2-7]{32,103}", encoded) is None:
		raise ValueError("local setup seed is invalid")
	seed = base64.b32decode(encoded + "=" * (-len(encoded) % 8))
	if not 20 <= len(seed) <= 64 or base64.b32encode(seed).decode("ascii").rstrip("=") != encoded:
		raise ValueError("local setup seed is invalid")
	return seed


#============================================
def code_for_counter(seed: bytes, counter: int) -> str:
	"""RFC 6238 SHA1 dynamic truncation, six digits, 30-second moving counter."""
	# ASVS 11.2.1: stdlib HMAC primitive; SHA1 is the existing TOTP protocol choice.
	digest = hmac.new(seed, counter.to_bytes(8, "big"), hashlib.sha1).digest()
	offset = digest[-1] & 15
	number = int.from_bytes(digest[offset:offset + 4], "big") & 0x7fffffff
	code = f"{number % 1_000_000:06d}"
	return code


#============================================
def main() -> None:
	"""Never print a seed or code to a terminal, regular file, or diagnostics."""
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("setup_file", type=pathlib.Path)
	parser.add_argument("--after-counter", type=int, default=-1)
	args = parser.parse_args()
	# ASVS 14.2.4: stdout must be the parent's private pipe (Node uses a socketpair).
	output_mode = os.fstat(sys.stdout.fileno()).st_mode
	if not (stat.S_ISFIFO(output_mode) or stat.S_ISSOCK(output_mode)):
		raise ValueError("authenticator requires a private process pipe")
	seed = read_setup_seed(args.setup_file)
	if not -1 <= args.after_counter <= int(time.time() // 30):
		raise ValueError("previous local counter is invalid")
	# ASVS 6.5.1, 6.5.5: wait, do not replay or retry a rejected authentication.
	while True:
		now = time.time()
		counter = int(now // 30)
		if counter > args.after_counter and (counter + 1) * 30 - now >= 5:
			break
		time.sleep(min(1, (counter + 1) * 30 - now + 0.05))
	result = {"code": code_for_counter(seed, counter), "counter": counter}
	sys.stdout.write(json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
	try:
		main()
	except (OSError, ValueError, local_stack_control.models.ControllerError):
		# Do not expose parser input, private content, or exception/traceback values.
		sys.stderr.write("Local demonstration authenticator failed.\n")
		sys.exit(1)
