#!/bin/bash
set -e

if [ -z "$APP_NAME" ]; then
    echo "Error: APP_NAME environment variable is required"
    exit 1
fi

exec /app/app