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
    print_message "${BLUE}" "=== Bot Foundry Manager ==="
    echo ""
    print_message "${YELLOW}" "Usage:"
    echo "  ./bot_foundry_manager.sh <COMMAND> <OBJECT>"
    echo ""
    print_message "${YELLOW}" "Commands:"
    echo "  restart <bot>          - Restart bot container (no rebuild)"
    echo "  redeploy <bot>         - Rebuild bot only (base untouched) + start"
    echo "  rebuild base           - Rebuild base image only (bots untouched)"
    echo "  full-redeploy <bot>    - Rebuild base + bot + start"
    echo ""
    print_message "${YELLOW}" "Examples:"
    echo "  ./bot_foundry_manager.sh restart probiot_bot"
    echo "  ./bot_foundry_manager.sh redeploy probiot_bot"
    echo "  ./bot_foundry_manager.sh rebuild base"
    echo "  ./bot_foundry_manager.sh full-redeploy probiot_bot"
    echo ""
    print_message "${YELLOW}" "Available bots:"
    BOTS=$(docker compose config --services | grep "_bot$")
    for bot in $BOTS; do
        echo "  - $bot"
    done
}

# ============================================================================
# Command: restart <bot>
# ============================================================================
restart_bot() {
    local bot=$1
    print_message "${GREEN}" "Restarting bot: $bot..."
    docker compose restart $bot
    print_message "${GREEN}" "Bot restarted successfully!"
}

# ============================================================================
# Command: redeploy <bot>
# ============================================================================
redeploy_bot() {
    local bot=$1
    print_message "${GREEN}" "Redeploying bot: $bot (base image untouched)..."

    print_message "${YELLOW}" "1. Stopping and removing $bot container..."
    docker compose stop $bot
    docker compose rm -f $bot

    IMAGE_ID=$(docker images -q blacksmith-core_${bot})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "2. Removing old $bot image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    print_message "${YELLOW}" "3. Building $bot..."
    docker compose build $bot

    print_message "${YELLOW}" "4. Starting $bot..."
    docker compose up -d $bot

    print_message "${GREEN}" "Bot redeployed successfully!"
}

# ============================================================================
# Command: rebuild base
# ============================================================================
rebuild_base() {
    print_message "${GREEN}" "Rebuilding bot_foundry_base image only..."

    # OPTIMIZATION: Do NOT remove the base image - this preserves BuildKit cache layers!
    print_message "${YELLOW}" "Note: Keeping existing base image for cache optimization"

    print_message "${YELLOW}" "Building bot_foundry_base with BuildKit cache..."
    DOCKER_BUILDKIT=1 docker compose build bot_foundry_base

    print_message "${GREEN}" "Base image rebuilt successfully!"
    print_message "${YELLOW}" "Tip: Use 'redeploy <bot>' to rebuild specific bot, or 'full-redeploy <bot>' to rebuild both"
}

# ============================================================================
# Command: full-redeploy <bot>
# ============================================================================
full_redeploy_bot() {
    local bot=$1
    print_message "${BLUE}" "=== Full redeploy: $bot ==="
    print_message "${BLUE}" "This will rebuild base + bot and start the bot"
    echo ""

    print_message "${YELLOW}" "1. Stopping and removing $bot container..."
    docker compose stop $bot
    docker compose rm -f $bot

    IMAGE_ID=$(docker images -q blacksmith-core_${bot})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "2. Removing old $bot image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    # OPTIMIZATION: Keep base image for cache optimization
    print_message "${YELLOW}" "3. Rebuilding base image (with cache)..."
    DOCKER_BUILDKIT=1 docker compose build bot_foundry_base

    print_message "${YELLOW}" "4. Building $bot..."
    docker compose build $bot

    print_message "${YELLOW}" "5. Starting $bot..."
    docker compose up -d $bot

    print_message "${GREEN}" "Full redeploy of $bot completed successfully!"
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
        # These commands require a bot name
        if [ -z "$OBJECT" ]; then
            print_message "${RED}" "ERROR: Command '$COMMAND' requires a bot name"
            echo ""
            print_help
            exit 1
        fi

        # Validate bot exists
        BOTS=$(docker compose config --services | grep "_bot$")
        if ! echo "$BOTS" | grep -q "^${OBJECT}$"; then
            print_message "${RED}" "ERROR: Bot '$OBJECT' does not exist"
            print_message "${YELLOW}" "Available bots:"
            echo "$BOTS"
            exit 1
        fi
        ;;
    rebuild)
        # rebuild command expects 'base' as object
        if [ "$OBJECT" != "base" ]; then
            print_message "${RED}" "ERROR: Command 'rebuild' only supports 'base' as object"
            echo "Usage: ./bot_foundry_manager.sh rebuild base"
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
        restart_bot $OBJECT
        ;;
    redeploy)
        redeploy_bot $OBJECT
        ;;
    rebuild)
        rebuild_base
        ;;
    full-redeploy)
        full_redeploy_bot $OBJECT
        ;;
esac

exit 0
