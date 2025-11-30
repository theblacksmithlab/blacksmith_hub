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
    print_message "${YELLOW}" "Usage:"
    echo "  ./agent_foundry_manager.sh base [rebuild]           - Rebuild the base image"
    echo "  ./agent_foundry_manager.sh <agent> [restart|redeploy] - Manage a specific agent"
    echo "  ./agent_foundry_manager.sh full-redeploy <agent>      - Rebuild base and redeploy specific agent only"
    echo ""
    echo "Examples:"
    echo "  ./agent_foundry_manager.sh base rebuild             - Rebuild agent_foundry_base image"
    echo "  ./agent_foundry_manager.sh agent_davon restart      - Restart the agent_davon"
    echo "  ./agent_foundry_manager.sh agent_davon redeploy     - Full redeploy of the agent_davon"
    echo "  ./agent_foundry_manager.sh full-redeploy agent_davon - Rebuild base and redeploy agent_davon only"
    echo ""

    print_message "${YELLOW}" "Available agents:"
    AGENTS=$(docker compose config --services | grep "^agent_" | grep -v "_base")
    for agent in $AGENTS; do
        echo "  - $agent"
    done
}

rebuild_base() {
    print_message "${GREEN}" "Rebuilding agent_foundry_base image..."

    print_message "${YELLOW}" "Stopping and removing dependent agent containers..."
    DEPENDENT_AGENTS=$(docker compose config --services | grep "^agent_")
    for agent in $DEPENDENT_AGENTS; do
        docker compose stop $agent
        docker compose rm -f $agent
    done

    BASE_IMAGE_ID=$(docker images -q agent_foundry_base)
    if [[ -n "$BASE_IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing agent_foundry_base image: $BASE_IMAGE_ID"
        docker rmi -f "$BASE_IMAGE_ID"
    fi

    print_message "${GREEN}" "Building agent_foundry_base image..."
    docker compose build agent_foundry_base

    print_message "${GREEN}" "Rebuilding dependent agents..."
    for agent in $DEPENDENT_AGENTS; do
        docker compose build $agent
    done

    print_message "${GREEN}" "Base image and dependent agents rebuilt successfully!"
    print_message "${YELLOW}" "To start agents, use: docker compose up -d <agent_name>"
}

restart_agent() {
    local agent=$1
    print_message "${GREEN}" "Simple restart for agent: $agent..."
    docker compose restart $agent
    print_message "${GREEN}" "Agent restarted successfully!"
}

redeploy_agent() {
    local agent=$1
    print_message "${GREEN}" "Full redeployment for agent: $agent..."
    docker compose stop $agent
    docker compose rm -f $agent

    IMAGE_ID=$(docker images -q blacksmith_lab_${agent})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing old image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    docker compose build $agent
    docker compose up -d $agent
    print_message "${GREEN}" "Agent redeployed successfully!"
}

if [ -z "$1" ]; then
    print_message "${RED}" "ACHTUNG! No agent specified."
    print_help
    exit 1
fi

if [ "$1" == "full-redeploy" ]; then
    AGENT_NAME=$2

    if [ -z "$AGENT_NAME" ]; then
        print_message "${RED}" "ACHTUNG! No agent specified for full-redeploy."
        print_help
        exit 1
    fi

    AGENTS=$(docker compose config --services | grep "^agent_" | grep -v "_base")
    if ! echo "$AGENTS" | grep -q "$AGENT_NAME"; then
        print_message "${RED}" "ACHTUNG! Agent '$AGENT_NAME' does not exist."
        print_message "${YELLOW}" "Available agents:"
        echo "$AGENTS"
        exit 1
    fi

    print_message "${BLUE}" "=== Full redeployment process for $AGENT_NAME ==="

    print_message "${YELLOW}" "1. Stopping and removing $AGENT_NAME..."
    docker compose stop $AGENT_NAME
    docker compose rm -f $AGENT_NAME

    IMAGE_ID=$(docker images -q blacksmith_lab_${AGENT_NAME})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing $AGENT_NAME image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    print_message "${YELLOW}" "2. Rebuilding base image..."
    BASE_IMAGE_ID=$(docker images -q agent_foundry_base)
    if [[ -n "$BASE_IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing agent_foundry_base image: $BASE_IMAGE_ID"
        docker rmi -f "$BASE_IMAGE_ID"
    fi

    docker compose build agent_foundry_base

    print_message "${YELLOW}" "3. Rebuilding and starting $AGENT_NAME..."
    docker compose build $AGENT_NAME
    docker compose up -d $AGENT_NAME

    print_message "${GREEN}" "Full redeployment of $AGENT_NAME completed successfully!"
    exit 0
fi

if [ "$1" == "help" ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    print_help
    exit 0
fi

AGENT_NAME=$1
ACTION=$2

if [ "$AGENT_NAME" == "base" ]; then
    if [ "$ACTION" == "rebuild" ]; then
        rebuild_base
    else
        print_message "${RED}" "ACHTUNG! Invalid action for base. Use 'rebuild'."
        print_help
        exit 1
    fi
    exit 0
fi

AGENTS=$(docker compose config --services | grep "^agent_" | grep -v "_base")
if ! echo "$AGENTS" | grep -q "$AGENT_NAME"; then
    print_message "${RED}" "ACHTUNG! Agent '$AGENT_NAME' does not exist."
    print_message "${YELLOW}" "Available agents:"
    echo "$AGENTS"
    exit 1
fi

if [ -z "$ACTION" ]; then
    print_message "${RED}" "ACHTUNG! No action specified."
    print_help
    exit 1
fi

case "$ACTION" in
    restart)
        restart_agent $AGENT_NAME
        ;;
    redeploy)
        redeploy_agent $AGENT_NAME
        ;;
    *)
        print_message "${RED}" "ACHTUNG! Invalid action: $ACTION"
        print_help
        exit 1
        ;;
esac

exit 0