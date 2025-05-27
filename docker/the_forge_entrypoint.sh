#!/bin/bash
set -e

if [ -z "$APP_NAME" ]; then
    echo "Error: APP_NAME environment variable is required"
    exit 1
fi

if [ ! -f "/app/config.yaml" ]; then
    echo "Error: Configuration file not found at /app/config.yaml"
    exit 1
fi

echo "Starting ${APP_NAME} service..."

exec /app/the_forge