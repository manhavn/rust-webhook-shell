#!/bin/bash
cd "$(dirname "$0")"

echo "Hello from test.sh!"
echo "Current Date: $(date)"
echo "Arguments: $@"
echo "Stdin content:"
cat
exit 0
