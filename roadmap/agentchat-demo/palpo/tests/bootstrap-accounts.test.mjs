import assert from 'node:assert/strict';
import test from 'node:test';

import { bootstrapAccounts, createComposeAdminPromoter } from '../bootstrap-accounts.mjs';

function json(status, body) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

function fakeHomeserver({
  existing = {},
  raceUsers = [],
  unavailableUsers = [],
  registrationToken = 'registration-token-0123456789',
} = {}) {
  const accounts = new Map(Object.entries(existing));
  const races = new Set(raceUsers);
  const unavailable = new Set(unavailableUsers);
  let registrations = 0;
  let session = 0;

  const fetchImpl = async (url, options = {}) => {
    const endpoint = new URL(url).pathname;
    const body = JSON.parse(options.body || '{}');
    const username = body.identifier?.user || body.username;

    if (unavailable.has(username)) return json(503, { errcode: 'M_UNAVAILABLE' });

    if (endpoint.endsWith('/login')) {
      return accounts.get(username) === body.password
        ? json(200, { user_id: `@${username}:example.test`, access_token: `login-token-${username}` })
        : json(403, { errcode: 'M_FORBIDDEN', error: 'Invalid username or password' });
    }

    if (!endpoint.endsWith('/register')) return json(404, { errcode: 'M_NOT_FOUND' });
    if (accounts.has(username)) return json(400, { errcode: 'M_USER_IN_USE' });

    if (!body.auth) {
      session += 1;
      return json(401, {
        session: `session-${session}`,
        flows: [{ stages: ['m.login.registration_token'] }],
      });
    }

    if (body.auth.type !== 'm.login.registration_token' || body.auth.token !== registrationToken) {
      return json(401, { errcode: 'M_FORBIDDEN' });
    }

    if (races.has(username)) {
      races.delete(username);
      accounts.set(username, body.password);
      return json(400, { errcode: 'M_USER_IN_USE' });
    }

    registrations += 1;
    accounts.set(username, body.password);
    return json(200, {
      user_id: `@${username}:example.test`,
      access_token: `registration-token-${username}`,
    });
  };

  return {
    fetchImpl,
    accountCount: () => accounts.size,
    registrations: () => registrations,
  };
}

const accounts = [
  { username: 'admin', password: 'admin-secret-012345' },
  { username: 'agent-bridge', password: 'bot-secret-01234567' },
];
const registrationToken = 'registration-token-0123456789';

test('first bootstrap registers and second bootstrap only logs in', async () => {
  const server = fakeHomeserver();

  const first = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts,
    registrationToken,
    fetchImpl: server.fetchImpl,
  });
  const countAfterFirst = server.accountCount();
  const second = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts,
    registrationToken,
    fetchImpl: server.fetchImpl,
  });

  assert.equal(first.ok, true);
  assert.deepEqual(first.results.map((result) => result.status), ['created', 'created']);
  assert.equal(second.ok, true);
  assert.deepEqual(second.results.map((result) => result.status), ['existing', 'existing']);
  assert.equal(server.registrations(), 2);
  assert.equal(server.accountCount(), countAfterFirst);
  assert.equal(JSON.stringify([first, second]).includes('secret-'), false);
  assert.equal(JSON.stringify([first, second]).includes('token-'), false);
});

test('registration race retries login and reports the account ready', async () => {
  const server = fakeHomeserver({ raceUsers: ['agent-bridge'] });
  const result = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts: [accounts[1]],
    registrationToken,
    fetchImpl: server.fetchImpl,
  });

  assert.equal(result.ok, true);
  assert.equal(result.results[0].status, 'existing-after-race');
});

test('wrong password for an existing account is surfaced without a secret', async () => {
  const server = fakeHomeserver({ existing: { admin: 'different-password-000' } });
  const result = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts: [accounts[0]],
    registrationToken,
    fetchImpl: server.fetchImpl,
  });

  assert.equal(result.ok, false);
  assert.equal(result.results[0].status, 'password-mismatch');
  assert.match(result.results[0].error, /password/i);
  assert.equal(JSON.stringify(result).includes(accounts[0].password), false);
});

test('one unavailable account does not prevent later accounts from bootstrapping', async () => {
  const server = fakeHomeserver({ unavailableUsers: ['offline'] });
  const result = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts: [
      { username: 'offline', password: 'offline-secret-0123' },
      accounts[1],
    ],
    registrationToken,
    fetchImpl: server.fetchImpl,
  });

  assert.equal(result.ok, false);
  assert.equal(result.results[0].status, 'failed');
  assert.equal(result.results[1].status, 'created');
  assert.equal(server.accountCount(), 1);
});

test('bootstrap rejects a missing or placeholder registration token', async () => {
  const server = fakeHomeserver();
  await assert.rejects(
    bootstrapAccounts({
      homeserver: 'http://matrix.example.test',
      accounts: [accounts[0]],
      registrationToken: '<generate-me>',
      fetchImpl: server.fetchImpl,
    }),
    /registrationToken/,
  );
});

test('admin account promotion is idempotent and promotion failures fail bootstrap', async () => {
  const server = fakeHomeserver();
  const promoted = [];
  const admin = { ...accounts[0], role: 'admin' };

  const first = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts: [admin],
    registrationToken,
    fetchImpl: server.fetchImpl,
    promoteAdmin: async (username) => promoted.push(username),
  });
  const second = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts: [admin],
    registrationToken,
    fetchImpl: server.fetchImpl,
    promoteAdmin: async (username) => promoted.push(username),
  });

  assert.equal(first.ok, true);
  assert.equal(first.results[0].admin, true);
  assert.equal(second.ok, true);
  assert.equal(second.results[0].admin, true);
  assert.deepEqual(promoted, ['admin', 'admin']);

  const failed = await bootstrapAccounts({
    homeserver: 'http://matrix.example.test',
    accounts: [admin],
    registrationToken,
    fetchImpl: server.fetchImpl,
    promoteAdmin: async () => { throw new Error('database rejected promotion'); },
  });
  assert.equal(failed.ok, false);
  assert.equal(failed.results[0].status, 'admin-promotion-failed');
  assert.match(failed.results[0].error, /promotion failed/i);
});

test('compose admin promoter performs one bounded database update', async () => {
  const calls = [];
  const promote = createComposeAdminPromoter({
    demoDir: '/private/demo/palpo',
    serverName: 'matrix.example.test',
    dbUser: 'palpo',
    dbName: 'palpo',
    envFile: '/private/runtime/e2e.env',
    projectName: 'agentchat-palpo-e2e',
    runCommand: async (command, args, options) => {
      calls.push({ command, args, options });
      return { status: 0, stdout: 't\n', stderr: '' };
    },
  });

  await promote('admin');
  assert.equal(calls.length, 1);
  assert.equal(calls[0].command, 'docker');
  assert.deepEqual(calls[0].args.slice(0, 9), [
    'compose', '--project-name', 'agentchat-palpo-e2e',
    '--env-file', '/private/runtime/e2e.env',
    '-f', '/private/demo/palpo/compose.yml', '--profile', 'palpo-local',
  ]);
  assert.match(calls[0].args.at(-1), /UPDATE users SET is_admin = TRUE/);
  assert.match(calls[0].args.at(-1), /@admin:matrix\.example\.test/);
  assert.equal(JSON.stringify(calls).includes('password'), false);

  const noRowPromoter = createComposeAdminPromoter({
    demoDir: '/private/demo/palpo',
    serverName: 'matrix.example.test',
    runCommand: async () => ({ status: 0, stdout: '', stderr: '' }),
  });
  await assert.rejects(noRowPromoter('admin'), /not found/i);
});
