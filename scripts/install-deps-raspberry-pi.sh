#!/usr/bin/env bash
set -euo pipefail
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config curl \
  chromium \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-plugins-good gstreamer1.0-tools \
  libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 \
  libv4l-dev v4l-utils
# Mount USB SSD at /var/lib/instant-replay (see docs/PI_DEPLOYMENT.md)
sudo mkdir -p /var/lib/instant-replay/buffer
echo "Pi dependencies installed."
