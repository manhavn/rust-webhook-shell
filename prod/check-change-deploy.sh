#!/bin/bash
cd "$(dirname "$0")"
cd ..

# Exit immediately if a command exits with a non-zero status
# (If git pull fails, the script will exit and not trigger deployment)
set -e

# Target branch for deployment
TARGET_BRANCH="deploy-prod"

# Ensure we are in a git repository
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Error: Not a git repository."
  exit 1
fi

# Get the current branch name
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

# If we are not on the target branch, check it out
if [ "$CURRENT_BRANCH" != "$TARGET_BRANCH" ]; then
  echo "Current branch is '$CURRENT_BRANCH'. Switching to '$TARGET_BRANCH'..."
  # Use -f to discard any local changes preventing the checkout
  git checkout -f "$TARGET_BRANCH"
fi

# Fetch the latest changes from origin
echo "Fetching latest changes from origin for $TARGET_BRANCH..."
git fetch origin "$TARGET_BRANCH"

# Get the local and remote commit hashes
LOCAL_COMMIT=$(git rev-parse HEAD)
REMOTE_COMMIT=$(git rev-parse origin/"$TARGET_BRANCH")

# Compare commit hashes
if [ "$LOCAL_COMMIT" != "$REMOTE_COMMIT" ]; then
  echo "============================================="
  echo "New changes detected. Resetting local branch to match origin/$TARGET_BRANCH..."
  echo "============================================="
  
  # Reset hard to origin/$TARGET_BRANCH to discard local commits/changes and resolve any conflicts
  git reset --hard origin/"$TARGET_BRANCH"
  
  echo "============================================="
  echo "start deploy"
  echo "============================================="
  ./prod/node-prod.sh
else
  echo "============================================="
  echo "no changes"
  echo "============================================="
fi

echo "============================================="
echo "Deployment finished. Switching back to main..."
echo "============================================="
git checkout main

