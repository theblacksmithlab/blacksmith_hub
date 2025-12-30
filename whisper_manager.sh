#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

CONTAINER_NAME="whisper_service"
IMAGE_NAME="whisper:latest"
PORT="9000"
WHISPER_MODEL="${WHISPER_MODEL:-medium}"

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
    echo "  build [MODEL]    - Build whisper Docker image"
    echo "                     MODEL: base|small|medium (default: medium)"
    echo "  start            - Start whisper service"
    echo "  stop             - Stop whisper service"
    echo "  restart          - Restart whisper service (no rebuild)"
    echo "  rebuild [MODEL]  - Rebuild image and restart service"
    echo "  logs             - Show whisper service logs"
    echo "  status           - Show whisper service status"
    echo "  clean            - Stop and remove container + image"
    echo "  help             - Show this help"
    echo ""
    print_message "${YELLOW}" "Examples:"
    echo "  ./whisper_manager.sh build              # Build with medium model"
    echo "  ./whisper_manager.sh build base         # Build with base model"
    echo "  ./whisper_manager.sh start              # Start service"
    echo "  ./whisper_manager.sh rebuild small      # Rebuild with small model"
    echo "  ./whisper_manager.sh logs               # Show logs"
    echo ""
    print_message "${YELLOW}" "Environment Variables:"
    echo "  WHISPER_MODEL    - Model to use (default: medium)"
    echo "  PORT             - Service port (default: 9000)"
    echo ""
    print_message "${BLUE}" "Model sizes:"
    echo "  base   - 142 MB  (fast, lower quality)"
    echo "  small  - 466 MB  (balanced)"
    echo "  medium - 1.5 GB  (best quality, default)"
}

# ============================================================================
# Command: build
# ============================================================================
build_image() {
    local model=${1:-$WHISPER_MODEL}

    print_message "${GREEN}" "Building whisper image with model: $model..."

    if docker build \
        --build-arg WHISPER_MODEL=$model \
        -f docker/Dockerfile.whisper \
        -t $IMAGE_NAME \
        .; then
        print_message "${GREEN}" "✓ Image built successfully with $model model!"
    else
        print_message "${RED}" "✗ Failed to build image"
        exit 1
    fi
}

# ============================================================================
# Command: start
# ============================================================================
start_service() {
    if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
            print_message "${YELLOW}" "⚠ Service is already running"
            return 0
        else
            print_message "${YELLOW}" "Starting existing container..."
            docker start $CONTAINER_NAME
            print_message "${GREEN}" "✓ Service started!"
            return 0
        fi
    fi

    print_message "${GREEN}" "Starting whisper service on port $PORT..."

    if docker run -d \
        --name $CONTAINER_NAME \
        -p 127.0.0.1:$PORT:9000 \
        --restart unless-stopped \
        $IMAGE_NAME; then
        print_message "${GREEN}" "✓ Service started successfully!"
        print_message "${BLUE}" "Service available at: http://127.0.0.1:$PORT"
    else
        print_message "${RED}" "✗ Failed to start service"
        exit 1
    fi
}

# ============================================================================
# Command: stop
# ============================================================================
stop_service() {
    print_message "${YELLOW}" "Stopping whisper service..."

    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        docker stop $CONTAINER_NAME
        print_message "${GREEN}" "✓ Service stopped"
    else
        print_message "${YELLOW}" "⚠ Service is not running"
    fi
}

# ============================================================================
# Command: restart
# ============================================================================
restart_service() {
    print_message "${GREEN}" "Restarting whisper service..."
    stop_service
    sleep 1
    start_service
}

# ============================================================================
# Command: rebuild
# ============================================================================
rebuild_service() {
    local model=${1:-$WHISPER_MODEL}

    print_message "${GREEN}" "Rebuilding whisper service with model: $model..."

    stop_service

    if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        print_message "${YELLOW}" "Removing old container..."
        docker rm -f $CONTAINER_NAME
    fi

    if docker images --format '{{.Repository}}:{{.Tag}}' | grep -q "^${IMAGE_NAME}$"; then
        print_message "${YELLOW}" "Removing old image..."
        docker rmi -f $IMAGE_NAME
    fi

    build_image $model
    start_service

    print_message "${GREEN}" "✓ Rebuild complete!"
}

# ============================================================================
# Command: logs
# ============================================================================
show_logs() {
    if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        print_message "${BLUE}" "Showing whisper service logs (Ctrl+C to exit)..."
        docker logs -f $CONTAINER_NAME
    else
        print_message "${RED}" "✗ Container not found"
        exit 1
    fi
}

# ============================================================================
# Command: status
# ============================================================================
show_status() {
    print_message "${BLUE}" "=== Whisper Service Status ==="
    echo ""

    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        print_message "${GREEN}" "Status: RUNNING ✓"

        # Show container details
        docker ps --filter "name=${CONTAINER_NAME}" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

        echo ""
        print_message "${BLUE}" "Service URL: http://127.0.0.1:$PORT"

        # Test endpoint
        echo ""
        print_message "${YELLOW}" "Testing endpoint..."
        if curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:$PORT/transcribe -X POST 2>/dev/null | grep -q "400\|405"; then
            print_message "${GREEN}" "✓ Service is responding"
        else
            print_message "${RED}" "✗ Service not responding"
        fi
    elif docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        print_message "${RED}" "Status: STOPPED ✗"
        docker ps -a --filter "name=${CONTAINER_NAME}" --format "table {{.Names}}\t{{.Status}}"
    else
        print_message "${RED}" "Status: NOT CREATED ✗"
        echo "Run: ./whisper_manager.sh build && ./whisper_manager.sh start"
    fi
}

# ============================================================================
# Command: clean
# ============================================================================
clean_service() {
    print_message "${YELLOW}" "Cleaning whisper service..."

    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        print_message "${YELLOW}" "Stopping container..."
        docker stop $CONTAINER_NAME
    fi

    if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        print_message "${YELLOW}" "Removing container..."
        docker rm -f $CONTAINER_NAME
    fi

    if docker images --format '{{.Repository}}:{{.Tag}}' | grep -q "^${IMAGE_NAME}$"; then
        print_message "${YELLOW}" "Removing image..."
        docker rmi -f $IMAGE_NAME
    fi

    print_message "${GREEN}" "✓ Cleanup complete!"
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
    build)
        build_image $1
        ;;
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
    clean)
        clean_service
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
