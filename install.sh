#!/bin/bash

# Determine OS architecture
ARCH=$(uname -m)
OS=$(uname -s)

if [ "$OS" != "Linux" ]; then
    echo "This script only supports Linux"
    exit 1
fi

# Set version
VERSION="0.24.2"

# Set download URL based on architecture
if [ "$OS" != "Linux" ]; then
    echo "This script only supports Linux"
    exit 1
fi

if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
    URL="https://github.com/parca-dev/parca/releases/download/v${VERSION}/parca_${VERSION}_Linux_arm64.tar.gz"
elif [ "$ARCH" = "x86_64" ]; then
    URL="https://github.com/parca-dev/parca/releases/download/v${VERSION}/parca_${VERSION}_Linux_x86_64.tar.gz"
else
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

# Create temp directory
TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

# Download and extract
echo "Downloading Parca v${VERSION}..."
curl -L "$URL" -o parca.tar.gz
tar xzf parca.tar.gz

# Install binary
sudo mv parca /usr/local/bin/
sudo chmod +x /usr/local/bin/parca

# Cleanup
cd - > /dev/null
rm -rf "$TMP_DIR"

echo "Parca v${VERSION} installed successfully!"
