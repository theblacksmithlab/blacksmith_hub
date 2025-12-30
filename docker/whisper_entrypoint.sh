#!/bin/bash
set -e

echo "Starting Whisper Transcription Service..."

# Check if whisper-cli is available
if ! command -v whisper-cli &> /dev/null; then
    echo "Error: whisper-cli not found in PATH"
    exit 1
fi

# Check if whisper model exists
if [ ! -f "$WHISPER_MODEL_PATH" ]; then
    echo "Error: Whisper model not found at $WHISPER_MODEL_PATH"
    exit 1
fi

echo "Using whisper model: $WHISPER_MODEL_PATH"

# Check if config file exists
if [ ! -f "/app/config.yaml" ]; then
    echo "Error: config.yaml not found"
    exit 1
fi

echo "Config file found at /app/config.yaml"

# Start the whisper service
exec /app/whisper
