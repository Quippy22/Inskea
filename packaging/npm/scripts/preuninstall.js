#!/usr/bin/env node

const os = require('os');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

function removeLinux() {
  const home = os.homedir();
  const desktopFile = path.join(home, '.local', 'share', 'applications', 'inskea.desktop');
  const iconFile = path.join(home, '.local', 'share', 'icons', 'hicolor', '128x128', 'apps', 'inskea.png');
  const mimeFile = path.join(home, '.local', 'share', 'mime', 'packages', 'inskea.xml');

  if (fs.existsSync(desktopFile)) {
    try {
      fs.unlinkSync(desktopFile);
      execSync(`update-desktop-database "${path.join(home, '.local', 'share', 'applications')}"`, { stdio: 'ignore' });
    } catch (_) {}
  }

  if (fs.existsSync(iconFile)) {
    try { fs.unlinkSync(iconFile); } catch (_) {}
  }

  if (fs.existsSync(mimeFile)) {
    try {
      fs.unlinkSync(mimeFile);
      execSync(`update-mime-database "${path.join(home, '.local', 'share', 'mime')}"`, { stdio: 'ignore' });
    } catch (_) {}
  }
}

function removeWindows() {
  const appData = process.env.APPDATA;
  if (!appData) return;

  const shortcutPath = path.join(appData, 'Microsoft', 'Windows', 'Start Menu', 'Programs', 'Inskea.lnk');
  if (fs.existsSync(shortcutPath)) {
    try { fs.unlinkSync(shortcutPath); } catch (_) {}
  }
}

try {
  if (process.platform === 'linux') {
    removeLinux();
  } else if (process.platform === 'win32') {
    removeWindows();
  }
} catch (_) {}
