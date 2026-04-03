#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_message() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_help() {
    print_message "${BLUE}" "=== Agent Foundry Manager ==="
    echo ""
    print_message "${YELLOW}" "Usage:"
    echo "  ./agent_foundry_manager.sh <COMMAND> <OBJECT>"
    echo ""
    print_message "${YELLOW}" "Commands:"
    echo "  restart <agent>          - Restart agent container (no rebuild)"
    echo "  redeploy <agent>         - Rebuild agent only (base untouched) + start"
    echo "  rebuild base             - Rebuild base image only (agents untouched)"
    echo "  full-redeploy <agent>    - Rebuild base + agent + start"
    echo ""
    print_message "${YELLOW}" "Examples:"
    echo "  ./agent_foundry_manager.sh restart agent_davon"
    echo "  ./agent_foundry_manager.sh redeploy agent_davon"
    echo "  ./agent_foundry_manager.sh rebuild base"
    echo "  ./agent_foundry_manager.sh full-redeploy agent_davon"
    echo ""
    print_message "${YELLOW}" "Available agents:"
    AGENTS=$(docker compose config --services | grep "^agent_" | grep -v "_base")
    for agent in $AGENTS; do
        echo "  - $agent"
    done
}

# ============================================================================
# Command: restart <agent>
# ============================================================================
restart_agent() {
    local agent=$1
    print_message "${GREEN}" "Restarting agent: $agent..."
    docker compose restart $agent
    print_message "${GREEN}" "Agent restarted successfully!"
}

# ============================================================================
# Command: redeploy <agent>
# ============================================================================
redeploy_agent() {
    local agent=$1
    print_message "${GREEN}" "Redeploying agent: $agent (base image untouched)..."

    print_message "${YELLOW}" "1. Stopping and removing $agent container..."
    docker compose stop $agent
    docker compose rm -f $agent

    IMAGE_ID=$(docker images -q blacksmith_hub-${agent})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "2. Removing old $agent image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    print_message "${YELLOW}" "3. Building $agent..."
    docker compose build $agent

    print_message "${YELLOW}" "4. Starting $agent..."
    docker compose up -d $agent

    print_message "${GREEN}" "Agent redeployed successfully!"
}

# ============================================================================
# Command: rebuild base
# ============================================================================
rebuild_base() {
    print_message "${GREEN}" "Rebuilding agent_foundry_base image only..."

    # OPTIMIZATION: Do NOT remove the base image - this preserves BuildKit cache layers!
    print_message "${YELLOW}" "Note: Keeping existing base image for cache optimization"

    print_message "${YELLOW}" "Building agent_foundry_base with BuildKit cache..."
    DOCKER_BUILDKIT=1 docker compose build agent_foundry_base

    print_message "${GREEN}" "Base image rebuilt successfully!"
    print_message "${YELLOW}" "Tip: Use 'redeploy <agent>' to rebuild specific agent, or 'full-redeploy <agent>' to rebuild both"
}

# ============================================================================
# Command: full-redeploy <agent>
# ============================================================================
full_redeploy_agent() {
    local agent=$1
    print_message "${BLUE}" "=== Full redeploy: $agent ==="
    print_message "${BLUE}" "This will rebuild base + agent and start the agent"
    echo ""

    print_message "${YELLOW}" "1. Stopping and removing $agent container..."
    docker compose stop $agent
    docker compose rm -f $agent

    IMAGE_ID=$(docker images -q blacksmith_hub-${agent})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "2. Removing old $agent image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    # OPTIMIZATION: Keep base image for cache optimization
    print_message "${YELLOW}" "3. Rebuilding base image (with cache)..."
    DOCKER_BUILDKIT=1 docker compose build agent_foundry_base

    print_message "${YELLOW}" "4. Building $agent..."
    docker compose build $agent

    print_message "${YELLOW}" "5. Starting $agent..."
    docker compose up -d $agent

    print_message "${GREEN}" "Full redeploy of $agent completed successfully!"
}

# ============================================================================
# Argument parsing
# ============================================================================

if [ -z "$1" ]; then
    print_message "${RED}" "ERROR: No command specified"
    echo ""
    print_help
    exit 1
fi

if [ "$1" == "help" ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    print_help
    exit 0
fi

COMMAND=$1
OBJECT=$2

# Validate command
case "$COMMAND" in
    restart|redeploy|full-redeploy)
        # These commands require an agent name
        if [ -z "$OBJECT" ]; then
            print_message "${RED}" "ERROR: Command '$COMMAND' requires an agent name"
            echo ""
            print_help
            exit 1
        fi

        # Validate agent exists
        AGENTS=$(docker compose config --services | grep "^agent_" | grep -v "_base")
        if ! echo "$AGENTS" | grep -q "^${OBJECT}$"; then
            print_message "${RED}" "ERROR: Agent '$OBJECT' does not exist"
            print_message "${YELLOW}" "Available agents:"
            echo "$AGENTS"
            exit 1
        fi
        ;;
    rebuild)
        # rebuild command expects 'base' as object
        if [ "$OBJECT" != "base" ]; then
            print_message "${RED}" "ERROR: Command 'rebuild' only supports 'base' as object"
            echo "Usage: ./agent_foundry_manager.sh rebuild base"
            exit 1
        fi
        ;;
    *)
        print_message "${RED}" "ERROR: Unknown command: $COMMAND"
        echo ""
        print_help
        exit 1
        ;;
esac

# ============================================================================
# Execute command
# ============================================================================

case "$COMMAND" in
    restart)
        restart_agent $OBJECT
        ;;
    redeploy)
        redeploy_agent $OBJECT
        ;;
    rebuild)
        rebuild_base
        ;;
    full-redeploy)
        full_redeploy_agent $OBJECT
        ;;
esac

exit 0
