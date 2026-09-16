#!/usr/bin/env python3
"""Complete ordinary local-demo Morgan MFA; return a session only through a pipe."""

import argparse
import http.cookies
import json
import os
import pathlib
import ssl
import stat
import subprocess
import sys
import time
import urllib.request
import uuid


#============================================
def post_local(
	port: int, path: str, payload: dict, context: ssl.SSLContext, cookie: str = "",
) -> tuple[dict, str]:
	"""Use only the fixed local HTTPS origin; never redirect credential requests."""
	origin = f"https://localhost:{port}"
	headers = {"Origin": origin, "Content-Type": "application/json"}
	if cookie:
		headers["Cookie"] = cookie
	request = urllib.request.Request(origin + path, json.dumps(payload).encode(), headers)
	opener = urllib.request.build_opener(
		urllib.request.ProxyHandler({}), urllib.request.HTTPSHandler(context=context), NoRedirect(),
	)
	with opener.open(request, timeout=12) as response:
		if response.status != 200:
			raise ValueError("local authentication status invalid")
		body = response.read(4097)
		if len(body) > 4096:
			raise ValueError("local authentication response too large")
		cookies = http.cookies.SimpleCookie()
		for value in response.headers.get_all("Set-Cookie", []):
			cookies.load(value)
		cookie_name = "__Host-ple_pending_mfa" if path.endswith("accounts") else "__Host-ple_session"
		if cookie_name not in cookies or not cookies[cookie_name].value:
			raise ValueError("local authentication cookie unavailable")
		return json.loads(body), f"{cookie_name}={cookies[cookie_name].value}"


class NoRedirect(urllib.request.HTTPRedirectHandler):
	"""ASVS 14.2.4: a server redirect cannot forward local credentials elsewhere."""

	def redirect_request(
		self, request: urllib.request.Request, response: object, code: int,
		message: str, headers: object, url: str,
	) -> None:
		return None


#============================================
def authenticate_morgan(port: int, setup_file: pathlib.Path, ca_file: pathlib.Path) -> str:
	"""Complete the existing pending-attestation flow once, without retry or bypass."""
	# ASVS 12.3.2, 12.3.4: trust only the selected gateway's local CA and verify localhost.
	# Construct directly so SSLKEYLOGFILE cannot export authentication transport keys.
	context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
	context.load_verify_locations(cafile=str(ca_file))
	pending, binding = post_local(
		port, "/api/auth/live-demo/accounts", {"persona": "morganSysadmin"}, context,
	)
	if set(pending) != {"pendingMfa", "attestationId"} or pending["pendingMfa"] is not True:
		raise ValueError("pending local MFA response invalid")
	attestation = str(uuid.UUID(pending["attestationId"]))
	if attestation != pending["attestationId"]:
		raise ValueError("pending local MFA identity invalid")
	# ASVS 6.5.1, 6.5.5: use a fresh counter, never replay a rejected code.
	# ASVS 13.3.2, 14.2.4: the existing child alone reads the seed; private pipe only.
	after_counter = int(time.time() // 30)
	child = subprocess.run(
		[sys.executable, "-m", "devel.local_demo_totp", str(setup_file),
			"--after-counter", str(after_counter)],
		stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=40, check=True,
	)
	if len(child.stdout) > 128:
		raise ValueError("local authenticator response too large")
	result = json.loads(child.stdout)
	if (set(result) != {"code", "counter"} or not isinstance(result["code"], str)
		or type(result["counter"]) is not int or result["counter"] <= after_counter):
		raise ValueError("local authenticator response invalid")
	code = result["code"]
	if len(code) != 6 or not code.isascii() or not code.isdigit():
		raise ValueError("local authenticator code invalid")
	receipt, session = post_local(
		port, f"/api/auth/sysadmin-totp/complete/{attestation}", {"code": code}, context, binding,
	)
	if receipt != {"authenticated": True}:
		raise ValueError("local MFA completion invalid")
	return session


#============================================
def main() -> None:
	"""Reject terminal/file credentials and expose only sanitized failure diagnostics."""
	parser = argparse.ArgumentParser(description=__doc__)
	parser.add_argument("port", type=int)
	parser.add_argument("setup_file", type=pathlib.Path)
	parser.add_argument("--ca-file", type=pathlib.Path, required=True)
	args = parser.parse_args()
	# ASVS 14.2.4: sessions, like authenticator codes, never reach terminal or files.
	mode = os.fstat(sys.stdout.fileno()).st_mode
	if not (stat.S_ISFIFO(mode) or stat.S_ISSOCK(mode)) or not 1 <= args.port <= 65535:
		raise ValueError("private pipe and local gateway port required")
	sys.stdout.write(authenticate_morgan(args.port, args.setup_file, args.ca_file))


if __name__ == "__main__":
	try:
		main()
	except (OSError, ValueError, KeyError, TypeError, http.cookies.CookieError, subprocess.SubprocessError):
		# ASVS 16.5.1, 16.5.3: fail closed, without response/cookie/seed diagnostics.
		sys.stderr.write("Ordinary local-demo Morgan MFA failed; no session issued.\n")
		sys.exit(1)
