#!/bin/bash
set -e

SCRIPT_PATH="$(dirname ${BASH_SOURCE[0]})"

cd "${SCRIPT_PATH}/../benchmark-runner"

docker compose run --build benchmark-runner

echo ""
echo "Complete!"
echo ""
echo "Results are in ${SCRIPT_PATH}/../benchmark-runner/results.json"
echo ""
