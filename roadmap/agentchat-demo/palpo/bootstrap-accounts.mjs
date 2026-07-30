#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const moduleDir = path.dirname(fileURLToPath(import.meta.url));

function validateHomeserver(homeserver) {
  let parsed;
  try {
    parsed = new URL(homeserver);
  } catch {
    throw new Error('homeserver must be an absolute HTTP(S) URL');
  }
  if (!['http:', 'https:'].includes(parsed.protocol) || !parsed.hostname) {
    throw new Error('homeserver must be an absolute HTTP(S) URL');
  }
  return homeserver.replace(/\/$/, '');
}

function validateAccounts(accounts) {
  if (!Array.isArray(accounts) || accounts.length === 0) {
    throw new Error('accounts must contain at least one account');
  }
  return accounts.map((account) => {
    const username = String(account.username ?? '').trim();
    const password = String(account.password ?? '');
    if (!/^[a-z0-9._=-]+$/i.test(username)) throw new Error(`invalid account username: ${username || '(empty)'}`);
    if (password.length < 12 || /[<>]/.test(password)) throw new Error(`invalid password for ${username}`);
    const role = account.role === 'admin' ? 'admin' : undefined;
    return { username, password, role };
  });
}

async function requestJson(fetchImpl, url, body) {
  const response = await fetchImpl(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  let payload = {};
  try {
    payload = await response.json();
  } catch {
    payload = {};
  }
  return { status: response.status, payload };
}

async function login(fetchImpl, homeserver, account) {
  const response = await requestJson(fetchImpl, `${homeserver}/_matrix/client/v3/login`, {
    type: 'm.login.password',
    identifier: { type: 'm.id.user', user: account.username },
    password: account.password,
  });
  if (response.status === 200 && response.payload.user_id) {
    return { kind: 'ready', userId: response.payload.user_id };
  }
  if (response.status === 401 || response.status === 403) return { kind: 'not-ready' };
  throw new Error(`login endpoint returned HTTP ${response.status}`);
}

async function loginAfterConflict(fetchImpl, homeserver, account) {
  const retry = await login(fetchImpl, homeserver, account);
  if (retry.kind === 'ready') {
    return {
      username: account.username,
      userId: retry.userId,
      ok: true,
      status: 'existing-after-race',
    };
  }
  return {
    username: account.username,
    ok: false,
    status: 'password-mismatch',
    error: 'account exists but its password does not match the configured credential',
  };
}

async function bootstrapAccount(fetchImpl, homeserver, account, registrationToken) {
  const initialLogin = await login(fetchImpl, homeserver, account);
  if (initialLogin.kind === 'ready') {
    return {
      username: account.username,
      userId: initialLogin.userId,
      ok: true,
      status: 'existing',
    };
  }

  const probe = await requestJson(fetchImpl, `${homeserver}/_matrix/client/v3/register`, {
    username: account.username,
    password: account.password,
  });
  if (probe.status === 200 && probe.payload.user_id) {
    return { username: account.username, userId: probe.payload.user_id, ok: true, status: 'created' };
  }
  if (probe.payload.errcode === 'M_USER_IN_USE') {
    return loginAfterConflict(fetchImpl, homeserver, account);
  }
  const supportsRegistrationToken = probe.payload.flows?.some((flow) =>
    flow.stages?.includes('m.login.registration_token'));
  if (!probe.payload.session || !supportsRegistrationToken) {
    throw new Error(`registration endpoint did not offer m.login.registration_token (HTTP ${probe.status})`);
  }

  const registration = await requestJson(fetchImpl, `${homeserver}/_matrix/client/v3/register`, {
    username: account.username,
    password: account.password,
    auth: {
      type: 'm.login.registration_token',
      token: registrationToken,
      session: probe.payload.session,
    },
  });
  if (registration.status === 200 && registration.payload.user_id) {
    return {
      username: account.username,
      userId: registration.payload.user_id,
      ok: true,
      status: 'created',
    };
  }
  if (registration.payload.errcode === 'M_USER_IN_USE') {
    return loginAfterConflict(fetchImpl, homeserver, account);
  }
  throw new Error(`registration failed with HTTP ${registration.status}`);
}

export async function bootstrapAccounts({
  homeserver,
  accounts,
  registrationToken,
  promoteAdmin,
  fetchImpl = globalThis.fetch,
}) {
  const baseUrl = validateHomeserver(homeserver);
  const validatedAccounts = validateAccounts(accounts);
  if (typeof registrationToken !== 'string' || registrationToken.length < 16 || /[<>]/.test(registrationToken)) {
    throw new Error('registrationToken must be a non-placeholder secret of at least 16 characters');
  }
  if (typeof fetchImpl !== 'function') throw new Error('fetchImpl must be a function');

  const results = [];
  for (const account of validatedAccounts) {
    try {
      const result = await bootstrapAccount(fetchImpl, baseUrl, account, registrationToken);
      if (result.ok && account.role === 'admin') {
        if (typeof promoteAdmin !== 'function') throw new Error('admin promotion is not configured');
        try {
          await promoteAdmin(account.username);
          result.admin = true;
        } catch {
          result.ok = false;
          result.status = 'admin-promotion-failed';
          result.error = 'admin promotion failed';
        }
      }
      results.push(result);
    } catch (error) {
      results.push({
        username: account.username,
        ok: false,
        status: 'failed',
        error: error instanceof Error ? error.message : 'unknown bootstrap failure',
      });
    }
  }
  return { ok: results.every((result) => result.ok), results };
}

function defaultRunCommand(command, args, options) {
  return spawnSync(command, args, { ...options, encoding: 'utf8' });
}

export function createComposeAdminPromoter({
  demoDir,
  serverName,
  dbUser = 'palpo',
  dbName = 'palpo',
  envFile = path.join(demoDir, '.env'),
  projectName,
  runCommand = defaultRunCommand,
}) {
  if (!path.isAbsolute(demoDir)) throw new Error('demoDir must be absolute');
  if (!/^[a-z0-9.-]+(?::[0-9]{1,5})?$/i.test(serverName)) throw new Error('invalid serverName');
  if (!/^[a-z0-9._=-]+$/i.test(dbUser) || !/^[a-z0-9._=-]+$/i.test(dbName)) {
    throw new Error('invalid database identifier');
  }

  return async (username) => {
    if (!/^[a-z0-9._=-]+$/.test(username)) throw new Error('invalid admin username');
    const userId = `@${username}:${serverName}`;
    const sql = `WITH promoted AS (UPDATE users SET is_admin = TRUE WHERE id = '${userId}' RETURNING is_admin) SELECT is_admin FROM promoted;`;
    const composeArgs = ['compose'];
    if (projectName) composeArgs.push('--project-name', projectName);
    composeArgs.push(
      '--env-file', envFile,
      '-f', path.join(demoDir, 'compose.yml'),
      '--profile', 'palpo-local',
      'exec', '-T', 'palpo-postgres',
      'psql', '-U', dbUser, '-d', dbName,
      '-tA', '-v', 'ON_ERROR_STOP=1', '-c', sql,
    );
    const result = await runCommand('docker', composeArgs, { cwd: demoDir });
    if (result.status !== 0) throw new Error('admin promotion command failed');
    if (!String(result.stdout ?? '').split(/\s+/).includes('t')) {
      throw new Error(`admin account ${userId} was not found`);
    }
  };
}

async function main() {
  const accounts = [
    { username: process.env.PALPO_ADMIN_USER, password: process.env.PALPO_ADMIN_PASSWORD, role: 'admin' },
    { username: process.env.PALPO_BOT_USER, password: process.env.PALPO_BOT_PASSWORD },
  ];
  const result = await bootstrapAccounts({
    homeserver: process.env.PALPO_PUBLIC_URL || 'http://127.0.0.1:8128',
    accounts,
    registrationToken: process.env.PALPO_REGISTRATION_TOKEN,
    promoteAdmin: createComposeAdminPromoter({
      demoDir: moduleDir,
      serverName: process.env.PALPO_SERVER_NAME,
      dbUser: process.env.PALPO_DB_USER || 'palpo',
      dbName: process.env.PALPO_DB_NAME || 'palpo',
    }),
  });
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  if (!result.ok) process.exitCode = 1;
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  main().catch((error) => {
    process.stderr.write(`[bootstrap-accounts] ${error.message}\n`);
    process.exitCode = 1;
  });
}
