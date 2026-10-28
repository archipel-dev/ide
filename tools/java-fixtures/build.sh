#!/usr/bin/env bash
# Builds a hermetic fixture jar from our own tiny Java sources (no network, no third-party
# binaries). Usage: build.sh <source-dir> <output.jar> [--release N]
# The sources only mimic the *shape* of well-known APIs (an annotation with @Target, a
# test-framework-like Test/Assertions); they are written for this repository.
set -euo pipefail
src="$1"
out="$2"
release="${4:-17}"
# A fixture may pin its target release in `<source-dir>/.release` (a JDK 8 style runtime jar).
if [ -f "$src/.release" ]; then release="$(tr -d '[:space:]' < "$src/.release")"; fi
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/classes"
find "$src" -name '*.java' -print0 | xargs -0 javac -proc:none -nowarn --release "$release" -d "$work/classes"
mkdir -p "$(dirname "$out")"
(cd "$work/classes" && jar --create --file "$work/out.jar" --no-manifest .)
mv "$work/out.jar" "$out"
