#!/bin/bash

SERVICE_NAME="ai_forge"

if [[ "$1" == "restart" ]]; then
    echo "Simple restart for container: $SERVICE_NAME..."
    docker-compose restart $SERVICE_NAME
    echo "Container restarted successfully!"

elif [[ "$1" == "redeploy" ]]; then
    echo "Full redeployment for container: $SERVICE_NAME..."
    docker-compose stop $SERVICE_NAME
    docker-compose rm -f $SERVICE_NAME

    IMAGE_ID=$(docker images -q blacksmith_lab_${SERVICE_NAME})
        if [[ -n "$IMAGE_ID" ]]; then
            echo "Removing old image: $IMAGE_ID"
            docker rmi -f "$IMAGE_ID"
        fi

    docker-compose build $SERVICE_NAME
    docker-compose up -d $SERVICE_NAME
    echo "Container redeployed successfully!"

else
    echo "ACHTUNG! Usage: ./ai_forge_manager.sh [restart|redeploy]"
fi