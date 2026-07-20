import assert from 'node:assert/strict';
import { randomBytes, createHash } from 'node:crypto';
import { chmod, mkdir, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { bootstrapAccounts, createComposeAdminPromoter } from '../bootstrap-accounts.mjs';
import { runDoctor } from '../demo-doctor.mjs';
import { renderConfig } from '../palpo-config.mjs';

const palpoDir = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const realEnabled = process.env.PALPO_REAL_E2E === '1';
const realOptions = {
  skip: realEnabled ? false : 'set PALPO_REAL_E2E=1 to run Docker/Palpo acceptance',
  timeout: 15 * 60 * 1000,
};

function secret(prefix) {
  return `${prefix}-${randomBytes(24).toString('hex')}`;
}

function fingerprint(value) {
  return createHash('sha256').update(value).digest('hex');
}

function redact(text, values) {
  return values.reduce((output, value) => output.replaceAll(value, '<redacted>'), String(text ?? ''));
}

function run(command, args, { cwd = palpoDir, env = process.env, secrets = [] } = {}) {
  const result = spawnSync(command, args, { cwd, env, encoding: 'utf8' });
  if (result.status !== 0) {
    const detail = redact(`${result.stdout || ''}\n${result.stderr || ''}`, secrets).trim().slice(-4000);
    throw new Error(`${command} failed with status ${result.status}${detail ? `\n${detail}` : ''}`);
  }
  return result.stdout;
}

async function login(homeserver, account) {
  const response = await fetch(`${homeserver}/_matrix/client/v3/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      type: 'm.login.password',
      identifier: { type: 'm.id.user', user: account.username },
      password: account.password,
    }),
  });
  assert.equal(response.status, 200, `login failed for ${account.username}`);
  return response.json();
}

async function withRealStack(fn) {
  const runId = `${process.pid}-${randomBytes(4).toString('hex')}`;
  const projectName = `agentchat-palpo-e2e-${runId}`;
  const runtimeDir = path.join(palpoDir, '.runtime', `e2e-${runId}`);
  const envFile = path.join(runtimeDir, 'operator.env');
  const port = Number(process.env.PALPO_E2E_HOST_PORT || 18128);
  const secrets = {
    db: secret('db'),
    as: secret('as'),
    hs: secret('hs'),
    registration: secret('registration'),
    admin: secret('admin'),
    bot: secret('bot'),
  };
  const palpoEnv = {
    COMPOSE_PROJECT_NAME: projectName,
    PALPO_SERVER_NAME: `127.0.0.1:${port}`,
    PALPO_PUBLIC_URL: `http://127.0.0.1:${port}`,
    PALPO_HOST_PORT: String(port),
    PALPO_DB_HOST: 'palpo-postgres',
    PALPO_DB_PORT: '5432',
    PALPO_DB_NAME: 'palpo',
    PALPO_DB_USER: 'palpo',
    PALPO_DB_PASSWORD: secrets.db,
    PALPO_APPSERVICE_URL: 'http://host.docker.internal:8091',
    PALPO_AS_TOKEN: secrets.as,
    PALPO_HS_TOKEN: secrets.hs,
    PALPO_REGISTRATION_TOKEN: secrets.registration,
    PALPO_SENDER_LOCALPART: '_agentchat_appservice',
    PALPO_ADMIN_USER: 'admin',
    PALPO_ADMIN_PASSWORD: secrets.admin,
    PALPO_BOT_USER: 'agent-bridge',
    PALPO_BOT_PASSWORD: secrets.bot,
    PALPO_RUNTIME_DIR: runtimeDir,
  };
  const env = { ...process.env, ...palpoEnv };
  const secretValues = Object.values(secrets);
  const composeBase = [
    'compose', '--project-name', projectName,
    '--env-file', envFile,
    '-f', path.join(palpoDir, 'compose.yml'),
    '--profile', 'palpo-local',
  ];
  const accounts = [
    { username: env.PALPO_ADMIN_USER, password: env.PALPO_ADMIN_PASSWORD, role: 'admin' },
    { username: env.PALPO_BOT_USER, password: env.PALPO_BOT_PASSWORD },
  ];

  await mkdir(runtimeDir, { recursive: true, mode: 0o700 });
  const envText = Object.entries(palpoEnv)
    .map(([name, value]) => `${name}=${value}`)
    .join('\n');
  await writeFile(envFile, `${envText}\n`, { mode: 0o600 });
  await chmod(envFile, 0o600);

  const start = async () => {
    await renderConfig({ env, outputDir: path.join(runtimeDir, 'config') });
    run('docker', [...composeBase, 'up', '-d', '--build', '--wait'], { env, secrets: secretValues });
  };
  const bootstrap = () => bootstrapAccounts({
    homeserver: env.PALPO_PUBLIC_URL,
    accounts,
    registrationToken: env.PALPO_REGISTRATION_TOKEN,
    promoteAdmin: createComposeAdminPromoter({
      demoDir: palpoDir,
      serverName: env.PALPO_SERVER_NAME,
      envFile,
      projectName,
    }),
  });
  const doctor = (overrides = {}) => runDoctor({
    deployment: overrides.deployment,
    homeserver: env.PALPO_PUBLIC_URL,
    asToken: overrides.asToken || env.PALPO_AS_TOKEN,
    hsToken: env.PALPO_HS_TOKEN,
    registrationToken: env.PALPO_REGISTRATION_TOKEN,
    adminAccount: accounts[0],
    botAccount: accounts[1],
  });
  const psql = (sql) => run('docker', [
    ...composeBase,
    'exec', '-T', 'palpo-postgres',
    'psql', '-U', 'palpo', '-d', 'palpo', '-tA', '-v', 'ON_ERROR_STOP=1', '-c', sql,
  ], { env, secrets: secretValues }).trim();

  try {
    await start();
    const rendered = await renderConfig({ env, outputDir: path.join(runtimeDir, 'config') });
    const bootstrapResult = await bootstrap();
    assert.equal(bootstrapResult.ok, true, JSON.stringify(bootstrapResult));
    await fn({
      env,
      envFile,
      runtimeDir,
      projectName,
      accounts,
      deployment: rendered.deployment,
      composeBase,
      start,
      bootstrap,
      doctor,
      psql,
      secretValues,
    });
  } finally {
    try {
      run('docker', [...composeBase, 'down', '-v', '--remove-orphans'], { env, secrets: secretValues });
    } finally {
      await rm(runtimeDir, { recursive: true, force: true });
    }
  }
}

test('test_palpo_fresh_start_healthy', realOptions, async () => {
  await withRealStack(async ({ deployment, doctor }) => {
    const result = await doctor({ deployment });
    assert.equal(result.ok, true, JSON.stringify(result));
  });
});

test('test_bootstrap_idempotent', realOptions, async () => {
  await withRealStack(async ({ env, bootstrap, psql }) => {
    const ids = [`@admin:${env.PALPO_SERVER_NAME}`, `@agent-bridge:${env.PALPO_SERVER_NAME}`];
    const query = `SELECT count(*) FROM users WHERE id IN ('${ids.join("','")}');`;
    const before = psql(query);
    const second = await bootstrap();
    const after = psql(query);
    assert.equal(second.ok, true, JSON.stringify(second));
    assert.deepEqual(second.results.map((result) => result.status), ['existing', 'existing']);
    assert.equal(after, before);
  });
});

test('test_doctor_reports_appservice_mismatch', realOptions, async () => {
  await withRealStack(async ({ env, runtimeDir, deployment, doctor }) => {
    const before = await doctor({ deployment });
    assert.equal(before.ok, true, JSON.stringify(before));
    const wrongToken = secret('wrong-as');
    const rerendered = await renderConfig({
      env: { ...env, PALPO_AS_TOKEN: wrongToken },
      outputDir: path.join(runtimeDir, 'config'),
    });
    const result = await doctor({ deployment: rerendered.deployment, asToken: wrongToken });
    const check = result.checks.find((item) => item.name === 'appservice-credential');
    assert.equal(check.ok, false);
    assert.match(check.cause, /registration mismatch/i);
  });
});

test('test_wrong_as_token_rejected', realOptions, async () => {
  await withRealStack(async ({ deployment, doctor }) => {
    const wrongToken = secret('wrong-as');
    const matchingManifest = {
      ...deployment,
      fingerprints: { ...deployment.fingerprints, asToken: fingerprint(wrongToken) },
    };
    const result = await doctor({ deployment: matchingManifest, asToken: wrongToken });
    assert.equal(result.ok, false);
    assert.equal(result.checks.find((item) => item.name === 'appservice-credential').ok, false);
  });
});

test('test_reset_restores_clean_state', realOptions, async () => {
  await withRealStack(async (context) => {
    const session = await login(context.env.PALPO_PUBLIC_URL, context.accounts[0]);
    const created = await fetch(`${context.env.PALPO_PUBLIC_URL}/_matrix/client/v3/createRoom`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${session.access_token}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify({ name: `fsf0-reset-${context.projectName}` }),
    });
    assert.equal(created.status, 200);
    const { room_id: roomId } = await created.json();

    run('bash', [path.join(palpoDir, 'demo-reset.sh'),
      '--state-root', context.runtimeDir, '--confirm'], {
      env: {
        ...context.env,
        PALPO_ENV_FILE: context.envFile,
        PALPO_COMPOSE_PROJECT_NAME: context.projectName,
      },
      secrets: context.secretValues,
    });
    await context.start();
    const bootstrapResult = await context.bootstrap();
    assert.equal(bootstrapResult.ok, true, JSON.stringify(bootstrapResult));
    const rendered = await renderConfig({
      env: context.env,
      outputDir: path.join(context.runtimeDir, 'config'),
    });
    const health = await context.doctor({ deployment: rendered.deployment });
    assert.equal(health.ok, true, JSON.stringify(health));

    const newSession = await login(context.env.PALPO_PUBLIC_URL, context.accounts[0]);
    const oldRoom = await fetch(
      `${context.env.PALPO_PUBLIC_URL}/_matrix/client/v3/rooms/${encodeURIComponent(roomId)}/state`,
      { headers: { authorization: `Bearer ${newSession.access_token}` } },
    );
    assert.notEqual(oldRoom.status, 200, 'room from the pre-reset database still exists');
  });
});
