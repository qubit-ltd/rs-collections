#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RS_CI_TARGET_DIR="${RS_CI_TARGET_DIR:-$PROJECT_ROOT/target/rs-ci-$$}"
exec env \
    RS_CI_PROJECT_ROOT="$PROJECT_ROOT" \
    RS_CI_TARGET_DIR="$RS_CI_TARGET_DIR" \
    "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"
