#!/bin/bash
PORT=${1:-9090}
echo "Starting webhook server in foreground without logs on port $PORT..."
./webhook --port "$PORT" --no-log "${@:2}" start --foreground
