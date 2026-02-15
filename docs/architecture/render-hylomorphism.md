# Algebraic Render Hylomorphism: Complete Implementation Plan

## Preamble: The Structural Analogy

The input side has already been converted:

```
KeyEvent --> interpret_assignment() --> Vec<InputEffect> --> execute_effects() --> TermWindow mutations
             (anamorphism)              (free algebra)       (catamorphism)
```

The hylomorphism: `perform_key_assignment = execute_effects(interpret_assignment(key))`

The output side will mirror this:

```
TermWindow state --> describe_frame() --> Frame<RenderCommand> --> execute_frame() --> GPU vertex buffers
                     (anamorphism)        (free algebra)           (catamorphism)
```

The hylomorphism: `paint_pass = execute_frame(describe_frame(state))`

The existing `InputEffect` has 65 variants with a `Multiple(Vec<InputEffect>)` for composition. The existing `RenderCommand` has 6 variants with a `Batch(Vec<RenderCommand>)` for composition. The structural parallel is exact.

---

## Question 1: Incremental Strategy

**Recommendation:** Dual-path with per-function strangler fig, NOT big-bang replacement.

### Detailed Rationale

The input side was a simpler transformation: one function (`perform_key_assignment`) was mechanically split into `interpret_assignment` (pure mapping from `KeyAssignment` to `Vec<InputEffect>`) and `execute_effects` (effectful interpretation of `Vec<InputEffect>`). That function lived in a single file and had no internal caching, no retry loops, and no interleaved state mutation.

The render side is fundamentally different in three ways:

**1. Deep interleaving.** `render_screen_line` (at `src/termwindow/render/screen_line.rs`, 905 lines) interleaves description (shape resolution, color computation, range intersection math) with execution (quad allocation via `layers.allocate()`, `quad.set_position()`, `quad.set_fg_color()`). You cannot replace one call site at a time without first splitting the function internally.

**2. Multi-level caching.** There are two caches:
- `line_to_ele_shape_cache` caches the shape/color description (describe-phase output)
- `line_quad_cache` caches the allocated quads (execute-phase output, stored as `HeapQuadAllocator`)

Both caches must be migrated. A big-bang replacement would require migrating both simultaneously.

**3. Retry loop coupling.** The `paint_impl` retry loop (at `src/termwindow/render/paint.rs`, lines 38–106) catches `OutOfTextureSpace` errors from deep within `render_screen_line`, recreates the texture atlas, and retries. The describe functions will also access the texture atlas (for glyph-to-texture-coord resolution), so the retry loop must wrap the new pipeline too.

### Concrete Dual-Path Strategy

**Step 1:** Build `describe_*` functions that return `Vec<RenderCommand>` or `Frame` sub-structures. These functions are written alongside the existing imperative code. They share the same inputs (same `RenderScreenLineParams`, same `PositionedPane`, etc.) but produce data instead of mutating quads.

**Step 2:** Build `execute_frame()` / `execute_commands()` that walks `Frame` / `Vec<RenderCommand>` and writes quads. This is a mechanical translator from `RenderCommand` to `QuadTrait` mutations.

**Step 3:** Add a temporary runtime gate in `paint_pass()` to branch between the old imperative path and the new `execute_frame(describe_frame(self))` path. Both paths go through the same `paint_impl` retry loop and the same `call_draw_webgpu()`.

**Step 4:** Validate. Run both paths side by side in CI or testing. Add a debug mode that runs both paths and compares quad counts per layer.

**Step 5:** Once parity is verified, flip the default to `true`, then delete the old path.

### Why Not Pure Strangler Fig Per-Function?

Because the functions are not independently replaceable. `paint_pane` calls `render_screen_line` which calls `filled_rectangle` and `populate_image_quad` and `layers.allocate` directly. You cannot replace `paint_pane` without also replacing `render_screen_line`. The minimum replaceable unit is the entire `paint_pass` function, which is why the dual-path switch happens at the `paint_pass` level.

### Why Not Big-Bang?

Because the render pipeline is the single most visible system in the application. Any regression means a broken screen. The dual-path approach means the old pipeline continues to work throughout development. The temporary runtime gate provides an escape hatch in production.

---

## Question 2: The Box Model Problem

### Current Architecture

The fancy tab bar, modals, and the experimental `use_box_model_render` pane path all use `render_element()` at `src/termwindow/box_model.rs` line 826.

This function does three things:

1. **Hover resolution** (lines 835–856): Reads `self.current_mouse_event` to determine if the mouse is within the element's bounds. If hovering, uses `hover_colors` instead of `colors`. This is *describe* work.
2. **Background rendering** (line 858): Calls `render_element_background()` which allocates quads for the element's background, borders, and border corners. This is *execute* work.
3. **Content rendering** (lines 861–943): Matches on `ComputedElementContent`:
   - `Text(cells)`: Iterates glyphs/sprites, allocates quads on layer 1 (glyphs) or layer 2 (sprites). Execute work.
   - `Children(kids)`: Recursively calls `render_element` for each child. Execute work.
   - `Poly { poly, line_width }`: Calls `self.poly_quad()` to render polygon shapes. Execute work.

### Design: `describe_element` Produces `Vec<RenderCommand>`

The `ComputedElement` struct (`box_model.rs` line 409) is already a description — it is the output of `compute_element()` and contains resolved bounds, colors, content rects, and baseline. The `render_element` function is purely a translator from `ComputedElement` to quad mutations. Replacing it with a translator from `ComputedElement` to `Vec<RenderCommand>` is mechanical.

```rust
/// Translates a ComputedElement tree into a flat list of RenderCommands.
/// This replaces render_element() in the algebraic pipeline.
pub fn describe_element(
    element: &ComputedElement,
    mouse_pos: Option<(f32, f32)>,
    mouse_capture: Option<&MouseCapture>,
    pixel_dims: (f32, f32),  // (pixel_width, pixel_height)
    inherited_colors: Option<&ElementColors>,
) -> Vec<RenderCommand> {
    let mut commands = vec![];

    // 1. Resolve hover state (currently lines 835-856 of render_element)
    let colors = resolve_hover_colors(element, mouse_pos, mouse_capture);

    // 2. Background (currently render_element_background, line 1010+)
    describe_element_background(element, colors, inherited_colors, pixel_dims, &mut commands);

    // 3. Content
    match &element.content {
        ComputedElementContent::Text(cells) => {
            describe_element_text(element, cells, colors, inherited_colors, pixel_dims, &mut commands);
        }
        ComputedElementContent::Children(kids) => {
            for kid in kids {
                commands.extend(describe_element(kid, mouse_pos, mouse_capture, pixel_dims, Some(colors)));
            }
        }
        ComputedElementContent::Poly { poly, line_width } => {
            describe_element_poly(element, poly, *line_width, colors, inherited_colors, pixel_dims, &mut commands);
        }
    }

    commands
}
```

### The Z-Index Complication

Currently, `render_element` calls `gl_state.layer_for_zindex(element.zindex)` to get a `RenderLayer`, then calls `layer.quad_allocator()` to get a `TripleLayerQuadAllocator` for that z-index group. This means elements at different z-indices write to different vertex buffers.

In the `RenderCommand` world, z-index must become part of the command. The existing `RenderCommand::DrawQuad` has a `layer: usize` field (0, 1, 2 for background/glyphs/overlays within a z-index group). We need to add a `zindex: i8` field or encode z-index differently.

**Design decision:** Add `zindex: i8` to `DrawQuad` and `FillRect`. Default to 0 for normal content. The `execute_frame` catamorphism groups commands by z-index, then by layer within each z-index, and writes to the appropriate vertex buffer. This exactly mirrors what `call_draw_webgpu` does today — it iterates layers sorted by z-index, then iterates the 3 sub-layers (0/1/2) within each.

### Where Fancy Tab Bar Fits in Frame

The `Frame` product type will have:

```rust
pub struct ChromeFrame {
    pub tab_bar: Option<RenderCommand>,  // Batch of commands from describe_element or describe_retro_tab_bar
    pub splits: Vec<RenderCommand>,
    pub borders: RenderCommand,          // Batch of FillRect commands
    pub modal: Option<RenderCommand>,    // Batch from describe_element
}
```

The fancy tab bar path: `build_fancy_tab_bar()` produces `ComputedElement`, then `describe_element()` translates it to `Vec<RenderCommand>`, which becomes `RenderCommand::Batch(commands)` stored in `ChromeFrame.tab_bar`.

The retro tab bar path: `describe_retro_tab_bar()` calls `describe_screen_line()` (the same function used for terminal lines), producing `Vec<RenderCommand>`, wrapped in `RenderCommand::Batch`.

---

## Question 3: The Multi-Pass Reallocation Loop

### Current Behavior

At `src/termwindow/render/paint.rs` lines 38–106:

```rust
'pass: for pass in 0.. {
    match self.paint_pass() {
        Ok(_) => {
            // Check if more quads were needed
            match self.render_state.as_mut().unwrap().allocated_more_quads() {
                Ok(allocated) => {
                    if !allocated { break 'pass; }
                    // Need more quads: invalidate caches, retry
                }
            }
        }
        Err(err) => {
            // OutOfTextureSpace: recreate/grow atlas, retry
            // ClearShapeCache: clear caches, bump generation, retry
            // Other: log and break
        }
    }
}
```

Three error conditions:

1. **OutOfTextureSpace:** The glyph cache atlas ran out of room. Solution: recreate atlas at `current_size` (pass 0) or grow to `requested size` (pass > 0). Then invalidate fancy tab bar and modal caches.
2. **ClearShapeCache:** Font resolution changed. Solution: clear shape caches, bump `shape_generation`, retry.
3. **`allocated_more_quads` returned true:** Quad vertex buffers needed to grow. Solution: invalidate fancy tab bar and modal caches, retry with larger buffers.

### Design: The Retry Loop Wraps the Full Hylomorphism

```rust
fn algebraic_paint_pass(&mut self) -> anyhow::Result<()> {
    // Clear quad allocation on all layers (same as today, paint.rs:184-189)
    {
        let gl_state = self.render_state.as_ref().unwrap();
        for layer in gl_state.layers.borrow().iter() {
            layer.clear_quad_allocation();
        }
    }
    self.ui_items.clear();

    // ANAMORPHISM: describe the frame
    let frame = self.describe_frame()?;

    // CATAMORPHISM: execute the frame into quads
    self.execute_frame(&frame)?;

    Ok(())
}
```

This replaces `paint_pass()` inside the existing retry loop in `paint_impl()`. The retry loop itself does not change.

### Why `describe_frame` Is Not Fully Pure

`describe_frame` calls into the glyph cache for texture lookups. Specifically:

- `build_line_element_shape()` (screen_line.rs:721) calls `gl_state.glyph_cache.borrow_mut().cached_line_sprite()` (line 762) for underline/strikethrough textures
- `cached_cluster_shape()` (mod.rs:782) calls `font.shape()` and `glyph_cache.cached_glyph()` for glyph textures
- `render_screen_line` calls `glyph_cache.cursor_sprite()` (screen_line.rs:410) for cursor textures
- `populate_image_quad` calls `glyph_cache.cached_image()` for image textures
- `render_background` (background.rs:429) calls `glyph_cache.cached_image()` for background images

All of these can fail with `OutOfTextureSpace` if the atlas is full.

**Decision:** Accept impurity for texture lookups. The describe phase takes a reference to the glyph cache and atlas, uses it to resolve glyphs to `TextureCoords`, and can fail with `OutOfTextureSpace`. The retry loop wraps the whole hylomorphism. This is pragmatic and matches the current architecture exactly.

The describe phase remains functionally pure in the following sense: for a given snapshot of (terminal state + glyph cache state), it produces deterministic output. The glyph cache is read-only from the describer's perspective — it reads texture coordinates out of it but does not modify atlas layout. The only mutation is lazy population of cache entries, which is idempotent.

### Alternative Considered and Rejected

A "two-level describe" approach would split describe into:

1. `describe_frame_logical` (pure: produces commands with glyph IDs, not texture coords)
2. `texturize_frame` (impure: resolves glyph IDs to `TextureCoords`)

This adds a third intermediate representation (logical commands with glyph IDs) and requires either a new enum variant or generic parameterization of `RenderCommand`. The complexity is not justified. The `OutOfTextureSpace` error path is already well-tested and works correctly with the retry loop.

---

## Question 4: Cache Granularity

**Recommendation:** Cache at the line level with `Vec<RenderCommand>` per line.

### Current Cache Architecture

**Level 1 — Shape cache** (`line_to_ele_shape_cache` at `src/termwindow/render/mod.rs` line 96–110):

- **Key:** `LineToEleShapeCacheKey { shape_hash, composing, shape_generation }`
- **Value:** `LineToElementShapeItem { expires, shaped: Rc<Vec<LineToElementShape>>, current_highlight, invalidate_on_hover_change }`
- This caches the DESCRIBE intermediate output: shaped glyph clusters with resolved colors, underline textures, and pixel widths.

**Level 2 — Quad cache** (`line_quad_cache` at mod.rs line 58–85):

- **Key:** `LineQuadCacheKey { config_generation, shape_generation, quad_generation, composing, selection, shape_hash, top_pixel_y, left_pixel_x, phys_line_idx, pane_id, pane_is_active, cursor, reverse_video, password_input }`
- **Value:** `LineQuadCacheValue { expires, layers: HeapQuadAllocator, current_highlight, invalidate_on_hover_change }`
- This caches the EXECUTE output: actual quads stored in `HeapQuadAllocator`.
- On cache hit, `cached_quad.layers.apply_to(self.layers)` replays the quads directly (see pane.rs line 462–464).

### New Cache Design

**Level 1 stays as-is.** The `line_to_ele_shape_cache` continues to cache the intermediate shape description. It is already correctly separated — it caches describe-phase output independent of cursor/selection/position.

**Level 2 becomes a command cache.** Replace:

```rust
// OLD
pub struct LineQuadCacheValue {
    pub expires: Option<Instant>,
    pub layers: HeapQuadAllocator,          // <-- execute-phase output
    pub current_highlight: Option<Arc<Hyperlink>>,
    pub invalidate_on_hover_change: bool,
}
```

with:

```rust
// NEW
pub struct LineCommandCacheValue {
    pub expires: Option<Instant>,
    pub commands: Vec<RenderCommand>,       // <-- describe-phase output
    pub current_highlight: Option<Arc<Hyperlink>>,
    pub invalidate_on_hover_change: bool,
}
```

The same `LineQuadCacheKey` (14 fields) stays as the key. On cache hit, instead of:

```rust
cached_quad.layers.apply_to(self.layers)?;  // old: replay quads
```

the executor does:

```rust
execute_commands(&cached_quad.commands, layers, pixel_dims)?;  // new: replay commands
```

### Why Line-Level Is the Right Granularity

Line-level is optimal because lines change independently. On a typical frame:

- 0–3 lines changed (cursor movement, new output, blink)
- Remaining lines are identical to previous frame
- Cache hit rate is typically 95–99% of lines

**Pane-level caching** would be wasteful. Any single line change invalidates the whole pane, forcing re-description of every line. A pane with 50 visible rows would lose its cache when 1 line changes.

**Frame-level caching** is useless. Almost every frame differs in some way. Cursor blink alone changes the cursor line every ~530ms (per `cursor_blink_rate`). Animated background shaders change every frame.

### Memory Analysis

A typical 80-column line produces approximately 20–40 `RenderCommand` instances:

- 1–10 `FillRect` for non-default backgrounds per cluster
- 1 `FillRect` for selection (if any)
- 1 `DrawQuad` for cursor (if on this line)
- 10–30 `DrawQuad` for glyph strips (after cursor/selection slicing)
- 0–5 `DrawQuad` for underlines
- 0–5 `DrawQuad` for images

Each `RenderCommand::DrawQuad` is approximately 120 bytes (layer + position rect + texture coords + fg_color + alt_color option + hsv option + mode). For 30 commands, that is approximately 3.6KB per line.

Compare with the current `HeapQuadAllocator`: each `BoxedQuad` is 84 bytes. For 30 quads across 3 layers, that is approximately 2.5KB per line.

The command cache is approximately 40% larger per entry, but the commands carry semantic information that enables future optimizations: command diffing between frames, GPU command buffer generation, partial re-execution.

### Cache Invalidation

Invalidation logic stays identical. The 14-field `LineQuadCacheKey` already captures every factor that could change the output: cursor position, selection range, config generation, shape generation, quad generation, position, composing text, pane activity, reverse video, password input. The `expires` field handles time-based invalidation (blinking text, animated images). The `invalidate_on_hover_change` flag handles hyperlink hover.

When the texture atlas is recreated (via `recreate_texture_atlas` at mod.rs line 863), all shape caches are cleared (`shape_cache`, `line_to_ele_shape_cache`). The command cache must also be cleared because the `TextureCoords` in cached `DrawQuad` commands reference the old atlas layout. Add `self.line_command_cache.borrow_mut().clear()` to `recreate_texture_atlas()`.

---

## Question 5: FrameObserver Design

### The State Surface That `describe_frame` Needs

I traced every field of `TermWindow` accessed during `paint_pass` and its transitive callees. The state breaks down into five categories:

**Category A — Window geometry (7 fields):**

- `dimensions: Dimensions` — pixel width, height, dpi
- `terminal_size: TerminalSize` — cols, rows, pixel dimensions
- `render_metrics: RenderMetrics` — cell_size, descender, underline_height, strikethrough_height
- `os_parameters: Option<parameters::Parameters>` — OS-specific border dimensions
- `show_tab_bar: bool`
- `show_scroll_bar: bool`
- `window_state: WindowState` — fullscreen, maximized flags

**Category B — Configuration (via `ConfigHandle`):**

Already has observer traits in `config/src/observers.rs`:

- `ColorConfigObserver` — inactive_pane_hsb, foreground_text_hsb, resolved_palette, bold_brightens_ansi_colors, colors
- `TextObserver` — text_background_opacity, text_blink_rate, text_blink_rate_rapid, custom_block_glyphs, experimental_pixel_positioning, use_box_model_render, text_min_contrast_ratio
- `WindowConfigObserver` — window_padding, window_frame, window_decorations, window_content_alignment, integrated_title_button_style, integrated_title_button_alignment
- `TabBarObserver` — tab_bar_at_bottom, use_fancy_tab_bar, show_close_tab_button_in_tabs
- `CursorObserver` — default_cursor_style, cursor_blink_rate, force_reverse_video_cursor, reverse_video_cursor_min_contrast
- `ScrollObserver` — min_scroll_bar_height
- `GpuObserver` — webgpu_shader_fps
- `TerminalFeaturesObserver` — detect_password_input, hyperlink_rules
- `BellObserver` — visual_bell settings

**Category C — Mux/pane state (accessed via mux and pane trait objects):**

- `get_panes_to_render() -> Vec<PositionedPane>` — pane positions within tab
- `get_splits() -> Vec<PositionedSplit>` — split positions within tab
- Per pane via `Arc<dyn Pane>`:
  - `pane.palette() -> ColorPalette`
  - `pane.get_cursor_position() -> StableCursorPosition`
  - `pane.get_dimensions() -> RenderableDimensions`
  - `pane.with_lines_mut(range, callback)` — line content access
  - `pane.get_metadata() -> Value` — password input detection
  - `pane.apply_hyperlinks(range, rules)` — hyperlink decoration
- Per pane via `TermWindow`:
  - `get_viewport(pane_id) -> Option<StableRowIndex>`
  - `selection(pane_id) -> SelectionState` (range + rectangular flag)

**Category D — Transient render state (10 fields):**

- `focused: Option<Instant>` — whether window has focus
- `dead_key_status: DeadKeyStatus` — composing state
- `current_highlight: Option<Arc<Hyperlink>>` — hovered hyperlink
- `current_mouse_event: Option<MouseEvent>` — for box model hover
- `current_mouse_capture: Option<MouseCapture>` — mouse capture mode
- `allow_images: AllowImage` — image rendering mode (Yes/Scale(n)/No)
- `window_background: Vec<LoadedBackgroundLayer>` — background images
- `cursor_blink_state: RefCell<ColorEase>` — cursor blink phase
- `blink_state: RefCell<ColorEase>` — text blink phase
- `rapid_blink_state: RefCell<ColorEase>` — rapid blink phase
- `prev_cursor: PrevCursorPos` — for blink reset on cursor movement

**Category E — GPU resources (impure, needed by describe):**

- `render_state: Option<RenderState>` — glyph cache, util sprites, texture atlas
- `fonts: Rc<FontConfiguration>` — font resolution

### Proposed Trait Design

Rather than one monolithic trait, use a compositional approach that mirrors the existing observer pattern:

```rust
// In phaedra-gui/src/observers.rs (extend existing file)

/// Provides read-only access to window geometry for the describe phase.
pub trait WindowGeometryObserver {
    fn dimensions(&self) -> &Dimensions;
    fn terminal_size(&self) -> &TerminalSize;
    fn render_metrics(&self) -> &RenderMetrics;
    fn os_border(&self) -> window::parameters::Border;
    fn show_tab_bar(&self) -> bool;
    fn show_scroll_bar(&self) -> bool;
    fn window_state(&self) -> WindowState;
    fn padding_left_top(&self) -> (f32, f32);
    fn tab_bar_pixel_height(&self) -> anyhow::Result<f32>;
}

/// Provides read-only access to pane layout for the describe phase.
pub trait PaneLayoutObserver {
    fn positioned_panes(&self) -> Vec<PositionedPane>;
    fn positioned_splits(&self) -> Vec<PositionedSplit>;
    fn viewport(&self, pane_id: PaneId) -> Option<StableRowIndex>;
    fn selection(&self, pane_id: PaneId) -> (Option<SelectionRange>, bool);
}

/// Provides read-only access to transient render state.
pub trait TransientRenderObserver {
    fn is_focused(&self) -> bool;
    fn dead_key_status(&self) -> &DeadKeyStatus;
    fn current_highlight(&self) -> Option<&Arc<Hyperlink>>;
    fn current_mouse_position(&self) -> Option<(f32, f32)>;
    fn current_mouse_capture(&self) -> Option<&MouseCapture>;
    fn allow_images(&self) -> AllowImage;
    fn has_window_background(&self) -> bool;
    fn window_background_layers(&self) -> &[LoadedBackgroundLayer];
}

/// The complete observer needed by describe_frame.
/// TermWindow implements this by delegating to its fields.
pub trait FrameObserver:
    WindowGeometryObserver
    + PaneLayoutObserver
    + TransientRenderObserver
    + FullConfigObserver  // from config/src/observers.rs
{}
```

### Why Not Pass Everything Through the Trait?

Two things cannot go through a read-only trait:

1. **Line content access.** Pane line content is accessed via `pane.with_lines_mut(stable_range, &mut callback)` which takes a `&mut impl WithPaneLines` callback. This requires mutable access to the callback and borrows the pane's internal line storage. The describe function for a line receives the `&Line` directly through this callback mechanism, just as today's `LineRender::render_line` does (pane.rs line 370–554).

2. **Glyph cache / texture atlas.** These are behind `RefCell` and require `borrow_mut` for lazy population. The describe function needs a `&RenderState` reference to access `gl_state.glyph_cache.borrow_mut()` and `gl_state.util_sprites`. This stays as a direct reference, not through the observer trait.

### Implementation of `FrameObserver` for `TermWindow`

```rust
// In phaedra-gui/src/observers.rs

impl WindowGeometryObserver for TermWindow {
    fn dimensions(&self) -> &Dimensions { &self.dimensions }
    fn terminal_size(&self) -> &TerminalSize { &self.terminal_size }
    fn render_metrics(&self) -> &RenderMetrics { &self.render_metrics }
    fn os_border(&self) -> window::parameters::Border {
        Self::get_os_border_impl(&self.os_parameters, &self.config, &self.dimensions, &self.render_metrics)
    }
    fn show_tab_bar(&self) -> bool { self.show_tab_bar }
    fn show_scroll_bar(&self) -> bool { self.show_scroll_bar }
    fn window_state(&self) -> WindowState { self.window_state }
    fn padding_left_top(&self) -> (f32, f32) { self.padding_left_top() }
    fn tab_bar_pixel_height(&self) -> anyhow::Result<f32> { self.tab_bar_pixel_height() }
}

impl PaneLayoutObserver for TermWindow {
    fn positioned_panes(&self) -> Vec<PositionedPane> { self.get_panes_to_render() }
    fn positioned_splits(&self) -> Vec<PositionedSplit> { /* delegate to get_splits */ }
    fn viewport(&self, pane_id: PaneId) -> Option<StableRowIndex> { self.get_viewport(pane_id) }
    fn selection(&self, pane_id: PaneId) -> (Option<SelectionRange>, bool) {
        let sel = self.selection(pane_id);
        (sel.range.clone(), sel.rectangular)
    }
}
// ... etc.
```

> **Note on `get_splits`:** The current `get_splits()` takes `&mut self` (mod.rs line 2922) because it calls `self.tab_state(tab_id)` which borrows `self.tab_state: RefCell<HashMap<TabId, TabState>>` mutably. For the observer trait, this must be refactored to take `&self` by using `borrow()` instead of `borrow_mut()` where possible, or by computing the splits from the mux directly without checking overlay state through `TermWindow`. This is a minor refactor required in Phase 2c.

### Testing Benefit

The trait-based design enables testing describe functions without a full `TermWindow`. A `MockFrameObserver` can provide synthetic state:

```rust
struct MockFrameObserver {
    dimensions: Dimensions,
    panes: Vec<PositionedPane>,
    // ... etc.
}
// Implement all observer traits with simple field access
```

This allows unit-testing `describe_frame` with known inputs and asserting exact `RenderCommand` output.

---

## Question 6: Phased Deployment

Minimum sequential phases. Given the dependency structure and the principle that each phase should produce a shippable PR that does not break existing behavior:

---

### Phase 0: Fix `RenderCommand::fold` Double-Count Bug (1 PR)

**File:** `src/render_command.rs`

The current fold at line 99–109:

```rust
pub fn fold<T, F>(&self, init: T, f: &F) -> T
where
    F: Fn(T, &RenderCommand) -> T,
{
    match self {
        RenderCommand::Batch(cmds) => {
            cmds.iter().fold(f(init, self), |acc, cmd| cmd.fold(acc, f))
            //              ^^^^^^^^^^^^^
            // BUG: applies f to the Batch node itself, THEN recurses into children.
            // This means Batch is double-counted: once as itself, once via children.
        }
        _ => f(init, self),
    }
}
```

Compare with the fixed `InputEffect::fold` at `src/input_effect.rs` line 208–216:

```rust
pub fn fold<T, F>(&self, init: T, f: &F) -> T
where
    F: Fn(T, &InputEffect) -> T,
{
    match self {
        InputEffect::Multiple(effects) => effects.iter().fold(init, |acc, e| e.fold(acc, f)),
        //                                               ^^^^
        // CORRECT: does NOT apply f to the Multiple node itself, only recurses into children.
        _ => f(init, self),
    }
}
```

**Fix:** Change `RenderCommand::fold` to NOT apply `f` to Batch nodes, only recurse into children. This matches the `InputEffect` pattern.

```rust
RenderCommand::Batch(cmds) => {
    cmds.iter().fold(init, |acc, cmd| cmd.fold(acc, f))  // no f(init, self)
}
```

**Risk:** Low. This is a leaf change with no callers depending on the double-count behavior (fold is currently unused).

---

### Phase 1: Extend `RenderCommand` and Add Frame Types (1–2 PRs)

**Files:**
- `src/render_command.rs` — extend enum
- New file: `src/frame.rs` — Frame product types

#### 1a. Extend `RenderCommand`

Add `Nop` variant:

```rust
pub enum RenderCommand {
    // ... existing variants ...
    Nop,
}
```

Add `zindex: i8` to `DrawQuad` and `FillRect`:

```rust
FillRect {
    zindex: i8,      // NEW: which z-index group (default 0, tab bar uses 10, backgrounds use -127)
    layer: usize,    // which sub-layer within the group (0=bg, 1=glyph, 2=overlay)
    rect: RectF,
    color: LinearRgba,
    hsv: Option<HsbTransform>,  // NEW: currently set after filled_rectangle returns
},
DrawQuad {
    zindex: i8,      // NEW
    layer: usize,
    position: RectF,
    texture: TextureCoords,
    fg_color: LinearRgba,
    alt_color: Option<(LinearRgba, f32)>,
    hsv: Option<HsbTransform>,
    mode: QuadMode,
},
```

Update `map_colors`, `fold`, `and_then` to handle the new variant.

#### 1b. Define Frame Product Types

```rust
// In phaedra-gui/src/frame.rs

use crate::render_command::RenderCommand;
use mux::pane::PaneId;

/// A complete frame description. The output of describe_frame (anamorphism).
/// The input to execute_frame (catamorphism).
#[derive(Debug, Clone)]
pub struct Frame {
    /// Window background (solid color or image background layers)
    pub background: Vec<RenderCommand>,
    /// Per-pane render data, in z-order
    pub panes: Vec<PaneFrame>,
    /// Chrome: tab bar, splits, borders, modal
    pub chrome: ChromeFrame,
    /// Post-processing parameters (for custom shaders)
    pub post_process: Option<PostProcessParams>,
    /// UI interaction items generated during description
    pub ui_items: Vec<UIItem>,
}

/// All render commands for a single pane.
#[derive(Debug, Clone)]
pub struct PaneFrame {
    pub pane_id: PaneId,
    pub is_active: bool,
    /// Pane background color fill
    pub background: Option<RenderCommand>,
    /// Visual bell overlay
    pub bell_overlay: Option<RenderCommand>,
    /// Scrollbar thumb
    pub scrollbar: Option<RenderCommand>,
    /// Per-line render commands, indexed by visible line offset
    pub lines: Vec<RenderCommand>,
}

/// Chrome elements that surround the terminal content.
#[derive(Debug, Clone)]
pub struct ChromeFrame {
    /// Tab bar (fancy or retro), including UI items
    pub tab_bar: Option<RenderCommand>,
    /// Pane split dividers
    pub splits: Vec<RenderCommand>,
    /// Window border fills
    pub borders: Vec<RenderCommand>,
    /// Modal overlay (command palette, pane selector, etc.)
    pub modal: Option<RenderCommand>,
}

/// Parameters for the post-processing shader pass.
#[derive(Debug, Clone)]
pub struct PostProcessParams {
    pub resolution: [f32; 2],
    pub time: f32,
}
```

**Risk:** Low. These are pure data types with no behavior yet. No existing code is modified.

---

### Phase 2: Extract Describe Functions (5–8 PRs, Partially Parallelizable)

This is the core work. Each sub-phase extracts a `describe_*` function from an existing imperative `paint_*` function.

#### Phase 2a: `describe_window_background` (1 PR)

**Source:** `src/termwindow/render/paint.rs` lines 207–268
**Target:** New function in a new file `src/termwindow/render/describe.rs`

Current logic:

1. Check if window has background images and images are allowed
2. If yes, call `render_backgrounds()` which loads images and renders them
3. If no (or images failed to load), emit a solid-color `FillRect` covering the whole window

Translates to:

```rust
pub fn describe_window_background(
    observer: &impl FrameObserver,
    panes: &[PositionedPane],
    gl_state: &RenderState,
    allow_images: AllowImage,
) -> anyhow::Result<Vec<RenderCommand>> {
    let mut commands = vec![];
    // Same branching logic as paint.rs lines 210-268
    // Instead of calling filled_rectangle, emit RenderCommand::FillRect
    // Instead of calling render_backgrounds, emit background image DrawQuad commands
    Ok(commands)
}
```

The background image rendering (at `src/termwindow/background.rs` line 415–553) currently writes directly to vertex buffers via `layer0.map()`. This must be translated to emit `DrawQuad` commands with `zindex: -127` (matching the current `layer_for_zindex(-127)`).

*Can be parallelized with: 2b, 2c*

---

#### Phase 2b: `describe_window_borders` (1 PR)

**Source:** `src/termwindow/render/borders.rs` lines 8–79
**Target:** Function in `src/termwindow/render/describe.rs`

This is trivial. The current function checks border dimensions, then emits up to 4 `filled_rectangle` calls for top/left/bottom/right borders. Each becomes a `FillRect` command.

```rust
pub fn describe_window_borders(
    observer: &impl FrameObserver,
) -> Vec<RenderCommand> {
    let border = observer.os_border();
    let dims = observer.dimensions();
    let config = observer.config();
    let mut commands = vec![];
    // 4 conditional FillRect commands, same logic as borders.rs
    commands
}
```

*Can be parallelized with: 2a, 2c*

---

#### Phase 2c: `describe_splits` (1 PR)

**Source:** `src/termwindow/render/split.rs` lines 9–82
**Target:** Function in `src/termwindow/render/describe.rs`

Also trivial. For each `PositionedSplit`, emit one `FillRect` on layer 2 (overlay). The current function also pushes `UIItem` entries — these must be collected into the `Frame.ui_items`.

```rust
pub fn describe_split(
    split: &PositionedSplit,
    pane_palette: &ColorPalette,
    observer: &impl FrameObserver,
) -> (RenderCommand, UIItem) {
    // Same geometry math as split.rs
    // Return (FillRect command, UIItem for mouse interaction)
}
```

> **Refactor note:** `get_splits()` currently takes `&mut self` (mod.rs:2922). For the observer pattern, change this to `&self` by using `self.tab_state.borrow()` instead of the mutable accessor when checking overlay state.

*Can be parallelized with: 2a, 2b*

---

#### Phase 2d: `describe_screen_line` — THE CRITICAL PATH (2–3 PRs)

**Source:** `src/termwindow/render/screen_line.rs` lines 27–719
**Target:** New function `describe_screen_line` alongside the existing `render_screen_line`

This is the hardest phase because the 700-line function deeply interleaves description and execution. The split must happen in sub-steps:

##### Phase 2d-i: Extract Background and Underline Description (1 PR)

Lines 173–283 of screen_line.rs. Currently:

```rust
// Reverse video background fill (lines 173-188)
if params.dims.reverse_video {
    let mut quad = self.filled_rectangle(layers, 0, rect, params.foreground)?;
    quad.set_hsv(hsv);
}

// Per-cluster background fills (lines 198-259)
for item in shaped.iter() {
    if !bg_is_default {
        let mut quad = self.filled_rectangle(layers, 0, rect, bg_color)?;
        quad.set_hsv(hsv);
    }
    // Underlines (lines 262-282)
    if item.underline_tex_rect != params.white_space {
        for i in 0..cluster_width {
            let mut quad = layers.allocate(0)?;
            quad.set_position(...);
            quad.set_texture(item.underline_tex_rect);
            quad.set_fg_color(item.underline_color);
        }
    }
}

// Selection background (lines 288-305)
if !params.selection.is_empty() {
    let mut quad = self.filled_rectangle(layers, 0, rect, params.selection_bg)?;
    quad.set_hsv(hsv);
}
```

Each of these becomes a `RenderCommand::FillRect` or `RenderCommand::DrawQuad`. The key transformation:

- `self.filled_rectangle(layers, layer_num, rect, color)` with `.set_hsv(hsv)` after becomes `RenderCommand::FillRect { zindex: 0, layer: layer_num, rect, color, hsv }`
- `layers.allocate(0)` with manual `.set_position`, `.set_texture`, `.set_fg_color`, `.set_hsv`, `.set_has_color` becomes `RenderCommand::DrawQuad { zindex: 0, layer: 0, position, texture, fg_color, alt_color: None, hsv, mode: QuadMode::Glyph }`

##### Phase 2d-ii: Extract Cursor Description (1 PR)

Lines 307–421 of screen_line.rs. The cursor rendering computes `ComputeCellFgBgResult` (which determines cursor shape, colors, and blend values), then:

1. Allocates a quad on the appropriate layer (layer 0 for block/underline, layer 2 for bar)
2. If password input, resolves the lock glyph texture and positions it
3. Otherwise, resolves the cursor sprite texture and positions it
4. Sets `fg_color` and `alt_color_and_mix_value` for blinking

This becomes a single `RenderCommand::DrawQuad` with the cursor texture coords and colors.

##### Phase 2d-iii: Extract Glyph Strip Description (1 PR)

Lines 424–698 of screen_line.rs. This is the most complex part — the multi-strip slicing logic for cursor/selection overlay on glyphs.

The current code:

1. Iterates shaped items (clusters)
2. For each glyph in the cluster, resolves its texture
3. Handles custom block glyphs (substitutes block glyph textures)
4. Computes `texture_range` (pixel range of the glyph texture)
5. Uses `range3()` to split the texture into strips against cursor and selection ranges
6. For each non-empty strip, calls `compute_cell_fg_bg` to determine the strip's foreground color
7. Computes the sub-texture coordinates for the strip
8. Allocates a quad on layer 1 with the strip's position, texture, and color

Each strip becomes a `RenderCommand::DrawQuad { zindex: 0, layer: 1, ... }`.

The below-text images (`z_index < 0`) become `DrawQuad { zindex: 0, layer: 0, ... }`.
The overlay images (`z_index >= 0`) become `DrawQuad { zindex: 0, layer: 2, ... }`.

##### Combining into `describe_screen_line`

```rust
pub fn describe_screen_line(
    &self,
    params: RenderScreenLineParams,
) -> anyhow::Result<(Vec<RenderCommand>, RenderScreenLineResult)> {
    let mut commands = vec![];

    // Phase 1: Shape resolution (reuses existing build_line_element_shape / cache)
    let (shaped, invalidate_on_hover_change) = /* same as today */;

    // Phase 2: Background commands
    describe_line_backgrounds(&params, &shaped, &mut commands);

    // Phase 3: Selection command
    describe_line_selection(&params, &mut commands);

    // Phase 4: Cursor command
    describe_line_cursor(&self, &params, &mut commands)?;

    // Phase 5: Glyph strip commands
    describe_line_glyphs(&self, &params, &shaped, &mut commands)?;

    // Phase 6: Image commands
    describe_line_images(&self, &params, &shaped, &mut commands)?;

    Ok((commands, RenderScreenLineResult { invalidate_on_hover_change }))
}
```

*Depends on: Phase 1 (needs extended RenderCommand types)*

---

#### Phase 2e: `describe_pane` (1 PR)

**Source:** `src/termwindow/render/pane.rs` lines 33–583
**Target:** New function `describe_pane` that produces a `PaneFrame`

This wraps `describe_screen_line` for all visible lines, adds:

- Pane background fill (lines 154–173, `FillRect`)
- Visual bell overlay (lines 175–222, `FillRect`)
- Scrollbar (lines 229–289, `FillRect` + `UIItems`)

The per-line rendering at lines 305–572 currently uses the `LineRender` struct that implements `WithPaneLines`. The new version will be structurally identical but call `describe_screen_line` instead of `render_screen_line`, and store results in `PaneFrame.lines` instead of writing to `TripleLayerQuadAllocator`.

Cache lookup/store logic (lines 445–552) moves to wrapping `describe_screen_line` output: on cache miss, call `describe_screen_line`, cache the `Vec<RenderCommand>`, and return it. On cache hit, return the cached `Vec<RenderCommand>`.

*Depends on: Phase 2d*

---

#### Phase 2f: `describe_tab_bar` (1–2 PRs)

**Source:** `src/termwindow/render/tab_bar.rs` (retro path) and `src/termwindow/render/fancy_tab_bar.rs` (fancy path)

**Retro tab bar path:** `paint_tab_bar` (tab_bar.rs:11–101) calls `render_screen_line` with a synthetic `RenderableDimensions` and the tab bar's `Line`. In the algebraic pipeline, it calls `describe_screen_line` instead, producing `Vec<RenderCommand>`. Plus the `UIItem` computation from `tab_bar.compute_ui_items()`.

**Fancy tab bar path:** `paint_fancy_tab_bar` (fancy_tab_bar.rs:462–473) calls `render_element` on the cached `ComputedElement`. In the algebraic pipeline, it calls `describe_element` instead. The `build_fancy_tab_bar` function (fancy_tab_bar.rs:59–460) stays as-is — it produces `ComputedElement`, which is already a description.

*Depends on: Phase 2d (for retro path via `describe_screen_line`)*

---

#### Phase 2g: `describe_element` (1 PR)

**Source:** `src/termwindow/box_model.rs` lines 826–946
**Target:** New function `describe_element` that returns `Vec<RenderCommand>`

As detailed in Question 2, this is a mechanical translation of `render_element`:

- Hover color resolution stays
- Background rendering becomes `FillRect` commands
- Text content becomes `DrawQuad` commands
- Children recurse
- Poly content becomes `DrawQuad` commands

*Can be parallelized with: Phase 2d (no dependency between them)*

---

#### Phase 2h: `describe_modal` (1 PR, trivial)

**Source:** `src/termwindow/render/paint.rs` lines 168–181

The current `paint_modal` calls `modal.computed_element(self)?` then `render_element`. In the algebraic pipeline, it calls `describe_element` on the computed element.

*Depends on: Phase 2g*

---

#### Phase 2i: `describe_frame` — Composition (1 PR)

**Source:** `src/termwindow/render/paint.rs` lines 183–299
**Target:** New function `describe_frame` that composes all `describe_*` functions into a `Frame`

```rust
pub fn describe_frame(&self) -> anyhow::Result<Frame> {
    let panes = self.get_panes_to_render();

    // Side effects that must happen during description
    // (today these happen in paint_pass)
    for pos in &panes {
        if pos.is_active {
            self.update_text_cursor(pos);
            if self.focused.is_some() {
                pos.pane.advise_focus();
                mux::Mux::get().record_focus_for_current_identity(pos.pane.pane_id());
            }
        }
    }

    let background = describe_window_background(self, &panes, gl_state, self.allow_images)?;

    let mut pane_frames = vec![];
    for pos in &panes {
        pane_frames.push(self.describe_pane(pos)?);
    }

    let splits = if let Some(pane) = self.get_active_pane_or_overlay() {
        self.get_splits().iter().map(|s| describe_split(s, &pane.palette(), self)).collect()
    } else {
        vec![]
    };

    let tab_bar = if self.show_tab_bar {
        Some(self.describe_tab_bar()?)
    } else {
        None
    };

    let borders = describe_window_borders(self);
    let modal = self.describe_modal()?;

    Ok(Frame {
        background,
        panes: pane_frames,
        chrome: ChromeFrame {
            tab_bar,
            splits,
            borders,
            modal,
        },
        post_process: self.webgpu.as_ref().and_then(|w| {
            if w.has_postprocess() {
                Some(PostProcessParams { /* ... */ })
            } else {
                None
            }
        }),
        ui_items: vec![],  // populated during description
    })
}
```

*Depends on: All of 2a through 2h*

---

### Phase 3: Build `execute_frame` Catamorphism (1–2 PRs)

**Target:** New file `src/execute_render.rs`

The catamorphism walks a `Frame` and translates each `RenderCommand` to quad mutations via the `QuadTrait` API.

```rust
use crate::frame::{Frame, PaneFrame, ChromeFrame};
use crate::render_command::{RenderCommand, QuadMode};
use crate::quad::{QuadTrait, TripleLayerQuadAllocator, TripleLayerQuadAllocatorTrait};
use crate::renderstate::RenderState;

/// The catamorphism: consumes a Frame, writes quads to GPU vertex buffers.
pub fn execute_frame(
    frame: &Frame,
    render_state: &RenderState,
    pixel_dims: (f32, f32),
) -> anyhow::Result<()> {
    execute_commands_on_zindex(&frame.background, render_state, pixel_dims, 0)?;

    for pane_frame in &frame.panes {
        execute_pane_frame(pane_frame, render_state, pixel_dims)?;
    }

    execute_chrome_frame(&frame.chrome, render_state, pixel_dims)?;

    Ok(())
}

fn execute_pane_frame(
    pf: &PaneFrame,
    render_state: &RenderState,
    pixel_dims: (f32, f32),
) -> anyhow::Result<()> {
    if let Some(ref bg) = pf.background {
        execute_command(bg, render_state, pixel_dims)?;
    }
    if let Some(ref bell) = pf.bell_overlay {
        execute_command(bell, render_state, pixel_dims)?;
    }
    for line_cmd in &pf.lines {
        execute_command(line_cmd, render_state, pixel_dims)?;
    }
    if let Some(ref sb) = pf.scrollbar {
        execute_command(sb, render_state, pixel_dims)?;
    }
    Ok(())
}

/// Translate a single RenderCommand into quad mutations.
fn execute_command(
    cmd: &RenderCommand,
    render_state: &RenderState,
    pixel_dims: (f32, f32),
) -> anyhow::Result<()> {
    match cmd {
        RenderCommand::Nop => Ok(()),

        RenderCommand::Clear { .. } => {
            // Handled at the GPU level in call_draw_webgpu, not here
            Ok(())
        }

        RenderCommand::FillRect { zindex, layer, rect, color, hsv } => {
            let render_layer = render_state.layer_for_zindex(*zindex)?;
            let mut layers = render_layer.quad_allocator();
            let mut quad = layers.allocate(*layer)?;
            let left_offset = pixel_dims.0 / 2.0;
            let top_offset = pixel_dims.1 / 2.0;
            quad.set_position(
                rect.min_x() - left_offset,
                rect.min_y() - top_offset,
                rect.max_x() - left_offset,
                rect.max_y() - top_offset,
            );
            let gl_state = render_state;
            quad.set_texture(gl_state.util_sprites.filled_box.texture_coords());
            quad.set_is_background();
            quad.set_fg_color(*color);
            quad.set_hsv(hsv.clone().map(|h| config::HsbTransform {
                hue: h.hue,
                saturation: h.saturation,
                brightness: h.brightness,
            }));
            Ok(())
        }

        RenderCommand::DrawQuad {
            zindex, layer, position, texture, fg_color, alt_color, hsv, mode,
        } => {
            let render_layer = render_state.layer_for_zindex(*zindex)?;
            let mut layers = render_layer.quad_allocator();
            let mut quad = layers.allocate(*layer)?;
            quad.set_position(
                position.min_x(), position.min_y(),
                position.max_x(), position.max_y(),
            );
            quad.set_texture(/* convert TextureCoords to TextureRect */);
            quad.set_fg_color(*fg_color);
            if let Some((alt, mix)) = alt_color {
                quad.set_alt_color_and_mix_value(*alt, *mix);
            }
            quad.set_hsv(/* convert */);
            match mode {
                QuadMode::Glyph => quad.set_has_color(false),
                QuadMode::ColorEmoji => quad.set_has_color(true),
                QuadMode::BackgroundImage => quad.set_is_background_image(),
                QuadMode::SolidColor => quad.set_is_background(),
                QuadMode::GrayScale => quad.set_grayscale(),
            }
            Ok(())
        }

        RenderCommand::SetClipRect(_) => Ok(()),
        RenderCommand::BeginPostProcess => Ok(()),

        RenderCommand::Batch(cmds) => {
            for cmd in cmds {
                execute_command(cmd, render_state, pixel_dims)?;
            }
            Ok(())
        }
    }
}
```

Key design decisions in the executor:

1. The executor accesses `render_state.layer_for_zindex()` to find the correct vertex buffer. This is the same mechanism used today.
2. The executor needs `pixel_dims` to compute the GL coordinate offset (`pixel_width / -2.0`, `pixel_height / -2.0`). Today this is done inline in `filled_rectangle` (mod.rs:266–288) and `render_screen_line` (screen_line.rs:65–66).
3. The executor needs access to `util_sprites.filled_box.texture_coords()` for `FillRect`. This comes from `render_state`.
4. The `DrawQuad` position values are in GL coordinates (already offset by `-pixel_width/2`, `-pixel_height/2`). This means the describe phase must compute GL coordinates. Alternatively, the describe phase can use screen coordinates and the executor does the offset. **Decision:** describe in screen coordinates, execute applies the offset. This keeps the describe phase in intuitive screen-space coordinates and centralizes the GL transform in the executor.

*Depends on: Phase 2i (needs the Frame types populated)*

---

### Phase 4: Wire Up the Hylomorphism (1 PR)

**File:** `src/termwindow/render/paint.rs`

Add a temporary runtime gate in `paint_pass` to branch:

```rust
pub fn paint_pass(&mut self) -> anyhow::Result<()> {
    {
        let gl_state = self.render_state.as_ref().unwrap();
        for layer in gl_state.layers.borrow().iter() {
            layer.clear_quad_allocation();
        }
    }
    self.ui_items.clear();

    if enable_algebraic_path {
        let frame = self.describe_frame()?;
        let render_state = self.render_state.as_ref().unwrap();
        let pixel_dims = (
            self.dimensions.pixel_width as f32,
            self.dimensions.pixel_height as f32,
        );
        execute_frame(&frame, render_state, pixel_dims)?;
        self.ui_items = frame.ui_items;
    } else {
        // ... existing imperative paint_pass body ...
    }

    Ok(())
}
```

The `paint_impl` retry loop, `call_draw_webgpu`, and animation scheduling remain untouched. Both paths produce the same output: quads in vertex buffers ready for `call_draw_webgpu`.

*Depends on: Phase 3*

---

### Phase 5: Replace `line_quad_cache` (1 PR)

**Files:**
- `src/termwindow/render/mod.rs` (cache types)
- `src/termwindow/render/pane.rs` (cache usage)

Change `LineQuadCacheValue`:

```rust
// OLD
pub struct LineQuadCacheValue {
    pub expires: Option<Instant>,
    pub layers: HeapQuadAllocator,
    pub current_highlight: Option<Arc<Hyperlink>>,
    pub invalidate_on_hover_change: bool,
}

// NEW
pub struct LineQuadCacheValue {
    pub expires: Option<Instant>,
    pub commands: Vec<RenderCommand>,
    pub current_highlight: Option<Arc<Hyperlink>>,
    pub invalidate_on_hover_change: bool,
}
```

In pane.rs, the cache hit path (lines 445–468) changes from:

```rust
// OLD (pane.rs:461-464)
cached_quad.layers.apply_to(self.layers)?;
```

to:

```rust
// NEW
execute_commands(&cached_quad.commands, render_state, pixel_dims)?;
```

And the cache miss path (lines 470–552) changes from writing to `HeapQuadAllocator` and caching it, to calling `describe_screen_line` and caching the `Vec<RenderCommand>`.

Also add `self.line_quad_cache.borrow_mut().clear()` to `recreate_texture_atlas()` (mod.rs:863) to invalidate command caches when atlas layout changes.

*Depends on: Phase 4 (needs `execute_commands` working)*

---

### Phase 6: Delete Old Imperative Path (1 PR)

**Files to modify:**

- `src/termwindow/render/paint.rs` — remove the `else` branch in `paint_pass`
- `src/termwindow/render/screen_line.rs` — remove `render_screen_line` (the quad-writing version), keep `build_line_element_shape` and `describe_screen_line`
- `src/termwindow/render/pane.rs` — remove `paint_pane` (the quad-writing version), keep `describe_pane`
- `src/termwindow/render/split.rs` — remove `paint_split`, keep `describe_split`
- `src/termwindow/render/borders.rs` — remove `paint_window_borders`, keep `describe_window_borders`
- `src/termwindow/render/tab_bar.rs` — remove `paint_tab_bar`, keep `describe_tab_bar`
- `src/termwindow/box_model.rs` — remove `render_element`, keep `describe_element`
- `src/quad.rs` — `HeapQuadAllocator` can be simplified or removed since the command cache replaces it
- Config/runtime: remove the temporary gate, algebraic path is now the only path

*Depends on: Phase 5, plus sufficient validation time with the algebraic path enabled*

---

### Phase 7: Extract `phaedra-render-command` Crate (1–2 PRs)

See Question 7 for full details.

**Phase 7a:** Create `phaedra-render-command/` with:
- `Cargo.toml` with minimal dependencies (`euclid`, `window::color`)
- `src/lib.rs` re-exporting the types
- `src/command.rs` — `RenderCommand`, `QuadMode`, `HsbTransform`, `TextureCoords`, `RectF`, `PointF`
- `src/frame.rs` — `Frame`, `PaneFrame`, `ChromeFrame`, `PostProcessParams`
- `src/algebra.rs` — `map_colors`, `fold`, `and_then`, `count_commands`

**Phase 7b:** Update `phaedra-gui/Cargo.toml` to depend on `phaedra-render-command`. Move type definitions from `phaedra-gui/src/render_command.rs` and `phaedra-gui/src/frame.rs` into the new crate. Re-export from `phaedra-gui` for backward compatibility.

*Depends on: Phase 6*

---

### Summary Timeline

```
Phase 0 ─────────────────────────── Fix fold bug
Phase 1 ─────────────────────────── Add types
Phase 2a ─┐
Phase 2b ─┼── parallelizable ───── Simple describe functions
Phase 2c ─┘
Phase 2d ─────────────────────────── describe_screen_line (CRITICAL PATH, 2-3 PRs)
Phase 2e ─────────────────────────── describe_pane (depends on 2d)
Phase 2f ─────────────────────────── describe_tab_bar (depends on 2d)
Phase 2g ─────────────────────────── describe_element (parallelizable with 2d)
Phase 2h ─────────────────────────── describe_modal (depends on 2g)
Phase 2i ─────────────────────────── describe_frame (depends on all 2*)
Phase 3 ──────────────────────────── execute_frame catamorphism
Phase 4 ──────────────────────────── Wire hylomorphism + config flag
Phase 5 ──────────────────────────── Replace line_quad_cache
Phase 6 ──────────────────────────── Delete old path
Phase 7 ──────────────────────────── Extract crate
```

**Total:** 14–19 PRs. The critical path goes through Phases 0 → 1 → 2d → 2e → 2i → 3 → 4 → 5 → 6 → 7. Phases 2a/2b/2c and 2g can be parallelized alongside 2d.

---

## Question 7: Crate Boundary

### The Dependency Problem

The describe functions need types from multiple crates:

- **Terminal types:** `phaedra_term::Line`, `phaedra_term::CellAttributes`, `phaedra_term::color::ColorAttribute`, `phaedra_term::StableRowIndex`
- **Font types:** `phaedra_font::LoadedFont`, `phaedra_font::GlyphInfo`, `phaedra_font::ClearShapeCache`
- **Window/GPU types:** `window::bitmaps::TextureRect`, `window::color::LinearRgba`, `window::Dimensions`
- **Config types:** `config::ConfigHandle`, `config::HsbTransform`, `config::TextStyle`
- **Mux types:** `mux::pane::Pane`, `mux::tab::PositionedPane`, `mux::renderable::RenderableDimensions`
- **wgpu types:** via `TripleLayerQuadAllocator`, `Vertex`, `QuadTrait`

The execute functions need:

- **wgpu types:** All quad allocation types, `Vertex`, `RenderState`, `WebGpuState`
- **Window types:** `LinearRgba`, `TextureRect`

### The Clean Boundary: `phaedra-render-command` as a Leaf Crate

```
phaedra-render-command (NEW, leaf)
    depends on: euclid, window::color (for LinearRgba)
    contains:   RenderCommand, Frame, PaneFrame, ChromeFrame,
                TextureCoords, HsbTransform, QuadMode, RectF, PointF,
                PostProcessParams, map_colors, fold, and_then

phaedra-gui (EXISTING, modified)
    depends on: phaedra-render-command, phaedra-term, phaedra-font,
                config, mux, window, wgpu, ...
    contains:   FrameObserver traits, all describe_* functions,
                execute_frame, execute_commands, caching logic,
                paint_impl retry loop, call_draw_webgpu
```

### What Goes in `phaedra-render-command`

This crate is tiny and has minimal dependencies. Its purpose is to define the algebra.

**`phaedra-render-command/src/command.rs`:**

```rust
use window::color::LinearRgba;

pub type RectF = euclid::default::Rect<f32>;
pub type PointF = euclid::default::Point2D<f32>;

#[derive(Debug, Clone)]
pub enum QuadMode { Glyph, ColorEmoji, BackgroundImage, SolidColor, GrayScale }

#[derive(Debug, Clone)]
pub struct HsbTransform { pub hue: f32, pub saturation: f32, pub brightness: f32 }

#[derive(Debug, Clone)]
pub struct TextureCoords { pub left: f32, pub top: f32, pub right: f32, pub bottom: f32 }

#[derive(Debug, Clone)]
pub enum RenderCommand {
    Clear { color: LinearRgba },
    FillRect { zindex: i8, layer: usize, rect: RectF, color: LinearRgba, hsv: Option<HsbTransform> },
    DrawQuad { zindex: i8, layer: usize, position: RectF, texture: TextureCoords,
               fg_color: LinearRgba, alt_color: Option<(LinearRgba, f32)>,
               hsv: Option<HsbTransform>, mode: QuadMode },
    SetClipRect(Option<RectF>),
    BeginPostProcess,
    Batch(Vec<RenderCommand>),
    Nop,
}
```

**`phaedra-render-command/src/algebra.rs`:**

```rust
impl RenderCommand {
    pub fn map_colors<F>(self, f: &F) -> RenderCommand
    where F: Fn(LinearRgba) -> LinearRgba { /* ... */ }

    pub fn fold<T, F>(&self, init: T, f: &F) -> T
    where F: Fn(T, &RenderCommand) -> T { /* ... */ }

    pub fn and_then<F>(self, f: F) -> RenderCommand
    where F: FnOnce(RenderCommand) -> RenderCommand { /* ... */ }

    pub fn count(&self) -> usize { self.fold(0, &|acc, _| acc + 1) }

    pub fn is_nop(&self) -> bool { matches!(self, RenderCommand::Nop) }
}
```

**`phaedra-render-command/src/frame.rs`:**

```rust
use crate::command::RenderCommand;

// Note: PaneFrame.pane_id uses a simple u32 alias, NOT mux::pane::PaneId,
// to avoid depending on the mux crate.
pub type PaneId = u32;

#[derive(Debug, Clone)]
pub struct Frame {
    pub background: Vec<RenderCommand>,
    pub panes: Vec<PaneFrame>,
    pub chrome: ChromeFrame,
    pub post_process: Option<PostProcessParams>,
}

#[derive(Debug, Clone)]
pub struct PaneFrame { pub pane_id: PaneId, /* ... */ }

#[derive(Debug, Clone)]
pub struct ChromeFrame { /* ... */ }

#[derive(Debug, Clone)]
pub struct PostProcessParams { pub resolution: [f32; 2], pub time: f32 }
```

**`phaedra-render-command/Cargo.toml`:**

```toml
[package]
name = "phaedra-render-command"
version = "0.1.0"
edition = "2021"

[dependencies]
euclid.workspace = true
window = { workspace = true }  # only for LinearRgba
```

### What Stays in `phaedra-gui`

Everything else:

- **FrameObserver traits** (in `phaedra-gui/src/observers.rs`) — references `ConfigHandle`, `PositionedPane`, `Dimensions`, `WindowState`, `DeadKeyStatus`, `MouseEvent`, etc.
- **All `describe_*` functions** — these need `phaedra-term` (`Line`, `CellAttributes`), `phaedra-font` (shaping, glyph cache), `window::bitmaps` (`TextureRect`), `config` (`ConfigHandle`), `mux` (`PositionedPane`, `Pane` trait).
- **`execute_frame` / `execute_commands`** — these need `TripleLayerQuadAllocator`, `QuadTrait`, `Vertex`, `RenderState`, all of which depend on wgpu.
- **Caching logic** — `line_to_ele_shape_cache`, `line_quad_cache` (renamed to `line_command_cache`), `shape_cache`.
- **`paint_impl` orchestration** — the retry loop, `call_draw_webgpu`.
- **Box model** — `ComputedElement`, `compute_element`, `Element`. These stay in `phaedra-gui` since they reference font types and glyph sprites.

### Future Extraction Path

If later a headless renderer or alternative GPU backend is needed:

1. **Extract `phaedra-render-describe` crate:**
   - Contains `FrameObserver` traits and `describe_*` functions
   - Depends on `phaedra-render-command`, `phaedra-term`, `phaedra-font`, `config`, `mux`, `window`
   - Does NOT depend on wgpu

2. **Extract `phaedra-render-wgpu` crate:**
   - Contains `execute_frame`, `execute_commands`, quad allocation, `call_draw_webgpu`
   - Depends on `phaedra-render-command`, `wgpu`
   - Does NOT depend on `phaedra-term`, `phaedra-font`, `mux`

This two-crate extraction is not needed now but the algebraic boundary makes it possible later.

---

## Appendix A: TextureCoords Invalidation

`RenderCommand::DrawQuad` contains `TextureCoords` which are normalized coordinates into the glyph atlas texture. If the atlas is recreated (due to `OutOfTextureSpace` in the retry loop), all cached `TextureCoords` become invalid.

This is handled by the existing invalidation cascade in `recreate_texture_atlas` (at `src/termwindow/render/mod.rs` line 863–871):

```rust
pub fn recreate_texture_atlas(&mut self, size: Option<usize>) -> anyhow::Result<()> {
    self.shape_generation += 1;
    self.shape_cache.borrow_mut().clear();
    self.line_to_ele_shape_cache.borrow_mut().clear();
    if let Some(render_state) = self.render_state.as_mut() {
        render_state.recreate_texture_atlas(&self.fonts, &self.render_metrics, size)?;
    }
    Ok(())
}
```

The new command cache (`line_command_cache`) must be added to this invalidation:

```rust
self.line_command_cache.borrow_mut().clear();  // ADD THIS
```

Plus `invalidate_fancy_tab_bar()` and `invalidate_modal()` are already called in the retry loop (paint.rs:68–69, 96–97).

No additional work is needed beyond adding the one cache clear call.

---

## Appendix B: Side Effects During Description

The current `paint_pass` performs several side effects that are interleaved with rendering:

1. `update_text_cursor(&pos)` (paint.rs:272) — updates OS-level cursor position for IME
2. `pos.pane.advise_focus()` (paint.rs:274) — tells the pane it has focus
3. `Mux::get().record_focus_for_current_identity()` (paint.rs:275) — records focus for workspace tracking
4. `check_for_dirty_lines_and_invalidate_selection()` (pane.rs:42) — checks for dirty lines
5. `pane.apply_hyperlinks(range, rules)` (pane.rs:312) — applies hyperlink rules to visible lines

These are not rendering operations — they are state maintenance that happens to be co-located with rendering. In the algebraic pipeline, these must be extracted and run before `describe_frame`:

```rust
pub fn pre_render_maintenance(&mut self) {
    let panes = self.get_panes_to_render();
    let focused = self.focused.is_some();
    for pos in &panes {
        if pos.is_active {
            self.update_text_cursor(pos);
            if focused {
                pos.pane.advise_focus();
                mux::Mux::get().record_focus_for_current_identity(pos.pane.pane_id());
            }
        }
        self.check_for_dirty_lines_and_invalidate_selection(&pos.pane);
        // apply_hyperlinks happens per-pane during describe_pane
    }
}
```

This is called at the top of `paint_pass` before `describe_frame`.

---

## Appendix C: UIItem Collection

The current `paint_pass` collects `UIItem` entries (mouse interaction regions) as a side effect of rendering. For example:

- `paint_pane` pushes scrollbar `UIItems` (pane.rs:253–277)
- `paint_split` pushes split divider `UIItems` (split.rs:44–78)
- `paint_tab_bar` pushes tab bar `UIItems` (tab_bar.rs:35–39)
- `paint_modal` appends modal `UIItems` (paint.rs:176)

In the algebraic pipeline, `describe_*` functions return `UIItems` alongside `RenderCommands`. The `Frame` struct has a `ui_items: Vec<UIItem>` field. After `describe_frame` completes, `self.ui_items = frame.ui_items`.

Alternatively, each `describe_*` function returns `(Vec<RenderCommand>, Vec<UIItem>)` tuples, and `describe_frame` concatenates all `UIItem` vectors into the `Frame`.

---

## Summary

| Question | Answer |
|----------|--------|
| 1. Incremental strategy | Dual-path with config flag at `paint_pass` level. Old + new run independently. No big-bang. |
| 2. Box model | `render_element` becomes `describe_element` returning `Vec<RenderCommand>`. `ComputedElement` is already a description. Z-index encoded in commands via new `zindex: i8` field. |
| 3. Multi-pass reallocation | Retry loop wraps the full hylomorphism unchanged. Describe phase accepts impurity for texture atlas lookups. `OutOfTextureSpace` propagates as today. |
| 4. Cache granularity | Line-level. Replace `HeapQuadAllocator` in cache with `Vec<RenderCommand>`. Same 14-field cache key. 95–99% hit rate unchanged. |
| 5. FrameObserver | Compositional trait hierarchy: `WindowGeometryObserver` + `PaneLayoutObserver` + `TransientRenderObserver` + `FullConfigObserver`. `TermWindow` implements all. Line content accessed via existing `WithPaneLines` callback, not through the trait. |
| 6. Phased deployment | 7 major phases, 14–19 PRs. Critical path through Phase 2d (`describe_screen_line`). Phases 2a/2b/2c and 2g parallelizable. Config flag in Phase 4, delete old path in Phase 6. |
| 7. Crate boundary | `phaedra-render-command` (leaf crate): `RenderCommand` enum + `Frame` types + algebraic operations. Depends only on `euclid` + `window::color`. Everything else stays in `phaedra-gui`. Future two-crate split (describe/execute) enabled but deferred. |

## Critical Files for Implementation

- **`src/termwindow/render/screen_line.rs`** — The 905-line hot path where describe/execute interleaving is tightest; must be split into `describe_screen_line` + kept `build_line_element_shape` (Phase 2d, the hardest phase)
- **`src/render_command.rs`** — The free algebra definition; must fix fold bug, add `Nop` variant, add `zindex` to `FillRect`/`DrawQuad`, add `hsv` to `FillRect` (Phases 0–1)
- **`src/termwindow/render/paint.rs`** — The `paint_pass` orchestrator and `paint_impl` retry loop; will be rewired to call `describe_frame`/`execute_frame` with dual-path config flag (Phase 4)
- **`src/termwindow/render/mod.rs`** — Contains `LineQuadCacheKey`/`Value`, `RenderScreenLineParams`, `ComputeCellFgBgParams`, and all shared render types; the cache must migrate from `HeapQuadAllocator` to `Vec<RenderCommand>` (Phase 5)
- **`src/termwindow/box_model.rs`** — Contains `render_element` (line 826) and `render_element_background` (line 1010) which must become `describe_element` to support fancy tab bar and modals through the algebraic pipeline (Phase 2g)
