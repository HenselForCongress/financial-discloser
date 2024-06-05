#!/bin/bash

# Ensure the script stops on any error
set -e

# Variables
MAX_SIZE=$((45 * 1024 * 1024))  # 45MB
CUMULATIVE_SIZE=0
FILES_TO_COMMIT=()
COMMIT_MESSAGE="Adding new files up to 45MB"

# List new/modified files while respecting .gitignore
NEW_FILES=$(git status --porcelain=v1 | grep '^??' | awk '{print $2}')

# Function to get the size of a file
get_file_size() {
    stat -f%z "$1"
}

# Iterating over new files to add them until we reach the size limit
for FILE in $NEW_FILES; do
    FILE_SIZE=$(get_file_size "$FILE")

    if (( CUMULATIVE_SIZE + FILE_SIZE <= MAX_SIZE )); then
        FILES_TO_COMMIT+=("$FILE")
        CUMULATIVE_SIZE=$((CUMULATIVE_SIZE + FILE_SIZE))
    else
        break
    fi
done

# If there are files to commit, add them, sign the commit, and push
if [ ${#FILES_TO_COMMIT[@]} -gt 0 ]; then
    git add "${FILES_TO_COMMIT[@]}"
    git commit -S -m "$COMMIT_MESSAGE"
    git push origin develop
else
    echo "No new files to commit or total file size exceeds 45MB."
fi
