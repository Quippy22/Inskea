#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node packaging/npm/publish.js <version> [--dry-run]');
  process.exit(1);
}

const isDryRun = process.argv.includes('--dry-run');
const rootDir = path.resolve(__dirname, '../..');
const npmDir = path.resolve(__dirname);
const outDir = path.join(rootDir, 'target', 'npm-dist');

let SCOPE = '@quippy';
try {
  const whoami = execSync('npm whoami', { encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] }).trim();
  if (whoami) {
    SCOPE = whoami.startsWith('@') ? whoami : `@${whoami}`;
    console.log(`[npm-publish] Authenticated as ${whoami}, using scope ${SCOPE}`);
  }
} catch (_) {
  console.log(`[npm-publish] Using default scope ${SCOPE}`);
}

const LINUX_PKG = `${SCOPE}/inskea-linux-x64`;
const WIN_PKG = `${SCOPE}/inskea-win32-x64`;

console.log(`[npm-publish] Preparing packages for Inskea v${version}...`);

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

// 1. Prepare Linux package
const linuxDir = path.join(outDir, 'linux-x64');
const linuxBinDir = path.join(linuxDir, 'bin');
fs.mkdirSync(linuxBinDir, { recursive: true });

const linuxBinSource = path.join(npmDir, 'dist-bin', 'linux', 'inskea');
if (!fs.existsSync(linuxBinSource)) {
  console.error(`Missing Linux binary at ${linuxBinSource}`);
  process.exit(1);
}
fs.copyFileSync(linuxBinSource, path.join(linuxBinDir, 'inskea'));
fs.chmodSync(path.join(linuxBinDir, 'inskea'), 0o755);

fs.writeFileSync(
  path.join(linuxDir, 'package.json'),
  JSON.stringify(
    {
      name: LINUX_PKG,
      version: version,
      description: 'Linux x64 native binary for Inskea',
      os: ['linux'],
      cpu: ['x64'],
      bin: {
        inskea: 'bin/inskea'
      },
      files: ['bin/inskea'],
      license: 'MIT',
      repository: {
        type: 'git',
        url: 'https://github.com/Quippy22/Inskea'
      }
    },
    null,
    2
  )
);

// 2. Prepare Windows package
const winDir = path.join(outDir, 'win32-x64');
const winBinDir = path.join(winDir, 'bin');
fs.mkdirSync(winBinDir, { recursive: true });

const winBinSource = path.join(npmDir, 'dist-bin', 'windows', 'inskea.exe');
if (!fs.existsSync(winBinSource)) {
  console.error(`Missing Windows binary at ${winBinSource}`);
  process.exit(1);
}
fs.copyFileSync(winBinSource, path.join(winBinDir, 'inskea.exe'));

fs.writeFileSync(
  path.join(winDir, 'package.json'),
  JSON.stringify(
    {
      name: WIN_PKG,
      version: version,
      description: 'Windows x64 native binary for Inskea',
      os: ['win32'],
      cpu: ['x64'],
      bin: {
        'inskea.exe': 'bin/inskea.exe'
      },
      files: ['bin/inskea.exe'],
      license: 'MIT',
      repository: {
        type: 'git',
        url: 'https://github.com/Quippy22/Inskea'
      }
    },
    null,
    2
  )
);

// 3. Prepare Main wrapper package
const mainDir = path.join(outDir, 'main');
fs.mkdirSync(mainDir, { recursive: true });

// Copy bin, scripts, assets
function copyFolder(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyFolder(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
      if (entry.name.endsWith('.js')) {
        fs.chmodSync(destPath, 0o755);
      }
    }
  }
}

copyFolder(path.join(npmDir, 'bin'), path.join(mainDir, 'bin'));
copyFolder(path.join(npmDir, 'scripts'), path.join(mainDir, 'scripts'));
copyFolder(path.join(npmDir, 'assets'), path.join(mainDir, 'assets'));

const readmeContent = `# Inskea

Infinite-canvas vector drawing application for sketching shapes, freehand strokes, and text.

## Installation

\`\`\`bash
npm install -g inskea
\`\`\`

## Run

\`\`\`bash
inskea
\`\`\`

Or open a specific drawing:

\`\`\`bash
inskea my-sketch.skea
\`\`\`

## Instant Trial (No install)

\`\`\`bash
npx inskea
\`\`\`
`;

fs.writeFileSync(path.join(mainDir, 'README.md'), readmeContent);

fs.writeFileSync(
  path.join(mainDir, 'package.json'),
  JSON.stringify(
    {
      name: 'inskea',
      version: version,
      description: 'Inskea - infinite-canvas vector illustration app',
      bin: {
        inskea: 'bin/inskea.js'
      },
      scripts: {
        postinstall: 'node scripts/postinstall.js',
        preuninstall: 'node scripts/preuninstall.js'
      },
      optionalDependencies: {
        [LINUX_PKG]: version,
        [WIN_PKG]: version
      },
      keywords: [
        'inskea',
        'drawing',
        'vector',
        'canvas',
        'illustration',
        'sketch',
        'tauri',
        'desktop'
      ],
      author: 'Quippy22',
      license: 'MIT',
      repository: {
        type: 'git',
        url: 'https://github.com/Quippy22/Inskea'
      },
      homepage: 'https://github.com/Quippy22/Inskea#readme'
    },
    null,
    2
  )
);

console.log('[npm-publish] Packages assembled successfully.');

const packagesToPublish = [
  { name: LINUX_PKG, dir: linuxDir },
  { name: WIN_PKG, dir: winDir },
  { name: 'inskea', dir: mainDir }
];

for (const pkg of packagesToPublish) {
  console.log(`[npm-publish] Publishing ${pkg.name}@${version}...`);
  if (isDryRun) {
    console.log(`[npm-publish] (dry run) Would run: npm publish ${pkg.dir} --access public`);
  } else {
    execSync(`npm publish "${pkg.dir}" --access public`, {
      stdio: 'inherit'
    });
  }
}

console.log('[npm-publish] All packages published to npm successfully!');
