# What's New in Inskea

## v3.1.3
### Canvas Action Notifications
- Action feedback: Instant toast notifications for grouping, ungrouping, saving, autosaving, duplicating, pasting, deleting, aligning, distributing, and layer reordering.
- Streamlined settings: Cleaned up settings panel version display.

## v3.1.2
### 120 FPS Rendering & GPU Acceleration
- High-refresh rate canvas: Optimized rendering pipeline for 120Hz, 144Hz, and ProMotion displays with zero-copy pointer caching.
- Hardware-accelerated graphics: Dedicated GPU composited canvas layers, asynchronous off-thread image decoding, and desktop GPU rasterization.
- Viewport overscan margin: Virtualized panning buffer prevents DOM churn when panning around complex scenes.

### Smooth Collection Snippet Placement
- Centered cursor placement: Adding snippets from the local or global catalogue centers them directly under the cursor until clicked onto the canvas.
- Continuous edge follow: Canvas edge panning seamlessly scrolls the viewport while moving snippets into position.

### Non-Destructive Imports & Smart Paste
- Seamless `.skea` drops: Drag and drop `.skea` files directly onto any existing canvas without wiping your active drawing.
- Smart clipboard paste: Paste screenshots and images directly from your clipboard onto the canvas.

### Refined Export Scopes
- Fine-grained exports: Export Full Canvas, Current Viewport, Selected Elements, or a Custom Crop Region.
- Improved document naming: Automatic filename inference based on opened drawings.
