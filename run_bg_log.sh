#!/bin/bash
PORT=${1:-9090}
echo "Starting webhook server in background with logs on port $PORT..."
./webhook --port "$PORT" background
