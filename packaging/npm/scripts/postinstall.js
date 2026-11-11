#!/usr/bin/env node

const os = require('os');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

function installLinux() {
  // If running with root/sudo, install to standard system-wide /usr/local/share so it is available to ALL users on the machine.
  // Otherwise, install to standard user-level ~/.local/share.
  const isRoot = typeof process.getuid === 'function' && process.getuid() === 0;
  const baseDir = isRoot ? '/usr/local/share' : path.join(os.homedir(), '.local', 'share');

  const appsDir = path.join(baseDir, 'applications');
  const iconsDir = path.join(baseDir, 'icons', 'hicolor', '128x128', 'apps');
  const mimeDir = path.join(baseDir, 'mime', 'packages');

  fs.mkdirSync(appsDir, { recursive: true });
  fs.mkdirSync(iconsDir, { recursive: true });
  fs.mkdirSync(mimeDir, { recursive: true });

  const assetsDir = path.join(__dirname, '..', 'assets');

  // Copy icon
  const iconSrc = path.join(assetsDir, 'inskea.png');
  const iconDest = path.join(iconsDir, 'inskea.png');
  if (fs.existsSync(iconSrc)) {
    fs.copyFileSync(iconSrc, iconDest);
  }

  // Copy MIME XML
  const mimeSrc = path.join(assetsDir, 'inskea-mime.xml');
  const mimeDest = path.join(mimeDir, 'inskea.xml');
  if (fs.existsSync(mimeSrc)) {
    fs.copyFileSync(mimeSrc, mimeDest);
    try {
      execSync(`update-mime-database "${path.join(baseDir, 'mime')}"`, { stdio: 'ignore' });
    } catch (_) {}
  }

  // Copy .desktop file
  const desktopSrc = path.join(assetsDir, 'inskea.desktop');
  const desktopDest = path.join(appsDir, 'inskea.desktop');
  if (fs.existsSync(desktopSrc)) {
    fs.copyFileSync(desktopSrc, desktopDest);
    try {
      execSync(`update-desktop-database "${appsDir}"`, { stdio: 'ignore' });
    } catch (_) {}
  }

  console.log(`[inskea] Registered desktop launcher in ${appsDir}`);
}

function installWindows() {
  const appData = process.env.APPDATA;
  if (!appData) return;

  const startMenuDir = path.join(appData, 'Microsoft', 'Windows', 'Start Menu', 'Programs');
  if (!fs.existsSync(startMenuDir)) return;

  const shortcutPath = path.join(startMenuDir, 'Inskea.lnk');
  const npmCmd = path.join(appData, 'npm', 'inskea.cmd');

  const psCommand = `
    $ws = New-Object -ComObject WScript.Shell;
    $s = $ws.CreateShortcut('${shortcutPath.replace(/'/g, "''")}');
    $s.TargetPath = '${npmCmd.replace(/'/g, "''")}';
    $s.Description = 'Inskea vector illustration app';
    $s.Save();
  `.replace(/\r?\n\s*/g, ' ');

  try {
    execSync(`powershell -NoProfile -Command "${psCommand}"`, { stdio: 'ignore' });
    console.log('[inskea] Registered Windows Start Menu shortcut');
  } catch (_) {}
}

try {
  if (process.platform === 'linux') {
    installLinux();
  } else if (process.platform === 'win32') {
    installWindows();
  }
} catch (err) {
  // Never fail npm install if desktop integration encounters permission/missing command warnings
  console.warn('[inskea] Notice: Could not register desktop shortcut automatically:', err.message);
}
