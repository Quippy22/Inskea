<p align="center">
  <img src="assets/banner.png" alt="Inskea, an infinite canvas for your ideas" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/version-1.4.1-blue?style=flat-square" alt="Version 1.4.1">
  <img src="https://img.shields.io/badge/platform-desktop%20%7C%20browser-blueviolet?style=flat-square" alt="Platform: desktop and browser">
</p>

---
Inskea is a drawing app without edges. One canvas that never runs out of room, for sketching diagrams, drafting ideas, and drawing by hand. When you finish, save your work to a real file and pick it back up later.

It runs as a native desktop app and in the browser, with the same tools and the same files either way.

### Draw

- **Shapes.** Rectangles with sharp or rounded corners, ellipses, and lines that are straight or gently curved. Hold Shift while drawing for perfect squares and circles.
- **Arrows.** Add an arrowhead to either end of a line, or both.
- **Freehand.** Strokes are smoothed as you draw, which keeps hand-drawn lines from looking jittery.
- **Images.** Import a PNG, JPEG, GIF, WebP, SVG, or BMP from Import → Image, or drop one onto the canvas. Images move, resize, and rotate like any other element, and the image data travels with the drawing.
- **Text.** Click to place a cursor and type. Resize the box to set how the text wraps, change the font size, and align text left, center, or right.
- **Eraser.** Drag across anything you want to remove.
- **Point edits.** Reshape a line or arrow after the fact by dragging its points, or drag between two points to add a bend.

### Style

- **Colors.** Quick swatches or a full color picker for strokes, plus fills for rectangles and ellipses.
- **Stroke options.** Solid, dashed, or dotted lines with adjustable width and opacity.

### Arrange

- **Selection.** Click an element, Shift-click to add or drop items, or drag a box to select everything inside.
- **Move, resize, rotate.** Drag elements directly, resize from eight handles, and rotate from the top handle. Hold Shift to keep proportions or snap rotation to 15 degree steps, and Alt to resize around the center. While moving, hold Alt to snap the selection to the grid live, even with the grid hidden.
- **Smart guides.** Hold Shift while moving elements and guides appear, snapping your selection to shared edges, centers, and even spacing.
- **Groups.** Combine elements into a single unit, then double-click to step inside and edit one part.
- **Alignment.** Align elements six ways, or distribute spacing evenly along either axis.
- **Layering.** Move elements forward or backward, or jump straight to front or back.
- **Copy, paste, duplicate.** Groups stay intact.
- **Undo and redo.** Up to 100 steps of history.

### Navigate

- **Infinite pan and zoom.** Zoom with the scroll wheel from 10% up to 2000%, centered on your pointer, and drag with the Hand tool to move around.
- **Reset view.** Click the zoom readout to jump back to 100%, centered.
- **Grid.** Dots, lines, or off, with adjustable spacing. Alt-snapping follows the spacing you set.
- **Center marker.** A crosshair or dot at the origin, so you can find your way back.
- **Themes.** Storm, Tokyo Night, Mono, Paper, and Greenboard. Switch from the settings panel; the choice applies to the whole app and is remembered.

### Files and export

- **`.skea` files.** Inskea's own format. Reopen your work at any time and keep editing; nothing gets flattened to a picture.
- **Save and open anywhere.** On the desktop, Open, Save, and Save As use native dialogs. In the browser, Save and Save As download a copy of the drawing, and Open reads one back from disk.
- **Autosave.** Turn it on in the settings and Inskea writes changes back a moment after you stop editing: to the open file on the desktop, or to browser storage so a reload restores the drawing. With no file yet, it asks for one.
- **No surprise losses.** Unsaved work prompts before you start a new drawing, open another one, or close the app, and failed saves, loads, and exports tell you what went wrong.
- **Opens from your file manager.** On the desktop, `.skea` files are associated with Inskea: double-click one to open it, and launching a second copy hands the file to the running window.
- **PNG and SVG export.** Save the current view, or drag a rectangle to pick exactly what to export. PNG for sharing, SVG for editing elsewhere.

## Keyboard shortcuts

The shortcuts are chosen for one-handed use, so your other hand can stay on the mouse or pen while you draw. Press `?` in the app for the full list. The ones you will reach for most:

| Action | Keys |
| --- | --- |
| Select, hand, draw, erase modes | `s`, `a`, `d`, `e` |
| Rectangle, ellipse, line, arrow, text | `1`, `2`, `3`, `4`, `5` |
| Freehand | `f` |
| Undo / redo | `Ctrl+Z` / `Ctrl+Shift+Z` |
| Copy / paste / duplicate | `Ctrl+C` / `Ctrl+V` / `Ctrl+D` |
| Group / ungroup | `Ctrl+G` / `Ctrl+Shift+G` |
| Delete selection | `Delete` or `Backspace` |
| New / open / save / save as | `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` |
| Cancel, deselect, or leave a group | `Esc` |
| Shortcut help | `?` |

## Windows notes

The Windows build installs `Inskea.exe`, uses an Inskea icon at every shell size (including the installer and uninstaller), and sets the bundle identifier as its AppUserModelID so shortcuts, taskbar pins, and file associations resolve to the same app.

`src-tauri/icons/icon.ico` is hand-tuned: 16–64 px use a simplified mark and 128–256 px use the full logo. `cargo tauri icon` regenerates every size from one source image, so restore the tuned `icon.ico` after running it.

Windows 11 also lists a separate `WebView2 Manager` group in Task Manager. Those are Microsoft's WebView2 runtime processes; an app cannot rename or absorb that group. The Inskea process itself is listed separately with the app name and icon.

The release workflow signs the Windows executable and installer when these repository secrets are set:

- `WINDOWS_CERTIFICATE`: the base64-encoded `.pfx` certificate. Run `certutil -encode certificate.pfx base64cert.txt` and paste the contents of `base64cert.txt`, or use `base64 -w0 certificate.pfx` on Linux/macOS.
- `WINDOWS_CERTIFICATE_PASSWORD`: the password used when exporting the `.pfx`

Without a code-signing certificate the installer is unsigned, and SmartScreen shows its standard warning. Signing replaces "Unknown publisher" with the certificate's publisher name; use a certificate from a publicly trusted CA, because a self-signed certificate does not remove the warning. SmartScreen reputation still builds over releases, so a newly issued certificate can show a warning until it has reputation; submitting the installer to Microsoft can also help.
