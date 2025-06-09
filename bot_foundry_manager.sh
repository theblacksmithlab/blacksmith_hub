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
    print_message "${YELLOW}" "Usage:"
    echo "  ./bot_foundry_manager.sh base [rebuild]           - Rebuild the base image"
    echo "  ./bot_foundry_manager.sh <bot> [restart|redeploy] - Manage a specific bot"
    echo "  ./bot_foundry_manager.sh full-redeploy <bot>      - Rebuild base and redeploy specific bot only"
    echo ""
    echo "Examples:"
    echo "  ./bot_foundry_manager.sh base rebuild             - Rebuild bot_foundry_base image"
    echo "  ./bot_foundry_manager.sh probiot_bot restart      - Restart the probiot_bot"
    echo "  ./bot_foundry_manager.sh groot_bot redeploy       - Full redeploy of the groot_bot"
    echo "  ./bot_foundry_manager.sh full-redeploy probiot_bot - Rebuild base and redeploy probiot_bot only"
    echo ""

    print_message "${YELLOW}" "Available bots:"
    BOTS=$(docker-compose config --services | grep "_bot$")
    for bot in $BOTS; do
        echo "  - $bot"
    done
}

rebuild_base() {
    print_message "${GREEN}" "Rebuilding bot_foundry_base image..."

    print_message "${YELLOW}" "Stopping and removing dependent bot containers..."
    DEPENDENT_BOTS=$(docker-compose config --services | grep "_bot$")
    for bot in $DEPENDENT_BOTS; do
        docker-compose stop $bot
        docker-compose rm -f $bot
    done

    BASE_IMAGE_ID=$(docker images -q bot_foundry_base)
    if [[ -n "$BASE_IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing bot_foundry_base image: $BASE_IMAGE_ID"
        docker rmi -f "$BASE_IMAGE_ID"
    fi

    print_message "${GREEN}" "Building bot_foundry_base image..."
    docker-compose build bot_foundry_base

    print_message "${GREEN}" "Rebuilding dependent bots..."
    for bot in $DEPENDENT_BOTS; do
        docker-compose build $bot
    done

    print_message "${GREEN}" "Base image and dependent bots rebuilt successfully!"
    print_message "${YELLOW}" "To start bots, use: docker-compose up -d <bot_name>"
}

restart_bot() {
    local bot=$1
    print_message "${GREEN}" "Simple restart for bot: $bot..."
    docker-compose restart $bot
    print_message "${GREEN}" "Bot restarted successfully!"
}

redeploy_bot() {
    local bot=$1
    print_message "${GREEN}" "Full redeployment for bot: $bot..."
    docker-compose stop $bot
    docker-compose rm -f $bot

    IMAGE_ID=$(docker images -q blacksmith_lab_${bot})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing old image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    docker-compose build $bot
    docker-compose up -d $bot
    print_message "${GREEN}" "Bot redeployed successfully!"
}

if [ -z "$1" ]; then
    print_message "${RED}" "ACHTUNG! No bot specified."
    print_help
    exit 1
fi

if [ "$1" == "full-redeploy" ]; then
    BOT_NAME=$2

    if [ -z "$BOT_NAME" ]; then
        print_message "${RED}" "ACHTUNG! No bot specified for full-redeploy."
        print_help
        exit 1
    fi

    BOTS=$(docker-compose config --services | grep "_bot$")
    if ! echo "$BOTS" | grep -q "$BOT_NAME"; then
        print_message "${RED}" "ACHTUNG! Bot '$BOT_NAME' does not exist."
        print_message "${YELLOW}" "Available bots:"
        echo "$BOTS"
        exit 1
    fi

    print_message "${BLUE}" "=== Full redeployment process for $BOT_NAME ==="

    print_message "${YELLOW}" "1. Stopping and removing $BOT_NAME..."
    docker-compose stop $BOT_NAME
    docker-compose rm -f $BOT_NAME

    IMAGE_ID=$(docker images -q blacksmith_lab_${BOT_NAME})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing $BOT_NAME image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    print_message "${YELLOW}" "2. Rebuilding base image..."
    BASE_IMAGE_ID=$(docker images -q bot_foundry_base)
    if [[ -n "$BASE_IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing bot_foundry_base image: $BASE_IMAGE_ID"
        docker rmi -f "$BASE_IMAGE_ID"
    fi

    docker-compose build bot_foundry_base

    print_message "${YELLOW}" "3. Rebuilding and starting $BOT_NAME..."
    docker-compose build $BOT_NAME
    docker-compose up -d $BOT_NAME

    print_message "${GREEN}" "Full redeployment of $BOT_NAME completed successfully!"
    exit 0
fi

if [ "$1" == "help" ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    print_help
    exit 0
fi

BOT_NAME=$1
ACTION=$2

if [ "$BOT_NAME" == "base" ]; then
    if [ "$ACTION" == "rebuild" ]; then
        rebuild_base
    else
        print_message "${RED}" "ACHTUNG! Invalid action for base. Use 'rebuild'."
        print_help
        exit 1
    fi
    exit 0
fi

BOTS=$(docker-compose config --services | grep "_bot$")
if ! echo "$BOTS" | grep -q "$BOT_NAME"; then
    print_message "${RED}" "ACHTUNG! Bot '$BOT_NAME' does not exist."
    print_message "${YELLOW}" "Available bots:"
    echo "$BOTS"
    exit 1
fi

if [ -z "$ACTION" ]; then
    print_message "${RED}" "ACHTUNG! No action specified."
    print_help
    exit 1
fi

case "$ACTION" in
    restart)
        restart_bot $BOT_NAME
        ;;
    redeploy)
        redeploy_bot $BOT_NAME
        ;;
    *)
        print_message "${RED}" "ACHTUNG! Invalid action: $ACTION"
        print_help
        exit 1
        ;;
esac

exit 0