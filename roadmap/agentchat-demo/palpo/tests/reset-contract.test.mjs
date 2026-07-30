import assert from 'node:assert/strict';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const palpoDir = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const resetScript = path.join(palpoDir, 'demo-reset.sh');

function run(args, env = {}) {
  return spawnSync('bash', [resetScript, ...args], {
    encoding: 'utf8',
    env: { ...process.env, ...env },
  });
}

test('reset script rejects empty, root, traversal, and unconfirmed paths', async () => {
  for (const args of [
    ['--state-root', '', '--confirm'],
    ['--state-root', '/', '--confirm'],
    ['--state-root', path.join(palpoDir, '..', 'outside'), '--confirm'],
    ['--state-root', path.join(palpoDir, '.runtime')],
  ]) {
    const result = run(args);
    assert.notEqual(result.status, 0, `${args.join(' ')} must be rejected`);
    assert.doesNotMatch(result.stderr, /No such file or directory/);
  }
});

test('reset dry-run reports bounded actions and preserves state', async () => {
  const runtime = path.join(palpoDir, '.runtime');
  await mkdir(runtime, { recursive: true });
  const marker = path.join(runtime, `reset-marker-${process.pid}.txt`);
  await writeFile(marker, 'keep', 'utf8');

  try {
    const result = run(['--state-root', runtime, '--confirm', '--dry-run']);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /docker compose/);
    assert.match(result.stdout, /config/);
    assert.match(result.stdout, /state/);
    assert.equal(await readFile(marker, 'utf8'), 'keep');
  } finally {
    await rm(marker, { force: true });
  }
});

test('reset rejects a state root that differs from the Compose runtime root', async () => {
  const runtime = path.join(palpoDir, '.runtime');
  const other = path.join(runtime, 'other');
  await mkdir(other, { recursive: true });
  try {
    const result = run(
      ['--state-root', runtime, '--confirm', '--dry-run'],
      { PALPO_RUNTIME_DIR: other },
    );
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /must match PALPO_RUNTIME_DIR/);
  } finally {
    await rm(other, { recursive: true, force: true });
  }
});

test('reset forwards an isolated Compose project and env file', async () => {
  const runtime = path.join(palpoDir, '.runtime');
  const envFile = path.join(runtime, `e2e-${process.pid}.env`);
  await mkdir(runtime, { recursive: true });
  await writeFile(envFile, 'PALPO_DB_PASSWORD=not-used-in-dry-run\n', 'utf8');
  try {
    const result = run(
      ['--state-root', runtime, '--confirm', '--dry-run'],
      {
        PALPO_RUNTIME_DIR: runtime,
        PALPO_ENV_FILE: envFile,
        PALPO_COMPOSE_PROJECT_NAME: 'agentchat-palpo-e2e',
      },
    );
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /--project-name agentchat-palpo-e2e/);
    assert.match(result.stdout, new RegExp(`--env-file ${envFile.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`));
  } finally {
    await rm(envFile, { force: true });
  }
});

test('reset contract is explicit and idempotent', async () => {
  const script = await readFile(resetScript, 'utf8');

  assert.match(script, /--confirm/);
  assert.match(script, /--dry-run/);
  assert.match(script, /docker compose[\s\S]*down[\s\S]*--remove-orphans/);
  assert.match(script, /rm -rf --/);
  assert.match(script, /\/config/);
  assert.match(script, /\/state/);
});
