# Inskea

**An infinite canvas for your ideas.**

Sketch shapes, jot notes, sketch freehand, drop in charts and images — all on a canvas that never runs out of room. Save your work as a real, reopenable file and pick up right where you left off.

## Features

- 🖊️ **Draw anything** — rectangles, ellipses, lines, arrows, freehand sketches, and text, all on one canvas
- 🔍 **Infinite pan & zoom** — your canvas has no edges; zoom in for detail, zoom out for the big picture
- 🎨 **Clean, consistent styling** — pick from a curated color palette, adjust stroke width, style, and opacity
- 🖱️ **Full manipulation** — select, move, resize, rotate, and layer any element with intuitive handles
- ↩️ **Undo/redo** — experiment freely, nothing is ever a mistake for long
- 📊 **Built-in charts** — drop in bar, line, or pie charts without leaving the canvas
- 🖼️ **Images** — drag and drop images straight onto your canvas
- 💾 **Real files, not screenshots** — save and reopen your work as a native `.skea` file
- 📤 **Export anywhere** — flatten your canvas to PNG or SVG for sharing outside the app
- ⚡ **Fast everywhere** — run it as a native desktop app, or right in your browser

## Built with

Inskea is built entirely in **Rust**, and runs both as a native desktop app and in the browser from the same codebase. It's powered by:

- **[Tauri](https://tauri.app/)** — wraps the app in a lightweight native window for the desktop build, with access to the file system, dialogs, and OS integrations, at a fraction of the size of an Electron app
- **[Leptos](https://leptos.dev/)** — a Rust web framework with fine-grained reactivity, compiled to WebAssembly, powering the UI and canvas in both the desktop and browser builds
- **[Tailwind CSS](https://tailwindcss.com/)** — utility-first styling for the toolbar, panels, and UI chrome
- **SVG rendering** — every shape on the canvas is a real, inspectable element rather than raw pixels, which is what makes selection, styling, and hit-testing feel snappy and precise
- **serde** — handles serialization for the native `.skea` file format

The core editor logic is the same either way — Tauri is just the shell that adds native file system access and packaging when you want a standalone desktop app; drop that layer and the same Leptos app runs straight in a browser tab.

