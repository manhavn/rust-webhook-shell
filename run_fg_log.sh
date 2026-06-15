#!/bin/bash
PORT=${1:-9090}
echo "Starting webhook server in foreground with logs on port $PORT..."
./webhook --port "$PORT" start --foreground
