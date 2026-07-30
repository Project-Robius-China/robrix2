import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const testDir = path.dirname(fileURLToPath(import.meta.url));
const composePath = path.join(testDir, '..', 'compose.yml');

test('compose defines a deterministic Palpo profile', async () => {
  const compose = await readFile(composePath, 'utf8');

  assert.match(compose, /profiles:\s*\["palpo-local"\]/);
  assert.match(compose, /palpo-postgres:[\s\S]*?healthcheck:/);
  assert.match(compose, /depends_on:[\s\S]*?palpo-postgres:[\s\S]*?condition: service_healthy/);
  assert.match(compose, /\$\{PALPO_HOST_PORT:-8128\}:8008/);
  assert.match(compose, /\$\{PALPO_RUNTIME_DIR:-\.\/\.runtime\}\/config\/palpo\.toml:\/var\/palpo\/palpo\.toml:ro/);
  assert.match(compose, /\$\{PALPO_RUNTIME_DIR:-\.\/\.runtime\}\/config\/appservice-agentchat\.yaml:\/var\/palpo\/appservices\/appservice-agentchat\.yaml:ro/);
  assert.match(compose, /\$\{PALPO_RUNTIME_DIR:-\.\/\.runtime\}\/state/);
  assert.doesNotMatch(compose, /PALPO_(?:RENDERED|STATE)_DIR/);
  assert.match(compose, /PALPO_CONFIG: \/var\/palpo\/palpo\.toml/);
});

test('compose requires operator-provided database credentials', async () => {
  const compose = await readFile(composePath, 'utf8');

  assert.match(compose, /PALPO_DB_PASSWORD:\?PALPO_DB_PASSWORD is required/);
  assert.equal(/(?:password|token)[^\n]*(?:change-me|dev-token|palpo_dev_password)/i.test(compose), false);
  assert.equal(/(?:as_token|hs_token)\s*:/i.test(compose), false);
});
