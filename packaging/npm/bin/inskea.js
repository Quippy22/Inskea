#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const key = `${process.platform}-${process.arch}`;

// Resolve platform package name from optionalDependencies in package.json
let pkgName;
try {
  const rootPkg = require(path.join(__dirname, '..', 'package.json'));
  const optionalDeps = Object.keys(rootPkg.optionalDependencies || {});
  pkgName = optionalDeps.find(name => name.endsWith(`-${key}`));
} catch (_) {}

if (!pkgName) {
  pkgName = `@quippy/inskea-${key}`;
}

if (process.platform !== 'linux' && process.platform !== 'win32') {
  console.error(`[inskea] Unsupported platform: ${process.platform}`);
  console.error('Currently supported platforms: linux-x64, win32-x64');
  process.exit(1);
}

function resolveBinary() {
  const binaryName = process.platform === 'win32' ? 'inskea.exe' : 'inskea';

  // 1. Try require.resolve for the scoped platform package
  try {
    const pkgJson = require.resolve(`${pkgName}/package.json`);
    const candidate = path.join(path.dirname(pkgJson), 'bin', binaryName);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  } catch (_) {}

  // 2. Search common node_modules layouts (flat, hoisted, or nested)
  const searchPaths = [
    path.join(__dirname, '..', 'node_modules', pkgName, 'bin', binaryName),
    path.join(__dirname, '..', '..', pkgName, 'bin', binaryName),
    path.join(__dirname, '..', '..', '..', pkgName, 'bin', binaryName),
    path.join(__dirname, '..', '..', '..', '..', pkgName, 'bin', binaryName)
  ];

  for (const candidate of searchPaths) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return null;
}

const binaryPath = resolveBinary();
if (!binaryPath) {
  console.error(
    `[inskea] Could not locate native executable for ${key}.\n` +
    `Expected package "${pkgName}" to be installed. Please try reinstalling with:\n` +
    `  npm install -g inskea`
  );
  process.exit(1);
}

// Ensure execution permissions on Unix
if (process.platform !== 'win32') {
  try {
    fs.chmodSync(binaryPath, 0o755);
  } catch (_) {}
}

const isCliCheck = process.argv.some(arg =>
  arg === '--help' || arg === '-h' || arg === '--version' || arg === '-V'
);

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: isCliCheck ? 'inherit' : 'ignore',
  detached: !isCliCheck
});

if (!isCliCheck) {
  child.unref();
  process.exit(0);
} else {
  child.on('exit', (code) => process.exit(code ?? 0));
}
