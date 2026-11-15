<p align="center">
  <img src="assets/banner.png" alt="Inskea, an infinite canvas for your ideas" width="100%">
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/inskea"><img src="https://img.shields.io/npm/v/inskea?style=flat-square&color=cb3837" alt="npm package"></a>
  <img src="https://img.shields.io/badge/version-2.2.0-blue?style=flat-square" alt="Version 2.1.4">
  <img src="https://img.shields.io/badge/platform-desktop%20%7C%20browser-blueviolet?style=flat-square" alt="Platform: desktop and browser">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License: MIT"></a>
</p>

---
Inskea is a drawing app without edges. One canvas that never runs out of room, for sketching diagrams, drafting ideas, and drawing by hand. When you finish, save your work to a real file and pick it back up later.

It runs as a native desktop app and in the browser, with the same tools and the same files either way. Designed for fluid one-handed use while drawing, so your other hand never has to leave the mouse or pen.

<p align="center">
  <img src="assets/app-preview.png" alt="Inskea Canvas" width="100%">
</p>


## Features

### Draw & Style
- **Shapes & Freehand** – Rectangles, ellipses, straight/curved lines, and smoothed freehand sketching.
- **Smart Connectors** – Dynamic endpoints that snap to shapes, track movement, and route with direct, curved, or orthogonal paths.
- **Images & Text** – Place and transform images (PNG, JPEG, GIF, WebP, SVG, BMP) or label shapes (`Ctrl+T`).
- **Styling** – Color palettes, fills, stroke widths, opacity levels, and dashed/dotted lines.

### Arrange & Canvas
- **Infinite Canvas** – Boundless pan and zoom (10% to 2000%) with customizable grid and center markers.
- **Transform & Align** – Multi-selection, 8-handle resizing, live Alt-snapping, smart alignment guides, and even distribution.
- **Groups & Layers** – Group elements with in-place nested editing and z-order layering.
- **Themes** – Tokyo Night, Storm, Paper, Greenboard, and Mono.

### Files & Export
- **`.skea` Format** – Fully editable, non-destructive file format.
- **Autosave** – Background autosaving to local files or browser storage.
- **Export** – High-resolution PNG and clean vector SVG exports with crop area selection.
- **Cross-Platform** – Native desktop app for macOS, Linux, and Windows, plus web browser support.

## Installation

### Via npm (Linux & Windows)

```bash
npm install -g inskea
```
```bash
npm update -g inskea
```

*(You can also run it instantly without installing: `npx inskea`)*

### Native Installers

Download the macOS disk image (`.dmg`, available for Apple Silicon and Intel), Windows installer (`.exe`), Debian/Ubuntu package (`.deb`), or Arch Linux package from [Releases](https://github.com/Quippy22/Inskea/releases).

On macOS, open the downloaded disk image and drag Inskea into Applications. If macOS blocks the app because it was downloaded from the internet, Control-click Inskea in Applications, choose **Open**, then confirm.

### Build and run on macOS

Install Xcode Command Line Tools, Rust with the WebAssembly target, Node.js with npm, and Trunk:

```bash
xcode-select --install
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

From the repository root, install the JavaScript dependencies once, then run either the browser version or the native macOS app:

```bash
npm install
trunk serve                 # Browser version
npm run tauri -- dev        # Native desktop app
```

To package the native app as a macOS disk image:

```bash
npm run tauri -- build --bundles dmg
```

## Keyboard shortcuts

Press `?` anywhere in the app to view the shortcut overlay.

<p align="center">
  <img src="assets/shortcuts.png" alt="Keyboard Shortcuts" width="85%">
</p>
