#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(CDPATH="" cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

AGENT_TYPE="${1:-codex}"
BEGIN_MARKER="<!-- BEGIN AUTO-GENERATED CODEX CONTEXT -->"
END_MARKER="<!-- END AUTO-GENERATED CODEX CONTEXT -->"

log_info() {
    echo "INFO: $1"
}

log_success() {
    echo "SUCCESS: $1"
}

log_error() {
    echo "ERROR: $1" >&2
}

usage() {
    cat <<'EOF'
Usage: ./update-agent-context.sh [codex]

Refresh the managed Codex runtime context block in AGENTS.md using
the current feature's plan.md data.
EOF
}

extract_plan_field() {
    local field_pattern="$1"
    local plan_file="$2"

    grep "^\*\*${field_pattern}\*\*: " "$plan_file" 2>/dev/null | \
        head -1 | \
        sed "s|^\*\*${field_pattern}\*\*: ||" | \
        sed 's/^[ \t]*//;s/[ \t]*$//'
}

normalize_field() {
    local value="${1:-}"
    if [[ -z "$value" ]]; then
        echo "not specified"
    else
        echo "$value"
    fi
}

render_block() {
    local current_date="$1"
    local current_branch="$2"
    local language="$3"
    local dependencies="$4"
    local storage="$5"
    local testing="$6"
    local project_type="$7"

    cat <<EOF
$BEGIN_MARKER
## Codex Runtime Context

This section is maintained by \`.specify/scripts/bash/update-agent-context.sh codex\` during planning work.

- Last updated: $current_date
- Active feature: $current_branch
- Language/Version: $language
- Primary Dependencies: $dependencies
- Storage: $storage
- Testing: $testing
- Project Type: $project_type
$END_MARKER
EOF
}

main() {
    if [[ "$AGENT_TYPE" == "--help" || "$AGENT_TYPE" == "-h" ]]; then
        usage
        exit 0
    fi

    if [[ "$AGENT_TYPE" != "codex" ]]; then
        log_error "Unsupported agent type: $AGENT_TYPE"
        log_error "This repository only maintains Codex agent context."
        exit 1
    fi

    local paths_output
    paths_output=$(get_feature_paths) || {
        log_error "Failed to resolve feature paths"
        exit 1
    }
    eval "$paths_output"
    unset paths_output

    check_feature_branch "$CURRENT_BRANCH" "$HAS_GIT" || exit 1

    if [[ ! -f "$IMPL_PLAN" ]]; then
        log_error "No plan.md found at $IMPL_PLAN"
        exit 1
    fi

    if [[ ! -f "$REPO_ROOT/AGENTS.md" ]]; then
        log_error "AGENTS.md not found at $REPO_ROOT/AGENTS.md"
        exit 1
    fi

    local current_date
    current_date=$(date +%Y-%m-%d)

    local language
    local dependencies
    local storage
    local testing
    local project_type
    language=$(normalize_field "$(extract_plan_field "Language/Version" "$IMPL_PLAN")")
    dependencies=$(normalize_field "$(extract_plan_field "Primary Dependencies" "$IMPL_PLAN")")
    storage=$(normalize_field "$(extract_plan_field "Storage" "$IMPL_PLAN")")
    testing=$(normalize_field "$(extract_plan_field "Testing" "$IMPL_PLAN")")
    project_type=$(normalize_field "$(extract_plan_field "Project Type" "$IMPL_PLAN")")

    local block
    block=$(render_block "$current_date" "$CURRENT_BRANCH" "$language" "$dependencies" "$storage" "$testing" "$project_type")

    local temp_file
    temp_file=$(mktemp)

    awk -v block="$block" -v begin="$BEGIN_MARKER" -v end="$END_MARKER" '
        BEGIN {
            in_block = 0
            replaced = 0
        }
        $0 == begin {
            if (!replaced) {
                print block
                replaced = 1
            }
            in_block = 1
            next
        }
        $0 == end {
            in_block = 0
            next
        }
        !in_block {
            print
        }
        END {
            if (!replaced) {
                print ""
                print block
            }
        }
    ' "$REPO_ROOT/AGENTS.md" > "$temp_file"

    mv "$temp_file" "$REPO_ROOT/AGENTS.md"
    log_success "Updated Codex runtime context in $REPO_ROOT/AGENTS.md"
}

main "$@"
