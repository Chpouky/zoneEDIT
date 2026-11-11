#!/usr/bin/env bash
# Vérification complète du projet : formatage, lints, tests.
# Usage : ./check.sh            (tests hermétiques, fixtures uniquement)
#         ZONEEDIT_GAMEDATA=..  ./check.sh   (ajoute les tests sur les vraies données)
set -euo pipefail
cd "$(dirname "$0")"

echo "== cargo fmt --check"
cargo fmt --all -- --check

echo "== cargo clippy"
cargo clippy --all-targets -- -D warnings

echo "== cargo test"
cargo test --all-targets

if [[ -n "${ZONEEDIT_GAMEDATA:-}" ]]; then
  echo "== tests sur données réelles ($ZONEEDIT_GAMEDATA)"
  cargo test --test gamedata_test -- --ignored
fi

echo "OK"
