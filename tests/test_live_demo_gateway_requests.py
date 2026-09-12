"""Safe argv builders for Live Demo course provisioning requests."""

# Standard Library
import pathlib

# local repo modules
import local_stack_control.live_demo_gateway


#============================================
def private_cookie_jar(tmp_path: pathlib.Path) -> tuple[pathlib.Path, str]:
	"""Create one test-owned jar containing a value that must stay out of argv."""
	secret = "session-value-that-must-not-enter-argv"
	jar_path = tmp_path / "mary.cookies"
	jar_path.write_text(f"localhost\tFALSE\t/\tTRUE\t0\tple_session\t{secret}\n", encoding="ascii")
	jar_path.chmod(0o600)
	return jar_path, secret


#============================================
def test_persona_session_argv_writes_the_jar_without_exposing_a_cookie(
	tmp_path: pathlib.Path,
) -> None:
	"""Session creation carries only the closed persona and private jar path."""
	jar_path, secret = private_cookie_jar(tmp_path)

	argv = local_stack_control.live_demo_gateway.persona_session_argv(
		"https://localhost:8443/",
		"maryStudent",
		jar_path,
	)

	assert secret not in " ".join(argv)

#============================================
def test_demo_request_argv_reads_the_jar_without_exposing_a_cookie(tmp_path: pathlib.Path) -> None:
	"""Product requests pass the private jar filename rather than its contents."""
	jar_path, secret = private_cookie_jar(tmp_path)

	argv = local_stack_control.live_demo_gateway.demo_request_argv(
		"https://localhost:8443/",
		"/api/course-instances/C-1/assignments/A-2",
		jar_path,
		"PUT",
		{"title": "Peptide Structure Practice"},
		"3",
	)

	assert secret not in " ".join(argv)
