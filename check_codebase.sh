#!/usr/bin/env bash
# Run TypeScript type checks, lint, formatting, and Node unit tests (no build).
set -euo pipefail

usage() {
	printf '%s\n' "Usage: check_codebase.sh [-h|--help]" "" \
		"  -h, --help  Print this help and exit 0."
}

while [ "$#" -gt 0 ]; do
	case "$1" in
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "ERROR: unknown flag: $1" >&2
			usage >&2
			exit 2
			;;
	esac
done

cd "$(git rev-parse --show-toplevel)"

if ! command -v node >/dev/null 2>&1; then
	echo "ERROR: node not found on PATH." >&2
	exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
	echo "ERROR: npm not found on PATH." >&2
	exit 1
fi

echo "node $(node --version), npm $(npm --version)"

if [ ! -f package.json ]; then
	echo "ERROR: package.json missing." >&2
	exit 1
fi

if [ ! -d node_modules ]; then
	echo "ERROR: node_modules missing. Run 'npm install' first." >&2
	exit 1
fi

if [ ! -f package-lock.json ]; then
	echo "WARN: package-lock.json missing; npm install will not produce a reproducible install." >&2
fi

echo "==> typecheck"
npx tsc --noEmit -p tsconfig.json

# tsconfig.lint.json covers TypeScript in tests/ and tools/.
echo "==> typecheck:lint"
npx tsc --noEmit -p tsconfig.lint.json
echo "==> lint"
npx eslint --max-warnings 0 '**/*.{ts,tsx,mts,cts,js,mjs,cjs}'
echo "==> format:check"
npx prettier --check '**/*.{ts,tsx,mts,cts,js,mjs,cjs}'
if compgen -G 'tests/test_*.mjs' >/dev/null; then
	echo "==> test:node"
	node --import tsx --test 'tests/test_*.mjs'
else
	echo "==> SKIP test:node (no tests/test_*.mjs files present)"
fi
echo "PASS: codebase checks passed."
