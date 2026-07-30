import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';

import { runDoctor } from '../demo-doctor.mjs';

function fingerprint(value) {
  return createHash('sha256').update(value).digest('hex');
}

function json(status, body) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

const asToken = 'as-token-0123456789abcdef';
const hsToken = 'hs-token-0123456789abcdef';
const registrationToken = 'registration-token-0123456789';
const botAccount = { username: 'agent-bridge', password: 'bot-secret-01234567' };
const adminAccount = { username: 'admin', password: 'admin-secret-012345' };
const deployment = {
  schemaVersion: 1,
  serverName: 'matrix.example.test',
  publicUrl: 'http://matrix.example.test',
  senderLocalpart: '_agentchat_appservice',
  fingerprints: {
    asToken: fingerprint(asToken),
    hsToken: fingerprint(hsToken),
    registrationToken: fingerprint(registrationToken),
  },
};

function fakeHomeserver({
  reachable = true,
  registeredAsToken = asToken,
  botPassword = botAccount.password,
  botExists = true,
  adminIsAdmin = true,
} = {}) {
  return async (url, options = {}) => {
    if (!reachable) throw new TypeError('connect ECONNREFUSED');
    const endpoint = new URL(url).pathname;
    if (endpoint === '/_matrix/client/versions') return json(200, { versions: ['v1.11'] });
    if (endpoint.endsWith('/account/whoami')) {
      return options.headers?.authorization === `Bearer ${registeredAsToken}`
        ? json(200, { user_id: '@agent-bridge:matrix.example.test' })
        : json(401, { errcode: 'M_UNKNOWN_TOKEN' });
    }
    if (endpoint.endsWith('/login')) {
      const body = JSON.parse(options.body || '{}');
      if (botExists && body.identifier?.user === botAccount.username && body.password === botPassword) {
        return json(200, { user_id: '@agent-bridge:matrix.example.test', access_token: 'private-login-token' });
      }
      if (body.identifier?.user === adminAccount.username && body.password === adminAccount.password) {
        return json(200, { user_id: '@admin:matrix.example.test', access_token: 'private-admin-token' });
      }
      return json(403, { errcode: 'M_FORBIDDEN' });
    }
    if (endpoint.startsWith('/_synapse/admin/v1/users/') && endpoint.endsWith('/admin')) {
      return options.headers?.authorization === 'Bearer private-admin-token'
        ? json(200, { admin: adminIsAdmin })
        : json(401, { errcode: 'M_UNKNOWN_TOKEN' });
    }
    return json(404, { errcode: 'M_NOT_FOUND' });
  };
}

function doctor(overrides = {}) {
  return runDoctor({
    deployment,
    homeserver: 'http://matrix.example.test',
    asToken,
    hsToken,
    registrationToken,
    botAccount,
    adminAccount,
    fetchImpl: fakeHomeserver(),
    ...overrides,
  });
}

test('healthy doctor reports homeserver, appservice, and bot account ready', async () => {
  const result = await doctor();

  assert.equal(result.ok, true);
  assert.deepEqual(result.checks.map((check) => [check.name, check.ok]), [
    ['homeserver', true],
    ['appservice-config', true],
    ['appservice-credential', true],
    ['admin-account', true],
    ['bot-account', true],
  ]);
  assert.equal(JSON.stringify(result).includes(asToken), false);
  assert.equal(JSON.stringify(result).includes(hsToken), false);
  assert.equal(JSON.stringify(result).includes(botAccount.password), false);
  assert.equal(JSON.stringify(result).includes(adminAccount.password), false);
  assert.equal(JSON.stringify(result).includes('private-login-token'), false);
  assert.equal(JSON.stringify(result).includes('private-admin-token'), false);
});

test('unreachable homeserver names the failed dependency', async () => {
  const result = await doctor({ fetchImpl: fakeHomeserver({ reachable: false }) });

  assert.equal(result.ok, false);
  assert.equal(result.checks.find((check) => check.name === 'homeserver').ok, false);
  assert.match(result.checks.find((check) => check.name === 'homeserver').cause, /unreachable/i);
});

test('token fingerprint mismatch is explicit', async () => {
  const result = await doctor({ asToken: 'different-as-token-01234567' });
  const check = result.checks.find((item) => item.name === 'appservice-config');

  assert.equal(result.ok, false);
  assert.equal(check.ok, false);
  assert.match(check.cause, /registration mismatch/i);
});

test('homeserver rejection names the appservice credential', async () => {
  const wrongAsToken = 'wrong-as-token-012345678901';
  const matchingManifest = {
    ...deployment,
    fingerprints: { ...deployment.fingerprints, asToken: fingerprint(wrongAsToken) },
  };
  const result = await doctor({
    deployment: matchingManifest,
    asToken: wrongAsToken,
    fetchImpl: fakeHomeserver({ registeredAsToken: asToken }),
  });
  const check = result.checks.find((item) => item.name === 'appservice-credential');

  assert.equal(result.ok, false);
  assert.equal(check.ok, false);
  assert.match(check.cause, /credential rejected/i);
});

test('missing bot account and wrong bot password are surfaced', async (t) => {
  await t.test('missing account', async () => {
    const result = await doctor({ fetchImpl: fakeHomeserver({ botExists: false }) });
    const check = result.checks.find((item) => item.name === 'bot-account');
    assert.equal(check.ok, false);
    assert.match(check.cause, /login rejected/i);
  });
  await t.test('wrong password', async () => {
    const result = await doctor({ botAccount: { ...botAccount, password: 'wrong-secret-0123456' } });
    const check = result.checks.find((item) => item.name === 'bot-account');
    assert.equal(check.ok, false);
    assert.match(check.cause, /login rejected/i);
  });
});

test('admin account must exist and hold server-admin privileges', async (t) => {
  await t.test('wrong admin password', async () => {
    const result = await doctor({ adminAccount: { ...adminAccount, password: 'wrong-admin-secret-000' } });
    const check = result.checks.find((item) => item.name === 'admin-account');
    assert.equal(check.ok, false);
    assert.match(check.cause, /login rejected/i);
  });
  await t.test('normal account is not accepted as admin', async () => {
    const result = await doctor({ fetchImpl: fakeHomeserver({ adminIsAdmin: false }) });
    const check = result.checks.find((item) => item.name === 'admin-account');
    assert.equal(check.ok, false);
    assert.match(check.cause, /not a server admin/i);
  });
});
