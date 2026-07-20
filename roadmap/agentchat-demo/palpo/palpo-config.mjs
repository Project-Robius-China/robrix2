#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { chmod, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const moduleDir = path.dirname(fileURLToPath(import.meta.url));
const placeholderPattern = /^(?:change[-_ ]?me|your[-_].*|dev[-_].*)$/i;

function required(env, name) {
  const value = String(env[name] ?? '').trim();
  if (!value || /[<>]/.test(value) || placeholderPattern.test(value)) {
    throw new Error(`${name} must be set to a non-placeholder value`);
  }
  if (/\p{Cc}/u.test(value)) {
    throw new Error(`${name} contains a control character`);
  }
  return value;
}

function validatePort(env, name) {
  const value = required(env, name);
  if (!/^[0-9]{1,5}$/.test(value) || Number(value) < 1 || Number(value) > 65535) {
    throw new Error(`${name} must be a TCP port between 1 and 65535`);
  }
  return value;
}

function validateServerName(env) {
  const value = required(env, 'PALPO_SERVER_NAME');
  const host = value.startsWith('[')
    ? value.match(/^\[([0-9a-f:]+)\](?::[0-9]{1,5})?$/i)?.[1]
    : value.match(/^([a-z0-9.-]+)(?::[0-9]{1,5})?$/i)?.[1];
  if (!host || host.startsWith('.') || host.endsWith('.') || host.includes('..')) {
    throw new Error('PALPO_SERVER_NAME must be a hostname or IP address with an optional port');
  }
  const port = value.match(/:([0-9]{1,5})$/)?.[1];
  if (port && (Number(port) < 1 || Number(port) > 65535)) {
    throw new Error('PALPO_SERVER_NAME contains an invalid port');
  }
  return value;
}

function validateUrl(env, name) {
  const value = required(env, name);
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    throw new Error(`${name} must be an absolute HTTP(S) URL`);
  }
  if (!['http:', 'https:'].includes(parsed.protocol) || !parsed.hostname
      || parsed.username || parsed.password || parsed.search || parsed.hash) {
    throw new Error(`${name} must be an absolute HTTP(S) URL without credentials, query, or fragment`);
  }
  return value.replace(/\/$/, '');
}

function validateIdentifier(env, name) {
  const value = required(env, name);
  if (!/^[a-z0-9._=-]+$/i.test(value)) {
    throw new Error(`${name} contains unsupported characters`);
  }
  return value;
}

function validateSecret(env, name) {
  const value = required(env, name);
  if (value.length < 16 || !/^[A-Za-z0-9._~+-]+$/.test(value)) {
    throw new Error(`${name} must contain at least 16 URL-safe characters`);
  }
  return value;
}

function fingerprint(value) {
  return createHash('sha256').update(value).digest('hex');
}

function renderTemplate(template, values) {
  const rendered = template.replace(/\{\{([A-Z0-9_]+)\}\}/g, (marker, name) => {
    if (!(name in values)) throw new Error(`unknown template marker ${marker}`);
    return values[name];
  });
  if (/\{\{[A-Z0-9_]+\}\}/.test(rendered)) throw new Error('unresolved template marker');
  return rendered;
}

async function writePrivate(filename, content) {
  await writeFile(filename, content, { encoding: 'utf8', mode: 0o600 });
  await chmod(filename, 0o600);
}

export async function renderConfig({ env, outputDir }) {
  if (!outputDir || !path.isAbsolute(outputDir)) {
    throw new Error('outputDir must be an absolute path');
  }

  const serverName = validateServerName(env);
  const publicUrl = validateUrl(env, 'PALPO_PUBLIC_URL');
  validatePort(env, 'PALPO_HOST_PORT');
  const dbHost = validateIdentifier(env, 'PALPO_DB_HOST');
  const dbPort = validatePort(env, 'PALPO_DB_PORT');
  const dbName = validateIdentifier(env, 'PALPO_DB_NAME');
  const dbUser = validateIdentifier(env, 'PALPO_DB_USER');
  const dbPassword = validateSecret(env, 'PALPO_DB_PASSWORD');
  const appserviceUrl = validateUrl(env, 'PALPO_APPSERVICE_URL');
  const asToken = validateSecret(env, 'PALPO_AS_TOKEN');
  const hsToken = validateSecret(env, 'PALPO_HS_TOKEN');
  const registrationToken = validateSecret(env, 'PALPO_REGISTRATION_TOKEN');
  const senderLocalpart = validateIdentifier(env, 'PALPO_SENDER_LOCALPART');

  const [palpoTemplate, appserviceTemplate] = await Promise.all([
    readFile(path.join(moduleDir, 'templates', 'palpo.toml.tpl'), 'utf8'),
    readFile(path.join(moduleDir, 'templates', 'appservice-agentchat.yaml.tpl'), 'utf8'),
  ]);
  const serverRegex = serverName.replace(/[\\.^$|?*+()[\]{}-]/g, '\\$&').replace(/\\/g, '\\\\');
  const palpo = renderTemplate(palpoTemplate, {
    SERVER_NAME: serverName,
    PUBLIC_URL: publicUrl,
    DB_HOST: dbHost,
    DB_PORT: dbPort,
    DB_NAME: dbName,
    DB_USER: dbUser,
    DB_PASSWORD: dbPassword,
    REGISTRATION_TOKEN: registrationToken,
  });
  const appservice = renderTemplate(appserviceTemplate, {
    APPSERVICE_URL: appserviceUrl,
    AS_TOKEN: asToken,
    HS_TOKEN: hsToken,
    SENDER_LOCALPART: senderLocalpart,
    SERVER_REGEX: serverRegex,
  });
  const deployment = {
    schemaVersion: 1,
    serverName,
    publicUrl,
    appserviceUrl,
    senderLocalpart,
    fingerprints: {
      asToken: fingerprint(asToken),
      hsToken: fingerprint(hsToken),
      registrationToken: fingerprint(registrationToken),
    },
  };

  await mkdir(outputDir, { recursive: true, mode: 0o700 });
  const files = {
    palpo: path.join(outputDir, 'palpo.toml'),
    appservice: path.join(outputDir, 'appservice-agentchat.yaml'),
    deployment: path.join(outputDir, 'deployment.json'),
  };
  await Promise.all([
    writePrivate(files.palpo, palpo),
    writePrivate(files.appservice, appservice),
    writePrivate(files.deployment, `${JSON.stringify(deployment, null, 2)}\n`),
  ]);
  return { files, deployment };
}

async function main() {
  const outputDir = path.resolve(
    process.argv[2]
      || path.join(process.env.PALPO_RUNTIME_DIR || path.join(moduleDir, '.runtime'), 'config'),
  );
  const { files } = await renderConfig({ env: process.env, outputDir });
  process.stdout.write(`${JSON.stringify({ ok: true, files }, null, 2)}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  main().catch((error) => {
    process.stderr.write(`[palpo-config] ${error.message}\n`);
    process.exitCode = 1;
  });
}
