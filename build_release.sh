#!/bin/bash
echo "Building release binary..."
cargo build --release
if [ $? -eq 0 ]; then
  cp target/release/webhook ./webhook
  echo "--------------------------------------------------"
  echo "Success! The binary is ready at: ./webhook"
  echo "Run './webhook --help' to see instructions."
  echo "--------------------------------------------------"
else
  echo "Error: Build failed."
  exit 1
fi
