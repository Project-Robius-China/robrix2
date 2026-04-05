#!/usr/bin/env bash
# ============================================================
# Robrix + Palpo + Octos — One-time Setup
# ============================================================
# Run this once before "docker compose up -d".
# It clones the required source repos and prepares .env.
# ============================================================
set -e

cd "$(dirname "$0")"

echo "==> Cloning Palpo (Matrix homeserver)..."
if [ -d repos/palpo ]; then
  echo "    repos/palpo already exists, skipping."
else
  git clone --depth 1 https://github.com/palpo-im/palpo.git repos/palpo
fi

echo "==> Cloning Octos (AI bot)..."
if [ -d repos/octos ]; then
  echo "    repos/octos already exists, skipping."
else
  git clone --depth 1 --recurse-submodules=no https://github.com/octos-org/octos.git repos/octos
fi

if [ ! -f .env ]; then
  cp .env.example .env
  echo "==> Created .env from .env.example."
  echo "    IMPORTANT: Edit .env and set your DEEPSEEK_API_KEY before starting."
else
  echo "==> .env already exists, skipping."
fi

echo ""
echo "Setup complete! Next steps:"
echo "  1. Edit .env and set DEEPSEEK_API_KEY"
echo "  2. docker compose up -d"
