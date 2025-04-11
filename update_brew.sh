#!/bin/bash

set -e

PROJECT_NAME="cpp-test"
FORMULA_PATH="/Users/zimengx/code/homebrew-tools/Formula/${PROJECT_NAME}.rb"
HOMEBREW_REPO_PATH="/Users/zimengx/code/homebrew-tools/Formula"
ARCHIVE_NAME="${PROJECT_NAME}-macos.tar.gz"
EXECUTABLE_PATH="target/release/${PROJECT_NAME}"

echo "Starting release process for ${PROJECT_NAME}..."

echo "Building release version..."
cargo build --release

if [ ! -f "${EXECUTABLE_PATH}" ]; then
    echo "Error: Build failed or executable not found at '${EXECUTABLE_PATH}'" >&2
    exit 1
fi
echo "Build successful."

echo "Creating tarball '${ARCHIVE_NAME}'..."
(cd target/release && tar czf "../../${ARCHIVE_NAME}" "${PROJECT_NAME}")

if [ ! -f "${ARCHIVE_NAME}" ]; then
    echo "Error: Failed to create tarball '${ARCHIVE_NAME}'" >&2
    exit 1
fi
echo "Tarball created."

echo "Calculating SHA256 sum..."
NEW_SHA256=$(shasum -a 256 "${ARCHIVE_NAME}" | awk '{ print $1 }')
echo "New SHA256: ${NEW_SHA256}"

echo "Updating Homebrew formula at '${FORMULA_PATH}'..."

if [ ! -f "${FORMULA_PATH}" ]; then
    echo "Error: Formula file not found at '${FORMULA_PATH}'" >&2
    exit 1
fi

sed -i.bak "s~^  sha256 \".*\"$~  sha256 \"${NEW_SHA256}\"~" "${FORMULA_PATH}"

if [ ! -f "${FORMULA_PATH}.bak" ]; then
     echo "Error: Failed to update sha256 in formula file." >&2
     exit 1
fi

rm "${FORMULA_PATH}.bak"
echo "Formula updated."

echo "Committing and pushing changes to Homebrew tap repository at '${HOMEBREW_REPO_PATH}'..."
(
  cd "${HOMEBREW_REPO_PATH}"

  if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
      echo "Error: '${HOMEBREW_REPO_PATH}' is not a git repository." >&2
      exit 1
  fi

  git add "${FORMULA_PATH##*/}"

  git commit -m "update ${PROJECT_NAME} version"

  git push
)
echo "Changes pushed to Homebrew repository."

echo "Release process completed successfully!"

exit 0