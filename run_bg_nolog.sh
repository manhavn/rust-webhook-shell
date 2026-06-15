#!/bin/bash
PORT=${1:-9090}
echo "Starting webhook server in background without logs on port $PORT..."
./webhook --port "$PORT" --no-log "${@:2}" background
