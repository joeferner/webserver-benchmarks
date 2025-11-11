#!/bin/bash
set -e

SCRIPT_PATH="$(dirname ${BASH_SOURCE[0]})"
SELF="$(realpath "$0")"

GREEN="\033[1;32m"
RED="\033[1;31m"
RESET="\033[0m"

find "${SCRIPT_PATH}/.." -type f -name "pre-commit.sh" | while read -r script; do
    SCRIPT_PATH="$(realpath "$script")"
    # Skip itself
    if [[ "$SCRIPT_PATH" != "$SELF" ]]; then
               echo -e "${GREEN}\nRunning $SCRIPT_PATH...\n------------------------------------------------------------------------------${RESET}"
        bash "$SCRIPT_PATH" || { echo -e "${RED}Script $SCRIPT_PATH failed! Aborting.${RESET}"; exit 1; }
    fi
done

echo ""
echo "Complete!"
