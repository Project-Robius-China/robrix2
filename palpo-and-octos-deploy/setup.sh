#!/usr/bin/env bash
# ============================================================
# Robrix + Palpo + Octos — One-time Setup
# ============================================================
# Run this once before "docker compose up -d".
# It initializes git submodules and prepares .env.
# ============================================================
set -e

cd "$(dirname "$0")"

echo "==> Initializing git submodules (Palpo + Octos)..."
git submodule update --init --depth 1 repos/palpo repos/octos

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
