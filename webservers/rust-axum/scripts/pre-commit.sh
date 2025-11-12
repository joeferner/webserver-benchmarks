#!/bin/bash
set -e

SCRIPT_PATH="$(dirname ${BASH_SOURCE[0]})"

cd "${SCRIPT_PATH}/.."

docker build --tag benchmark-webservers-rust-axum .
docker run --rm -v "${SCRIPT_PATH}/../src":"/app/src" benchmark-webservers-rust-axum bash -c "cargo fmt && cargo clippy"
