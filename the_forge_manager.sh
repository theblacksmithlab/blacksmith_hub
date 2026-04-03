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
    print_message "${BLUE}" "=== The Forge Manager ==="
    echo ""
    print_message "${YELLOW}" "Usage:"
    echo "  ./the_forge_manager.sh <COMMAND> <OBJECT>"
    echo ""
    print_message "${YELLOW}" "Commands:"
    echo "  restart <service>          - Restart service container (no rebuild)"
    echo "  redeploy <service>         - Rebuild service only (base untouched) + start"
    echo "  rebuild base               - Rebuild base image only (services untouched)"
    echo "  full-redeploy <service>    - Rebuild base + service + start"
    echo ""
    print_message "${YELLOW}" "Examples:"
    echo "  ./the_forge_manager.sh restart blacksmith_web"
    echo "  ./the_forge_manager.sh redeploy uniframe_studio"
    echo "  ./the_forge_manager.sh rebuild base"
    echo "  ./the_forge_manager.sh full-redeploy the_viper_room"
    echo ""
    print_message "${YELLOW}" "Available services:"
    SERVICES=$(docker compose config --services | grep -E "^(blacksmith_web|uniframe_studio|the_viper_room)$")
    for service in $SERVICES; do
        echo "  - $service"
    done
}

# ============================================================================
# Command: restart <service>
# ============================================================================
restart_service() {
    local service=$1
    print_message "${GREEN}" "Restarting service: $service..."
    docker compose restart $service
    print_message "${GREEN}" "Service restarted successfully!"
}

# ============================================================================
# Command: redeploy <service>
# ============================================================================
redeploy_service() {
    local service=$1
    print_message "${GREEN}" "Redeploying service: $service (base image untouched)..."

    print_message "${YELLOW}" "1. Stopping and removing $service container..."
    docker compose stop $service
    docker compose rm -f $service

    IMAGE_ID=$(docker images -q blacksmith_hub-${service})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "2. Removing old $service image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    print_message "${YELLOW}" "3. Building $service..."
    docker compose build $service

    print_message "${YELLOW}" "4. Starting $service..."
    docker compose up -d $service

    print_message "${GREEN}" "Service redeployed successfully!"
}

# ============================================================================
# Command: rebuild base
# ============================================================================
rebuild_base() {
    print_message "${GREEN}" "Rebuilding the_forge_base image only..."

    # OPTIMIZATION: Do NOT remove the base image - this preserves BuildKit cache layers!
    print_message "${YELLOW}" "Note: Keeping existing base image for cache optimization"

    print_message "${YELLOW}" "Building the_forge_base with BuildKit cache..."
    DOCKER_BUILDKIT=1 docker compose build the_forge_base

    print_message "${GREEN}" "Base image rebuilt successfully!"
    print_message "${YELLOW}" "Tip: Use 'redeploy <service>' to rebuild specific service, or 'full-redeploy <service>' to rebuild both"
}

# ============================================================================
# Command: full-redeploy <service>
# ============================================================================
full_redeploy_service() {
    local service=$1
    print_message "${BLUE}" "=== Full redeploy: $service ==="
    print_message "${BLUE}" "This will rebuild base + service and start the service"
    echo ""

    print_message "${YELLOW}" "1. Stopping and removing $service container..."
    docker compose stop $service
    docker compose rm -f $service

    IMAGE_ID=$(docker images -q blacksmith_hub-${service})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "2. Removing old $service image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    # OPTIMIZATION: Keep base image for cache optimization
    print_message "${YELLOW}" "3. Rebuilding base image (with cache)..."
    DOCKER_BUILDKIT=1 docker compose build the_forge_base

    print_message "${YELLOW}" "4. Building $service..."
    docker compose build $service

    print_message "${YELLOW}" "5. Starting $service..."
    docker compose up -d $service

    print_message "${GREEN}" "Full redeploy of $service completed successfully!"
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
        # These commands require a service name
        if [ -z "$OBJECT" ]; then
            print_message "${RED}" "ERROR: Command '$COMMAND' requires a service name"
            echo ""
            print_help
            exit 1
        fi

        # Validate service exists
        SERVICES=$(docker compose config --services | grep -E "^(blacksmith_web|uniframe_studio|the_viper_room)$")
        if ! echo "$SERVICES" | grep -q "^${OBJECT}$"; then
            print_message "${RED}" "ERROR: Service '$OBJECT' does not exist"
            print_message "${YELLOW}" "Available services:"
            echo "$SERVICES"
            exit 1
        fi
        ;;
    rebuild)
        # rebuild command expects 'base' as object
        if [ "$OBJECT" != "base" ]; then
            print_message "${RED}" "ERROR: Command 'rebuild' only supports 'base' as object"
            echo "Usage: ./the_forge_manager.sh rebuild base"
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
        restart_service $OBJECT
        ;;
    redeploy)
        redeploy_service $OBJECT
        ;;
    rebuild)
        rebuild_base
        ;;
    full-redeploy)
        full_redeploy_service $OBJECT
        ;;
esac

exit 0
