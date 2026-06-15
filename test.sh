#!/bin/bash
echo "Hello from test.sh!"
echo "Current Date: $(date)"
echo "Arguments: $@"
echo "Stdin content:"
cat
exit 0
