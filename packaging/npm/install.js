#!/usr/bin/env node

"use strict";

const { spawnSync } = require("node:child_process");
const path = require("node:path");

if (process.env.ROBRIX_NPM_SKIP_INSTALL === "1") {
  process.stdout.write("Skipping native Robrix installation (ROBRIX_NPM_SKIP_INSTALL=1).\n");
  process.exit(0);
}

const requestedArgs = process.argv.slice(2);
const dryRun = requestedArgs.includes("--dry-run");
const downloadOnlyIndex = requestedArgs.indexOf("--download-only");
const downloadDirectory =
  downloadOnlyIndex >= 0 ? requestedArgs[downloadOnlyIndex + 1] : undefined;

if (downloadOnlyIndex >= 0 && !downloadDirectory) {
  process.stderr.write("robrix-install: --download-only requires a directory\n");
  process.exit(2);
}

let command;
let args;

if (process.platform === "win32") {
  command = "powershell.exe";
  args = [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    path.join(__dirname, "robrix-installer.ps1"),
  ];
  if (dryRun) {
    args.push("-DryRun");
  }
  if (downloadDirectory) {
    args.push("-DownloadDirectory", downloadDirectory);
  }
} else if (process.platform === "darwin" || process.platform === "linux") {
  command = "sh";
  args = [path.join(__dirname, "robrix-installer.sh")];
  if (dryRun) {
    args.push("--dry-run");
  }
  if (downloadDirectory) {
    args.push("--download-only", downloadDirectory);
  }
} else {
  process.stderr.write(`robrix-install: unsupported platform ${process.platform}\n`);
  process.exit(1);
}

const result = spawnSync(command, args, {
  env: process.env,
  stdio: "inherit",
});

if (result.error) {
  process.stderr.write(`robrix-install: ${result.error.message}\n`);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
