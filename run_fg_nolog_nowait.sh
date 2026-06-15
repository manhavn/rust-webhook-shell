#!/bin/bash
PORT=${1:-9090}
echo "Starting webhook server in foreground without logs (No-wait mode enabled) on port $PORT..."
./webhook --port "$PORT" --no-log --no-wait "${@:2}" start --foreground
