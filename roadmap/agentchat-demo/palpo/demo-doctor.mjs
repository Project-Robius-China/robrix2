#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const moduleDir = path.dirname(fileURLToPath(import.meta.url));

function fingerprint(value) {
  return createHash('sha256').update(value).digest('hex');
}

async function request(fetchImpl, url, options = {}) {
  try {
    return await fetchImpl(url, options);
  } catch {
    return null;
  }
}

export async function runDoctor({
  deployment,
  homeserver,
  asToken,
  hsToken,
  registrationToken,
  botAccount,
  adminAccount,
  fetchImpl = globalThis.fetch,
}) {
  const checks = [];
  const baseUrl = String(homeserver ?? '').replace(/\/$/, '');

  const versions = await request(fetchImpl, `${baseUrl}/_matrix/client/versions`);
  const homeserverReady = versions?.status === 200;
  checks.push(homeserverReady
    ? { name: 'homeserver', ok: true }
    : { name: 'homeserver', ok: false, cause: 'homeserver unreachable or unhealthy' });

  const configMatches = Boolean(
    deployment?.fingerprints?.asToken
      && deployment?.fingerprints?.hsToken
      && typeof asToken === 'string'
      && typeof hsToken === 'string'
      && typeof registrationToken === 'string'
      && fingerprint(asToken) === deployment.fingerprints.asToken
      && fingerprint(hsToken) === deployment.fingerprints.hsToken
      && fingerprint(registrationToken) === deployment.fingerprints.registrationToken,
  );
  checks.push(configMatches
    ? { name: 'appservice-config', ok: true }
    : { name: 'appservice-config', ok: false, cause: 'appservice registration mismatch' });

  if (!configMatches) {
    checks.push({
      name: 'appservice-credential',
      ok: false,
      cause: 'appservice credential not checked because registration configuration mismatched',
    });
  } else {
    const sender = `@${deployment.senderLocalpart}:${deployment.serverName}`;
    const whoami = await request(
      fetchImpl,
      `${baseUrl}/_matrix/client/v3/account/whoami?user_id=${encodeURIComponent(sender)}`,
      { headers: { authorization: `Bearer ${asToken}` } },
    );
    checks.push(whoami?.status === 200
      ? { name: 'appservice-credential', ok: true }
      : {
          name: 'appservice-credential',
          ok: false,
          cause: 'appservice registration mismatch: credential rejected by homeserver',
        });
  }

  const adminLogin = await request(fetchImpl, `${baseUrl}/_matrix/client/v3/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      type: 'm.login.password',
      identifier: { type: 'm.id.user', user: adminAccount?.username },
      password: adminAccount?.password,
    }),
  });
  if (adminLogin?.status !== 200) {
    checks.push({ name: 'admin-account', ok: false, cause: 'admin account login rejected' });
  } else {
    let adminSession = {};
    try {
      adminSession = await adminLogin.json();
    } catch {
      adminSession = {};
    }
    const adminStatus = adminSession.access_token && adminSession.user_id
      ? await request(
          fetchImpl,
          `${baseUrl}/_synapse/admin/v1/users/${encodeURIComponent(adminSession.user_id)}/admin`,
          { headers: { authorization: `Bearer ${adminSession.access_token}` } },
        )
      : null;
    let statusBody = {};
    try {
      statusBody = adminStatus ? await adminStatus.json() : {};
    } catch {
      statusBody = {};
    }
    checks.push(adminStatus?.status === 200 && statusBody.admin === true
      ? { name: 'admin-account', ok: true }
      : { name: 'admin-account', ok: false, cause: 'configured admin account is not a server admin' });
  }

  const login = await request(fetchImpl, `${baseUrl}/_matrix/client/v3/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      type: 'm.login.password',
      identifier: { type: 'm.id.user', user: botAccount?.username },
      password: botAccount?.password,
    }),
  });
  checks.push(login?.status === 200
    ? { name: 'bot-account', ok: true }
    : { name: 'bot-account', ok: false, cause: 'bot account login rejected' });

  return { ok: checks.every((check) => check.ok), checks };
}

function printText(result) {
  return result.checks
    .map((check) => `${check.ok ? 'OK' : 'FAIL'} ${check.name}${check.cause ? `: ${check.cause}` : ''}`)
    .join('\n');
}

async function main() {
  const deploymentPath = path.resolve(
    process.env.PALPO_DEPLOYMENT_FILE
      || path.join(process.env.PALPO_RUNTIME_DIR || path.join(moduleDir, '.runtime'), 'config', 'deployment.json'),
  );
  const deployment = JSON.parse(await readFile(deploymentPath, 'utf8'));
  const result = await runDoctor({
    deployment,
    homeserver: process.env.PALPO_PUBLIC_URL || deployment.publicUrl,
    asToken: process.env.PALPO_AS_TOKEN,
    hsToken: process.env.PALPO_HS_TOKEN,
    registrationToken: process.env.PALPO_REGISTRATION_TOKEN,
    adminAccount: {
      username: process.env.PALPO_ADMIN_USER,
      password: process.env.PALPO_ADMIN_PASSWORD,
    },
    botAccount: {
      username: process.env.PALPO_BOT_USER,
      password: process.env.PALPO_BOT_PASSWORD,
    },
  });
  process.stdout.write(process.argv.includes('--json')
    ? `${JSON.stringify(result, null, 2)}\n`
    : `${printText(result)}\n`);
  if (!result.ok) process.exitCode = 1;
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  main().catch((error) => {
    process.stderr.write(`[demo-doctor] ${error.message}\n`);
    process.exitCode = 1;
  });
}
