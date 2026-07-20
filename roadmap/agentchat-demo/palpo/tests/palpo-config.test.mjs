import assert from 'node:assert/strict';
import { mkdtemp, readFile, stat } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { renderConfig } from '../palpo-config.mjs';

const validEnv = {
  PALPO_SERVER_NAME: 'matrix.example.test',
  PALPO_PUBLIC_URL: 'https://matrix.example.test:8448',
  PALPO_HOST_PORT: '8448',
  PALPO_DB_HOST: 'palpo-postgres',
  PALPO_DB_PORT: '5432',
  PALPO_DB_NAME: 'palpo',
  PALPO_DB_USER: 'palpo',
  PALPO_DB_PASSWORD: 'db-secret-0123456789',
  PALPO_APPSERVICE_URL: 'http://host.docker.internal:8091',
  PALPO_AS_TOKEN: 'as-token-0123456789abcdef',
  PALPO_HS_TOKEN: 'hs-token-0123456789abcdef',
  PALPO_REGISTRATION_TOKEN: 'registration-token-0123456789',
  PALPO_SENDER_LOCALPART: '_agentchat_appservice',
};

async function outputDir() {
  return mkdtemp(path.join(os.tmpdir(), 'palpo-config-test-'));
}

test('renderConfig rejects missing and placeholder secrets', async () => {
  for (const [key, value] of [
    ['PALPO_AS_TOKEN', ''],
    ['PALPO_HS_TOKEN', '<generate-me>'],
    ['PALPO_DB_PASSWORD', 'change-me'],
    ['PALPO_REGISTRATION_TOKEN', '<generate-me>'],
  ]) {
    await assert.rejects(
      renderConfig({ env: { ...validEnv, [key]: value }, outputDir: await outputDir() }),
      new RegExp(key),
    );
  }
});

test('renderConfig rejects invalid network and identity values', async () => {
  for (const [key, value] of [
    ['PALPO_SERVER_NAME', 'https://matrix.example.test'],
    ['PALPO_PUBLIC_URL', 'ftp://matrix.example.test'],
    ['PALPO_HOST_PORT', '70000'],
    ['PALPO_DB_PORT', 'not-a-port'],
    ['PALPO_SENDER_LOCALPART', 'Agent Bridge'],
  ]) {
    await assert.rejects(
      renderConfig({ env: { ...validEnv, [key]: value }, outputDir: await outputDir() }),
      new RegExp(key),
    );
  }
});

test('renderConfig rejects TOML and YAML control characters', async () => {
  for (const [key, value] of [
    ['PALPO_SERVER_NAME', 'matrix.example.test\nadmin = true'],
    ['PALPO_APPSERVICE_URL', 'http://bridge:8091\nadmin: true'],
    ['PALPO_DB_NAME', 'palpo\"; injected=true'],
    ['PALPO_AS_TOKEN', 'safe-prefix\nunsafe'],
  ]) {
    await assert.rejects(
      renderConfig({ env: { ...validEnv, [key]: value }, outputDir: await outputDir() }),
      new RegExp(key),
    );
  }
});

test('renderConfig writes deterministic private files and redacts deployment metadata', async () => {
  const dir = await outputDir();
  const result = await renderConfig({ env: validEnv, outputDir: dir });

  assert.deepEqual(Object.keys(result.files).sort(), [
    'appservice',
    'deployment',
    'palpo',
  ]);

  const palpo = await readFile(path.join(dir, 'palpo.toml'), 'utf8');
  const appservice = await readFile(path.join(dir, 'appservice-agentchat.yaml'), 'utf8');
  const deploymentText = await readFile(path.join(dir, 'deployment.json'), 'utf8');
  const deployment = JSON.parse(deploymentText);

  assert.match(palpo, /server_name = "matrix\.example\.test"/);
  assert.match(palpo, /postgres:\/\/palpo:db-secret-0123456789@palpo-postgres:5432\/palpo/);
  assert.match(palpo, /registration_token = "registration-token-0123456789"/);
  assert.match(palpo, /rc_registration = \{ per_second = 1\.0, burst = 10 \}/);
  assert.doesNotMatch(palpo, /open_registration_server_prone_to_abuse = true/);
  assert.match(appservice, /as_token: "as-token-0123456789abcdef"/);
  assert.match(appservice, /hs_token: "hs-token-0123456789abcdef"/);
  assert.match(appservice, /sender_localpart: "_agentchat_appservice"/);
  assert.doesNotMatch(appservice, /@agent-bridge:/);
  assert.equal(deployment.serverName, 'matrix.example.test');
  assert.match(deployment.fingerprints.asToken, /^[a-f0-9]{64}$/);
  assert.match(deployment.fingerprints.hsToken, /^[a-f0-9]{64}$/);
  assert.match(deployment.fingerprints.registrationToken, /^[a-f0-9]{64}$/);

  for (const secret of [
    validEnv.PALPO_DB_PASSWORD,
    validEnv.PALPO_AS_TOKEN,
    validEnv.PALPO_HS_TOKEN,
    validEnv.PALPO_REGISTRATION_TOKEN,
  ]) {
    assert.equal(deploymentText.includes(secret), false);
  }

  for (const filename of ['palpo.toml', 'appservice-agentchat.yaml', 'deployment.json']) {
    assert.equal((await stat(path.join(dir, filename))).mode & 0o777, 0o600);
  }
});
