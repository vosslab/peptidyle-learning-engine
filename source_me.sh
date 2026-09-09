set | grep -q '^BASH_VERSION=' || echo "use bash for your shell"
set | grep -q '^BASH_VERSION=' || exit 1

# Set Python environment optimizations
export PYTHONUNBUFFERED=1
export PYTHONDONTWRITEBYTECODE=1

source ~/.bashrc

# Make the repository's root Python packages available to subdirectory launchers.
ple_repository_root="$(git rev-parse --show-toplevel)"
export PYTHONPATH="$ple_repository_root${PYTHONPATH:+:$PYTHONPATH}"
unset ple_repository_root
