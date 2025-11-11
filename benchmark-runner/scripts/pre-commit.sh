#!/bin/bash
set -e

SCRIPT_PATH="$(dirname ${BASH_SOURCE[0]})"

cd "${SCRIPT_PATH}/.."

docker build --tag benchmark-runner .
docker run --rm \
    -v "./src":"/app/src" \
    -v "./target":"/app/target" \
    benchmark-runner bash -c "cargo fmt && cargo clippy"