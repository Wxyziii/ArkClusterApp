#!/usr/bin/env bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

if command -v steamcmd >/dev/null 2>&1; then
  echo "SteamCMD already installed: $(command -v steamcmd)"
  exit 0
fi

echo steam steam/question select "I AGREE" | sudo debconf-set-selections
echo steam steam/license note '' | sudo debconf-set-selections

sudo dpkg --add-architecture i386
sudo add-apt-repository -y multiverse
sudo apt-get update -y
sudo apt-get install -y steamcmd lib32gcc-s1 ca-certificates

steamcmd +quit
