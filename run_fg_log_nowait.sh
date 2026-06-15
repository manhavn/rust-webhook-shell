#!/bin/bash
PORT=${1:-9090}
echo "Starting webhook server in foreground with logs (No-wait mode enabled) on port $PORT..."
./webhook --port "$PORT" --no-wait "${@:2}" start --foreground
