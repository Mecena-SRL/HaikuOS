#!/bin/sh
# Compila pacchetti con hdev (host). Uso: sh tools/build-pkgs.sh <arch> <dir>...
# Ogni <dir> deve contenere haiku.toml. Le dir sono relative alla cartella corrente.
set -e
exec cargo run -q --manifest-path "$(dirname "$0")/../devtools/Cargo.toml" -- build "$@"
