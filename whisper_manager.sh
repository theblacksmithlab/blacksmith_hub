#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SERVICE_NAME="whisper"

print_message() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_help() {
    print_message "${BLUE}" "=== Whisper Service Manager ==="
    echo ""
    print_message "${YELLOW}" "Usage:"
    echo "  ./whisper_manager.sh <COMMAND> [OPTIONS]"
    echo ""
    print_message "${YELLOW}" "Commands:"
    echo "  start            - Start whisper service"
    echo "  stop             - Stop whisper service"
    echo "  restart          - Restart whisper service (no rebuild)"
    echo "  rebuild [MODEL]  - Rebuild whisper image and restart"
    echo "                     MODEL: base|small|medium|large (default: small)"
    echo "  logs             - Show whisper service logs"
    echo "  status           - Show whisper service status"
    echo "  help             - Show this help"
    echo ""
    print_message "${YELLOW}" "Examples:"
    echo "  ./whisper_manager.sh start"
    echo "  ./whisper_manager.sh restart"
    echo "  ./whisper_manager.sh rebuild medium"
    echo "  ./whisper_manager.sh logs"
    echo ""
    print_message "${BLUE}" "Model sizes:"
    echo "  base   - 142 MB  (fast, lower quality)"
    echo "  small  - 466 MB  (balanced, default)"
    echo "  medium - 1.5 GB  (better quality, slower)"
    echo "  large  - 2.9 GB  (maximum quality)"
}

# ============================================================================
# Command: start
# ============================================================================
start_service() {
    print_message "${GREEN}" "Starting whisper service..."
    docker compose up -d $SERVICE_NAME
    print_message "${GREEN}" "✓ Service started!"
}

# ============================================================================
# Command: stop
# ============================================================================
stop_service() {
    print_message "${YELLOW}" "Stopping whisper service..."
    docker compose stop $SERVICE_NAME
    print_message "${GREEN}" "✓ Service stopped"
}

# ============================================================================
# Command: restart
# ============================================================================
restart_service() {
    print_message "${GREEN}" "Restarting whisper service..."
    docker compose restart $SERVICE_NAME
    print_message "${GREEN}" "✓ Service restarted!"
}

# ============================================================================
# Command: rebuild
# ============================================================================
rebuild_service() {
    local model=${1:-small}

    print_message "${GREEN}" "Rebuilding whisper service with model: $model..."

    print_message "${YELLOW}" "1. Stopping whisper service..."
    docker compose stop $SERVICE_NAME

    print_message "${YELLOW}" "2. Removing old container..."
    docker compose rm -f $SERVICE_NAME

    IMAGE_ID=$(docker images -q whisper:latest)
    if [[ -n "$IMAGE_ID" ]]; then
        print_message "${YELLOW}" "3. Removing old image: $IMAGE_ID"
        docker rmi -f "$IMAGE_ID"
    fi

    print_message "${YELLOW}" "4. Building whisper with $model model..."
    docker compose build --build-arg WHISPER_MODEL=$model $SERVICE_NAME

    print_message "${YELLOW}" "5. Starting whisper service..."
    docker compose up -d $SERVICE_NAME

    print_message "${GREEN}" "✓ Rebuild complete!"
}

# ============================================================================
# Command: logs
# ============================================================================
show_logs() {
    print_message "${BLUE}" "Showing whisper service logs (Ctrl+C to exit)..."
    docker compose logs -f $SERVICE_NAME
}

# ============================================================================
# Command: status
# ============================================================================
show_status() {
    print_message "${BLUE}" "=== Whisper Service Status ==="
    echo ""

    if docker compose ps $SERVICE_NAME | grep -q "Up"; then
        print_message "${GREEN}" "Status: RUNNING ✓"
        docker compose ps $SERVICE_NAME

        echo ""
        print_message "${BLUE}" "Service URL (internal): http://whisper:9000"

        echo ""
        print_message "${YELLOW}" "Testing endpoint..."
        CONTAINER_NAME=$(docker compose ps -q $SERVICE_NAME)
        if [[ -n "$CONTAINER_NAME" ]]; then
            docker exec $CONTAINER_NAME sh -c "command -v curl >/dev/null 2>&1 && curl -s -o /dev/null -w '%{http_code}' http://localhost:9000/transcribe -X POST 2>/dev/null || echo 'curl not available'" | grep -q "400\|405" && \
                print_message "${GREEN}" "✓ Service is responding" || \
                print_message "${RED}" "✗ Service not responding"
        fi
    else
        print_message "${RED}" "Status: STOPPED ✗"
        docker compose ps $SERVICE_NAME
        echo ""
        echo "Run: ./whisper_manager.sh start"
    fi
}

# ============================================================================
# Main
# ============================================================================

if [ $# -eq 0 ]; then
    print_help
    exit 0
fi

COMMAND=$1
shift

case $COMMAND in
    start)
        start_service
        ;;
    stop)
        stop_service
        ;;
    restart)
        restart_service
        ;;
    rebuild)
        rebuild_service $1
        ;;
    logs)
        show_logs
        ;;
    status)
        show_status
        ;;
    help|--help|-h)
        print_help
        ;;
    *)
        print_message "${RED}" "Unknown command: $COMMAND"
        echo ""
        print_help
        exit 1
        ;;
esac
