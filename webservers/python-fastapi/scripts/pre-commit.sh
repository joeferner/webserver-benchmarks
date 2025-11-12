#!/bin/bash
set -e

SCRIPT_PATH="$(dirname ${BASH_SOURCE[0]})"

cd "${SCRIPT_PATH}/.."

docker build --tag benchmark-webservers-python-fastapi .
docker run --rm -v "${SCRIPT_PATH}/../src":"/app/src" benchmark-webservers-python-fastapi uv run ruff format
