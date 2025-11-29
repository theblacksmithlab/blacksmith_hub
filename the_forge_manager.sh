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
    print_message "${YELLOW}" "Usage:"
    echo "  ./the_forge_manager.sh base [rebuild]             - Rebuild the base image"
    echo "  ./the_forge_manager.sh <service> [restart|redeploy]  - Manage a specific service"
    echo "  ./the_forge_manager.sh full-redeploy <service>    - Rebuild base and redeploy specific service only"
    echo ""
    echo "Examples:"
    echo "  ./the_forge_manager.sh base rebuild               - Rebuild the_forge_base image"
    echo "  ./the_forge_manager.sh blacksmith_web restart     - Restart the blacksmith_web service"
    echo "  ./the_forge_manager.sh uniframe_studio redeploy   - Full redeploy of the uniframe_studio service"
    echo "  ./the_forge_manager.sh the_viper_room redeploy    - Full redeploy of the the_viper_room service"
    echo "  ./the_forge_manager.sh full-redeploy blacksmith_web - Rebuild base and redeploy blacksmith_web only"
    echo ""

    print_message "${YELLOW}" "Available services:"
    SERVICES=$(docker compose config --services | grep -E "^(blacksmith_web|uniframe_studio|the_viper_room)$")
    for service in $SERVICES; do
        echo "  - $service"
    done
}

rebuild_base() {
    print_message "${GREEN}" "Rebuilding the_forge_base image..."

    print_message "${YELLOW}" "Stopping and removing dependent containers..."
    DEPENDENT_SERVICES=$(docker compose config --services | grep -E "^(blacksmith_web|uniframe_studio|the_viper_room)$")
    for service in $DEPENDENT_SERVICES; do
        docker compose stop $service
        docker compose rm -f $service
    done

    BASE_IMAGE_ID=$(docker images -q the_forge_base)
    if [[ -n "$BASE_IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing the_forge_base image: $BASE_IMAGE_ID"
        docker rmi -f "$BASE_IMAGE_ID"
    fi

    print_message "${GREEN}" "Building the_forge_base image..."
    docker compose build the_forge_base

    print_message "${GREEN}" "Rebuilding dependent services..."
    for service in $DEPENDENT_SERVICES; do
        docker compose build $service
    done

    print_message "${GREEN}" "Base image and dependent services rebuilt successfully!"
    print_message "${YELLOW}" "To start services, use: docker compose up -d <service_name>"
}

restart_service() {
    local service=$1
    print_message "${GREEN}" "Simple restart for service: $service..."
    docker compose restart $service
    print_message "${GREEN}" "Service restarted successfully!"
}

redeploy_service() {
    local service=$1
    print_message "${GREEN}" "Full redeployment for service: $service..."
    docker compose stop $service
    docker compose rm -f $service

    IMAGE_ID=$(docker images -q blacksmith_lab_${service})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing old image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    docker compose build $service
    docker compose up -d $service
    print_message "${GREEN}" "Service redeployed successfully!"
}

if [ -z "$1" ]; then
    print_message "${RED}" "ACHTUNG! No service specified."
    print_help
    exit 1
fi

if [ "$1" == "full-redeploy" ]; then
    SERVICE_NAME=$2

    if [ -z "$SERVICE_NAME" ]; then
        print_message "${RED}" "ACHTUNG! No service specified for full-redeploy."
        print_help
        exit 1
    fi

    SERVICES=$(docker compose config --services | grep -E "^(blacksmith_web|uniframe_studio|the_viper_room)$")
    if ! echo "$SERVICES" | grep -q "$SERVICE_NAME"; then
        print_message "${RED}" "ACHTUNG! Service '$SERVICE_NAME' does not exist."
        print_message "${YELLOW}" "Available services:"
        echo "$SERVICES"
        exit 1
    fi

    print_message "${BLUE}" "=== Full redeployment process for $SERVICE_NAME ==="

    print_message "${YELLOW}" "1. Stopping and removing $SERVICE_NAME..."
    docker compose stop $SERVICE_NAME
    docker compose rm -f $SERVICE_NAME

    IMAGE_ID=$(docker images -q blacksmith_lab_${SERVICE_NAME})
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing $SERVICE_NAME image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    print_message "${YELLOW}" "2. Rebuilding base image..."
    BASE_IMAGE_ID=$(docker images -q the_forge_base)
    if [[ -n "$BASE_IMAGE_ID" ]]; then
        print_message "${YELLOW}" "Removing the_forge_base image: $BASE_IMAGE_ID"
        docker rmi -f "$BASE_IMAGE_ID"
    fi

    docker compose build the_forge_base

    print_message "${YELLOW}" "3. Rebuilding and starting $SERVICE_NAME..."
    docker compose build $SERVICE_NAME
    docker compose up -d $SERVICE_NAME

    print_message "${GREEN}" "Full redeployment of $SERVICE_NAME completed successfully!"
    exit 0
fi

if [ "$1" == "help" ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    print_help
    exit 0
fi

SERVICE_NAME=$1
ACTION=$2

if [ "$SERVICE_NAME" == "base" ]; then
    if [ "$ACTION" == "rebuild" ]; then
        rebuild_base
    else
        print_message "${RED}" "ACHTUNG! Invalid action for base. Use 'rebuild'."
        print_help
        exit 1
    fi
    exit 0
fi

SERVICES=$(docker compose config --services | grep -E "^(blacksmith_web|uniframe_studio|the_viper_room)$")
if ! echo "$SERVICES" | grep -q "$SERVICE_NAME"; then
    print_message "${RED}" "ACHTUNG! Service '$SERVICE_NAME' does not exist."
    print_message "${YELLOW}" "Available services:"
    echo "$SERVICES"
    exit 1
fi

if [ -z "$ACTION" ]; then
    print_message "${RED}" "ACHTUNG! No action specified."
    print_help
    exit 1
fi

case "$ACTION" in
    restart)
        restart_service $SERVICE_NAME
        ;;
    redeploy)
        redeploy_service $SERVICE_NAME
        ;;
    *)
        print_message "${RED}" "ACHTUNG! Invalid action: $ACTION"
        print_help
        exit 1
        ;;
esac

exit 0