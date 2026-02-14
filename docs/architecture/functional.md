# Phaedra — Functional Architecture

> *"The bright one. Born from WezTerm's ashes. Rebuilt with algebraic precision."*

---

## Premise

Phaedra inherits a 393K-line Rust codebase from WezTerm. Beneath the surface: 158K lines of actual logic, three god objects, a global mutable singleton, 42+ latent panics via `.unwrap()`, and a custom derive macro system that reinvents serde. The platform has been narrowed to macOS-only with a single wgpu rendering backend.

This document describes how to transform WezTerm's accumulative architecture into Phaedra's compositional one. The decomposition follows three algebraic layers that solve three distinct problems: *how config is loaded*, *how config is consumed*, and *how config is introspected*.

---

## I. The Three Algebraic Layers

### Layer 1 — Hylomorphic Resolution (Loading)

Config loading is an unfold-then-fold: a **hylomorphism**.

```
                         ana (unfold)
  ┌─────────┐     ┌──────────────────┐     ┌───────────────────┐
  │ Sources  │ ──► │ Vec<PartialConfig>│ ──► │  ResolvedConfig   │
  └─────────┘     └──────────────────┘     └───────────────────┘
   defaults                                       cata (fold)
   .phaedra.lua                              merge with precedence
   env vars
   CLI args
   Lua overrides
```

**Anamorphism (unfold):** Each config source is parsed into a `PartialConfig` — a sparse representation containing *only the fields the user explicitly set*. The current codebase conflates "not set" with "default value," making it impossible to distinguish user intent from defaults.

**Catamorphism (fold):** Partial configs are merged with explicit precedence:

```
defaults < file < env < CLI < Lua runtime overrides
```

Each layer only overwrites fields it explicitly contains. The merge strategy is a monoid: `PartialConfig` has an identity (empty) and an associative binary operation (merge-with-precedence).

**Current state:** `config/src/config.rs` line ~959 (`load_config`) performs loading as a single imperative procedure. No separation between unfolding sources and folding layers. No `PartialConfig` type exists — everything deserializes directly into the full `Config` struct with defaults pre-filled, destroying sparsity information.

**Files involved:**
| File | Role | Lines |
|------|------|------:|
| `config/src/config.rs:959-1050` | Config file discovery and loading | ~90 |
| `config/src/config.rs:1060-1200` | Lua evaluation and override application | ~140 |
| `config/src/lib.rs:92-150` | `toml_to_dynamic()`, `json_to_dynamic()` conversion | ~60 |
| `config/src/lib.rs:300-450` | `reload()`, config generation management | ~150 |
| `config/src/lua.rs:192-370` | Lua environment setup, module registration | ~180 |

---

### Layer 2 — Coalgebraic Observation (Consumption)

The rest of the system does not construct Config. It **observes** it. Config is defined by its observations, not its constructors. This is the coalgebraic dual.

**Current state:** Every consumer grabs the full 253-field Config via `configuration()` and projects out the 1-4 fields it actually needs. 40 files call `configuration()`. Only ONE crosses more than 3 config domains — and that one (`config/src/terminal.rs`) is already an intentional adapter.

**Target state:** Each subsystem declares an observation trait:

```
FontConfigView ◄── font system observes font fields
ColorConfigView ◄── renderer observes color fields
KeyConfigView ◄── input handler observes key fields
         ▲
         │
    Config implements all views
```

The dependency arrow inverts. Consumers don't depend on Config; Config depends on consumer-defined traits. This is the **Port** in hexagonal architecture — the consumer defines the port, Config is the adapter.

**Consumer coupling analysis (top 10 files by `configuration()` calls):**

| File | Calls | Fields Accessed | Domains Crossed |
|------|------:|-----------------|----------------:|
| `config/src/terminal.rs` | 16 | scrollback_lines, enable_kitty_*, enq_answerback, canonicalize_pasted_*, etc. | 4 |
| `phaedra-gui/src/customglyph.rs` | 7 | anti_alias_custom_block_glyphs | 1 |
| `mux/src/localpane.rs` | 7 | exit_behavior, exit_behavior_messaging, log_unknown_escape_sequences | 2 |
| `phaedra-gui/src/shapecache.rs` | 6 | (none — passes config to setup functions) | 0 |
| `phaedra-gui/src/main.rs` | 6 | (passes whole config downstream) | 2 |
| `phaedra-gui/src/frontend.rs` | 6 | allow_download_protocols, quit_when_all_windows_are_closed | 2 |
| `mux/src/lib.rs` | 6 | exit_behavior, mux_enable_ssh_agent, mux_output_parser_* | 2 |
| `phaedra-mux-server/src/main.rs` | 5 | mux_env_remove | 1 |
| `phaedra-gui/src/update.rs` | 4 | check_for_updates, check_for_updates_interval_seconds | 1 |
| `phaedra-gui/src/termwindow/mod.rs` | 4 | (passes config to FontConfiguration) | 1 |

**Key finding:** The coupling surface is narrow. Most consumers touch 1-2 domains. Decomposition will not require rewriting the world.

---

### Layer 3 — Optics for Introspection (Hot-Reload + Lua + Validation)

Lenses provide typed, bidirectional paths into the config tree:

```
Lens<Config, FontConfig>           — focus on the font sub-record
Lens<FontConfig, f64>              — focus on font_size within font
Lens<Config, f64>                  — composed: config → font → size
```

**Three consumers of optics:**

1. **Hot-reload change detection:** Each sub-record carries a generation counter. When config reloads, only sub-records whose generation changed trigger downstream effects. "Font config changed" reloads fonts. "Color config changed" rebuilds palette. No more "something changed somewhere, reinitialize everything."

2. **Lua binding generation:** The Lua API presents a flat namespace (`config.font_size = 14`). Optics route flat key access to the correct sub-record via `__index`/`__newindex` metatables. Users see simplicity; the implementation is structured.

3. **Validation and introspection:** `ConfigMeta` already generates field-level metadata (name, doc, type, default). Extending this with domain tags and lens paths gives you a complete schema — enough to auto-generate documentation, config editors, and validation error messages.

**Existing infrastructure:**

| Component | Location | Status |
|-----------|----------|--------|
| `ConfigHandle` (Arc + generation) | `config/src/lib.rs:775-811` | Exists, functional |
| `ConfigMeta` derive macro | `config/derive/src/` | Exists, partial |
| Subscriber-based hot-reload | `config/src/lib.rs:300-450` | Exists, functional |
| Field metadata generation | `config/src/meta.rs` | Exists, TODOs remain |
| Lua config environment | `config/src/lua.rs:192-370` | Exists, needs restructuring |

---

## II. The Config Decomposition

### Field Inventory

178 public fields on the `Config` struct, clustered by domain:

#### Font (21 fields)
| Field | Type |
|-------|------|
| `font_size` | `f64` |
| `font` | `TextStyle` |
| `font_rules` | `Vec<StyleRule>` |
| `font_dirs` | `Vec<PathBuf>` |
| `font_locator` | `FontLocatorSelection` |
| `font_rasterizer` | `FontRasterizerSelection` |
| `font_colr_rasterizer` | `FontRasterizerSelection` |
| `font_shaper` | `FontShaperSelection` |
| `display_pixel_geometry` | `DisplayPixelGeometry` |
| `freetype_load_target` | `FreeTypeLoadTarget` |
| `freetype_render_target` | `Option<FreeTypeLoadTarget>` |
| `freetype_load_flags` | `Option<FreeTypeLoadFlags>` |
| `freetype_interpreter_version` | `Option<u32>` |
| `freetype_pcf_long_family_names` | `bool` |
| `harfbuzz_features` | `Vec<String>` |
| `dpi` | `Option<f64>` |
| `dpi_by_screen` | `HashMap<String, f64>` |
| `allow_square_glyphs_to_overflow_width` | `AllowSquareGlyphOverflow` |
| `ignore_svg_fonts` | `bool` |
| `sort_fallback_fonts_by_coverage` | `bool` |
| `search_font_dirs_for_fallback` | `bool` |
| `use_cap_height_to_scale_fallback_fonts` | `bool` |
| `char_select_font` | `Option<TextStyle>` |
| `char_select_font_size` | `f64` |
| `command_palette_font` | `Option<TextStyle>` |
| `command_palette_font_size` | `f64` |
| `pane_select_font` | `Option<TextStyle>` |
| `pane_select_font_size` | `f64` |

#### Window (18 fields)
| Field | Type |
|-------|------|
| `window_decorations` | `WindowDecorations` |
| `integrated_title_buttons` | `Vec<IntegratedTitleButton>` |
| `integrated_title_button_alignment` | `IntegratedTitleButtonAlignment` |
| `integrated_title_button_style` | `IntegratedTitleButtonStyle` |
| `integrated_title_button_color` | `IntegratedTitleButtonColor` |
| `window_frame` | `WindowFrameConfig` |
| `window_padding` | `WindowPadding` |
| `window_content_alignment` | `WindowContentAlignment` |
| `window_close_confirmation` | `WindowCloseConfirmation` |
| `initial_rows` | `u16` |
| `initial_cols` | `u16` |
| `macos_window_background_blur` | `i64` |
| `native_macos_fullscreen_mode` | `bool` |
| `macos_fullscreen_extend_behind_notch` | `bool` |
| `adjust_window_size_when_changing_font_size` | `Option<bool>` |
| `use_resize_increments` | `bool` |
| `unzoom_on_switch_pane` | `bool` |
| `quit_when_all_windows_are_closed` | `bool` |
| `enable_zwlr_output_manager` | `bool` |
| `tiling_desktop_environments` | `Vec<String>` |
| `win32_system_backdrop` | `SystemBackdrop` |
| `win32_acrylic_accent_color` | `RgbaColor` |

#### Text Rendering (18 fields)
| Field | Type |
|-------|------|
| `line_height` | `f64` |
| `cell_width` | `f64` |
| `underline_thickness` | `Option<Dimension>` |
| `underline_position` | `Option<Dimension>` |
| `strikethrough_position` | `Option<Dimension>` |
| `custom_block_glyphs` | `bool` |
| `anti_alias_custom_block_glyphs` | `bool` |
| `text_background_opacity` | `f32` |
| `text_min_contrast_ratio` | `Option<f32>` |
| `text_blink_rate` | `u64` |
| `text_blink_ease_in` | `EasingFunction` |
| `text_blink_ease_out` | `EasingFunction` |
| `text_blink_rate_rapid` | `u64` |
| `text_blink_rapid_ease_in` | `EasingFunction` |
| `text_blink_rapid_ease_out` | `EasingFunction` |
| `normalize_output_to_unicode_nfc` | `bool` |
| `bidi_enabled` | `bool` |
| `bidi_direction` | `ParagraphDirectionHint` |
| `experimental_pixel_positioning` | `bool` |
| `use_box_model_render` | `bool` |
| `warn_about_missing_glyphs` | `bool` |
| `canonicalize_pasted_newlines` | `Option<NewlineCanon>` |
| `unicode_version` | `u8` |
| `treat_east_asian_ambiguous_width_as_wide` | `bool` |
| `cell_widths` | `Option<Vec<CellWidth>>` |

#### Keys & Input (17 fields)
| Field | Type |
|-------|------|
| `keys` | `Vec<Key>` |
| `key_tables` | `HashMap<String, Vec<Key>>` |
| `leader` | `Option<LeaderKey>` |
| `disable_default_key_bindings` | `bool` |
| `debug_key_events` | `bool` |
| `send_composed_key_when_left_alt_is_pressed` | `bool` |
| `send_composed_key_when_right_alt_is_pressed` | `bool` |
| `macos_forward_to_ime_modifier_mask` | `Modifiers` |
| `treat_left_ctrlalt_as_altgr` | `bool` |
| `swap_backspace_and_delete` | `bool` |
| `use_ime` | `bool` |
| `xim_im_name` | `Option<String>` |
| `ime_preedit_rendering` | `ImePreeditRendering` |
| `use_dead_keys` | `bool` |
| `enable_csi_u_key_encoding` | `bool` |
| `key_map_preference` | `KeyMapPreference` |
| `ui_key_cap_rendering` | `UIKeyCapRendering` |
| `launcher_alphabet` | `String` |

#### Color (16 fields)
| Field | Type |
|-------|------|
| `colors` | `Option<Palette>` |
| `resolved_palette` | `Palette` |
| `color_scheme` | `Option<String>` |
| `color_schemes` | `HashMap<String, Palette>` |
| `color_scheme_dirs` | `Vec<PathBuf>` |
| `bold_brightens_ansi_colors` | `BoldBrightening` |
| `foreground_text_hsb` | `HsbTransform` |
| `inactive_pane_hsb` | `HsbTransform` |
| `background` | `Vec<BackgroundLayer>` |
| `char_select_fg_color` | `RgbaColor` |
| `char_select_bg_color` | `RgbaColor` |
| `command_palette_fg_color` | `RgbaColor` |
| `command_palette_bg_color` | `RgbaColor` |
| `pane_select_fg_color` | `RgbaColor` |
| `pane_select_bg_color` | `RgbaColor` |

#### Tab Bar (11 fields)
| Field | Type |
|-------|------|
| `tab_bar_style` | `TabBarStyle` |
| `enable_tab_bar` | `bool` |
| `use_fancy_tab_bar` | `bool` |
| `tab_bar_at_bottom` | `bool` |
| `mouse_wheel_scrolls_tabs` | `bool` |
| `show_tab_index_in_tab_bar` | `bool` |
| `show_tabs_in_tab_bar` | `bool` |
| `show_new_tab_button_in_tab_bar` | `bool` |
| `show_close_tab_button_in_tabs` | `bool` |
| `tab_and_split_indices_are_zero_based` | `bool` |
| `tab_max_width` | `usize` |
| `hide_tab_bar_if_only_one_tab` | `bool` |
| `switch_to_last_active_tab_when_closing_tab` | `bool` |

#### Mouse & Selection (11 fields)
| Field | Type |
|-------|------|
| `mouse_bindings` | `Vec<Mouse>` |
| `disable_default_mouse_bindings` | `bool` |
| `bypass_mouse_reporting_modifiers` | `Modifiers` |
| `selection_word_boundary` | `String` |
| `quick_select_patterns` | `Vec<String>` |
| `quick_select_alphabet` | `String` |
| `quick_select_remove_styling` | `bool` |
| `disable_default_quick_select_patterns` | `bool` |
| `hide_mouse_cursor_when_typing` | `bool` |
| `swallow_mouse_click_on_pane_focus` | `bool` |
| `swallow_mouse_click_on_window_focus` | `bool` |
| `pane_focus_follows_mouse` | `bool` |
| `quote_dropped_files` | `DroppedFileQuoting` |

#### Cursor (10 fields)
| Field | Type |
|-------|------|
| `cursor_thickness` | `Option<Dimension>` |
| `cursor_blink_rate` | `u64` |
| `cursor_blink_ease_in` | `EasingFunction` |
| `cursor_blink_ease_out` | `EasingFunction` |
| `default_cursor_style` | `DefaultCursorStyle` |
| `force_reverse_video_cursor` | `bool` |
| `reverse_video_cursor_min_contrast` | `f32` |
| `xcursor_theme` | `Option<String>` |
| `xcursor_size` | `Option<u32>` |

#### Launch & Spawn (10 fields)
| Field | Type |
|-------|------|
| `default_prog` | `Option<Vec<String>>` |
| `default_gui_startup_args` | `Vec<String>` |
| `default_cwd` | `Option<PathBuf>` |
| `launch_menu` | `Vec<SpawnCommand>` |
| `exit_behavior` | `ExitBehavior` |
| `exit_behavior_messaging` | `ExitBehaviorMessaging` |
| `clean_exit_codes` | `Vec<u32>` |
| `set_environment_variables` | `HashMap<String, String>` |
| `prefer_to_spawn_tabs` | `bool` |
| `term` | `String` |
| `default_workspace` | `Option<String>` |
| `command_palette_rows` | `Option<usize>` |
| `skip_close_confirmation_for_processes_named` | `Vec<String>` |

#### Domain & Networking (9 fields)
| Field | Type |
|-------|------|
| `exec_domains` | `Vec<ExecDomain>` |
| `unix_domains` | `Vec<UnixDomain>` |
| `ssh_domains` | `Option<Vec<SshDomain>>` |
| `ssh_backend` | `SshBackend` |
| `tls_servers` | `Vec<TlsDomainServer>` |
| `tls_clients` | `Vec<TlsDomainClient>` |
| `mux_enable_ssh_agent` | `bool` |
| `default_ssh_auth_sock` | `Option<String>` |
| `mux_env_remove` | `Vec<String>` |
| `default_domain` | `Option<String>` |
| `default_mux_server_domain` | `Option<String>` |

#### Scroll (4 fields)
| Field | Type |
|-------|------|
| `scrollback_lines` | `usize` |
| `enable_scroll_bar` | `bool` |
| `min_scroll_bar_height` | `Dimension` |
| `scroll_to_bottom_on_input` | `bool` |
| `alternate_buffer_wheel_scroll_speed` | `u8` |

#### Bell (2 fields)
| Field | Type |
|-------|------|
| `visual_bell` | `VisualBell` |
| `audible_bell` | `AudibleBell` |

#### Update (2 fields)
| Field | Type |
|-------|------|
| `check_for_updates` | `bool` |
| `check_for_updates_interval_seconds` | `u64` |

#### Hyperlinks (1 field)
| Field | Type |
|-------|------|
| `hyperlink_rules` | `Vec<hyperlink::Rule>` |

#### Performance & Runtime (remaining ~28 fields)
| Field | Type | Sub-category |
|-------|------|-------------|
| `front_end` | `FrontEndSelection` | GPU |
| `webgpu_power_preference` | `WebGpuPowerPreference` | GPU |
| `webgpu_force_fallback_adapter` | `bool` | GPU |
| `webgpu_preferred_adapter` | `Option<GpuInfo>` | GPU |
| `webgpu_shader` | `Option<PathBuf>` | GPU |
| `webgpu_shader_fps` | `u8` | GPU |
| `max_fps` | `u64` | Performance |
| `animation_fps` | `u8` | Performance |
| `shape_cache_size` | `usize` | Cache |
| `line_state_cache_size` | `usize` | Cache |
| `line_quad_cache_size` | `usize` | Cache |
| `line_to_ele_shape_cache_size` | `usize` | Cache |
| `glyph_cache_image_cache_size` | `usize` | Cache |
| `ratelimit_mux_line_prefetches_per_second` | `u32` | Mux tuning |
| `mux_output_parser_buffer_size` | `usize` | Mux tuning |
| `mux_output_parser_coalesce_delay_ms` | `u64` | Mux tuning |
| `daemon_options` | `DaemonOptions` | Runtime |
| `log_unknown_escape_sequences` | `bool` | Debug |
| `detect_password_input` | `bool` | Security |
| `enable_kitty_graphics` | `bool` | Protocol |
| `enable_kitty_keyboard` | `bool` | Protocol |
| `enable_title_reporting` | `bool` | Protocol |
| `allow_download_protocols` | `bool` | Protocol |
| `allow_win32_input_mode` | `bool` | Protocol |
| `notification_handling` | `NotificationHandling` | Protocol |
| `enq_answerback` | `String` | Protocol |
| `automatically_reload_config` | `bool` | Runtime |
| `periodic_stat_logging` | `u64` | Debug |
| `status_update_interval` | `u64` | Runtime |
| `ulimit_nofile` | `u64` | Runtime |
| `ulimit_nproc` | `u64` | Runtime |
| `palette_max_key_assigments_for_action` | `usize` | Runtime |

---

## III. God Object Inventory

### God Object 1: Config (253 fields, 2,140 lines)

**Location:** `config/src/config.rs:51-2140`

**Decomposition strategy:** Product factoring into ~12 sub-records (see Section II), with coalgebraic observation traits (see Layer 2) and optic-based introspection (see Layer 3).

### God Object 2: Tab (43 public methods, 2,528 lines)

**Location:** `mux/src/tab.rs:40-2528`

**What it manages:**
- Pane tree structure (binary tree of splits)
- Zoom state (single pane expanded to fill tab)
- Recency tracking (which pane was last active)
- Size calculations (distributing pixels across panes)
- Pane lifecycle (add, remove, rotate, swap)

**Decomposition target:** Extract `PaneTree` (owns the binary tree), `ZoomController` (zoom state machine), `SizeAllocator` (pixel distribution algorithm). Tab becomes a facade that coordinates these three.

**Duplication found:** Three similar resize implementations at lines 1139, 1261, and 1377 with structural overlap.

### God Object 3: Mux (59 public methods, global singleton)

**Location:** `mux/src/lib.rs:1-1466`

**The singleton pattern:**
```rust
lazy_static! {
    static ref MUX: Mutex<Option<Arc<Mux>>> = Mutex::new(None);
}
```

**What it manages:** Windows, tabs, panes, domains, clients, activities, workspaces, notifications — all behind a single global mutex.

**Decomposition target:** Dependency injection. Create `WindowManager`, `DomainRegistry`, `ActivityTracker` as injected services. Mux becomes a coordinator that holds references, not a god that holds everything.

---

## IV. Architectural Weaknesses

### Severity 5 — Critical

| Issue | Location | Impact |
|-------|----------|--------|
| Config god object (253 fields) | `config/src/config.rs:51` | Every subsystem depends on one struct |
| Tab god object (43 methods) | `mux/src/tab.rs:40` | Pane management is untestable |

### Severity 4 — High

| Issue | Location | Impact |
|-------|----------|--------|
| Mux global singleton | `mux/src/lib.rs:19` | Untestable, thread-unsafe design |
| 42+ `.unwrap()` calls | `tab.rs`, `lib.rs`, `config/lib.rs` | Latent panics in production |
| 12+ lazy_static globals in config | `config/src/lib.rs:64-82` | Hidden mutable state, test pollution |

### Severity 3 — Medium

| Issue | Location | Impact |
|-------|----------|--------|
| Stringly-typed domain names | `keyassignment.rs`, `exec_domain.rs` | No compile-time validation |
| Inconsistent error handling | Multiple crates | Mix of anyhow, unwrap, Option |
| `downcast-rs` on Pane and Domain traits | `mux/pane.rs:167`, `domain.rs:49` | Runtime type checks defeat type safety |
| phaedra-dynamic reimplements serde | `phaedra-dynamic/` | ~2,400 lines of redundant infrastructure |
| Mux crate has 36 dependencies | `mux/Cargo.toml` | Impossible to test in isolation |

---

## V. Existing Infrastructure

### ConfigHandle

```
config/src/lib.rs:775-811

ConfigHandle {
    config: Arc<Config>,     // Thread-safe shared ownership
    generation: usize,       // Monotonic generation counter
}
```

The generation counter enables cheap change detection: compare generations instead of diffing the entire config. This mechanism survives decomposition — each sub-record gains its own generation counter for fine-grained change tracking.

### ConfigMeta Derive

```
config/derive/src/

Generates: ConfigMeta trait
    → get_config_options() -> &'static [ConfigOption]
    → ConfigOption { name, doc, type_name, default_value, container }
```

Embryonic optics. Extend with:
- Domain tags (which sub-record does this field belong to?)
- Lens path (typed accessor into the config tree)
- Validation rules (range constraints, enum variants)
- Lua key mapping (flat namespace → structured access)

### Hot-Reload Subscription

```
config/src/lib.rs:300-450

subscribe_to_config_reload(F) -> ConfigSubscription
    F: Fn() -> bool + Send
    Drop: auto-unsubscribe
```

Currently notifies "config changed" without specifying *what* changed. After decomposition, notifications become domain-specific: "font config changed," "color config changed," etc.

### Lua Config Environment

```
config/src/lua.rs:192-370

- Registers `wezterm` module (rename to `phaedra` pending)
- Sets up package.path for Lua module loading
- Provides font(), font_with_fallback(), color functions
- Config file discovery: .phaedra.lua, phaedra.lua
```

The Lua module currently exposes a flat namespace. After decomposition, `__index`/`__newindex` metatables route flat key access (`config.font_size`) to the appropriate sub-record (`config.font.size`). Users see no change; the implementation gains structure.

---

## VI. The Decomposition Protocol

Each sub-record extraction follows the same five-step protocol:

### Step 1: Factor the Sub-Record

Create a new struct containing the clustered fields. The parent Config struct holds an instance of it.

### Step 2: Define the Observation Trait

The consumers of those fields define a trait expressing what they need. This is the coalgebraic interface — the consumer declares its observations.

### Step 3: Implement the Trait on Config

Config (or ConfigHandle) implements the observation trait by delegating to the sub-record. Consumers now depend on the trait, not on Config.

### Step 4: Add Generation Tracking

The sub-record gains a generation counter. Hot-reload notifications become domain-specific. Only the subsystems that observe the changed domain need to react.

### Step 5: Wire the Lua Metatable

The Lua config table's `__index`/`__newindex` metatables are extended to route the affected keys to the new sub-record. The user-facing API remains flat.

---

## VII. The Comment Audit

### Summary

11,609 comment lines across the codebase. Breakdown:

| Category | Count | Verdict |
|----------|------:|---------|
| Doc comments (///) | 6,215 | Keep on public API; strip obvious restatements |
| Module docs (//!) | 468 | Keep |
| TODO/FIXME/HACK | 107 | **Keep all** — this is the debt map |
| Explanation of WHY | ~800-1,000 | **Keep all** — irreplaceable context |
| Links/references | ~244 | **Keep all** — external context |
| Noise (restates code) | ~500-600 | Strip |
| Dead/stale | ~50-100 | Strip |
| License headers | ~50 | Keep (legally required) |
| Section markers | ~20-40 | Keep |

**Action:** Strip ~2,000 lines of noise/dead comments. Preserve ~9,600 lines of TODOs, WHYs, links, and useful API docs.

### The 107 Known Wounds (TODO/FIXME/HACK)

Every TODO and FIXME in the codebase, by area:

**Config (6):**
- `config/src/config.rs:1926` — `FIXME: also allow deserializing from bool`
- `config/src/config.rs:1938` — `TODO: something smart where we see whether the`
- `config/src/meta.rs:22` — `TODO: tags to categorize the option`
- `config/src/meta.rs:29` — `TODO: For enum types, the set of possible values`
- `config/src/meta.rs:31` — `TODO: For struct types, the fields in the child struct`
- `config/src/ssh.rs:24,41` — `TODO: Tmux-cc`, `TODO: Cmd, PowerShell`

**Mux (9):**
- `mux/src/lib.rs:1117,1189` — `TODO: disambiguate with TabId`
- `mux/src/lib.rs:1233,1240,1389` — `FIXME: clipboard`, `FIXME: split pane pixel dimensions`, `FIXME: clipboard?`
- `mux/src/localpane.rs:879` — `TODO: do we need to proactively list available tabs here?`
- `mux/src/ssh.rs:266` — `FIXME: this isn't useful without a way to talk to the remote mux`
- `mux/src/ssh.rs:565` — `TODO: TerminalWaker assumes SystemTerminal`
- `mux/src/termwiztermtab.rs:113,441,530` — `FIXME: connect to something?`, `TODO: TerminalWaker`, `TODO: make a singleton`
- `mux/src/tmux.rs:287` — `TODO: pass session here`

**GUI (15):**
- `phaedra-gui/src/commands.rs:243,273` — `FIXME: use domain_label here, but needs to be async`
- `phaedra-gui/src/commands.rs:855,866,877,888,899` — `FIXME: find a new assignment` (5 instances)
- `phaedra-gui/src/frontend.rs:127` — `FIXME: if notification.focus is true`
- `phaedra-gui/src/frontend.rs:212` — `TODO: arrange for this to happen on config reload`
- `phaedra-gui/src/scripting/guiwin.rs:130` — `FIXME: expose other states here`
- `phaedra-gui/src/tabbar.rs:188` — `TODO: Decide what to do here to indicate this`
- `phaedra-gui/src/termwindow/keyevent.rs:751` — `TODO: consider eliminating these codes`
- `phaedra-gui/src/termwindow/render/*` — 5 TODOs across fancy_tab_bar, mod, pane, screen_line

**Terminal Emulation (10):**
- `term/src/screen.rs:261,305` — `FIXME: borrow bidi mode from line`
- `term/src/terminalstate/kitty.rs:47` — `FIXME: make this configurable`
- `term/src/terminalstate/kitty.rs:750` — `TODO: send an EINVAL error back`
- `term/src/terminalstate/mod.rs:141,1260` — `TODO: selective_erase`, `TODO: see DECSTR spec`
- `term/src/test/c0.rs:13,36` — `TODO: when we can set the left margin`
- `term/src/test/mod.rs:9` — `FIXME: port to render layer`

**Escape Parser (5):**
- `phaedra-escape-parser/src/csi.rs:213` — `TODO: data size optimization opportunity`
- `phaedra-escape-parser/src/esc.rs:133` — `TODO: data size optimization opportunity`
- `phaedra-escape-parser/src/hyperlink.rs:104,110` — `TODO: protect against k, v containing : or =`, `TODO: ensure link.uri doesn't contain characters`
- `phaedra-escape-parser/src/parser/sixel.rs:117` — `FIXME: from linear`

**Font (4):**
- `phaedra-font/src/hbwrap.rs:442,480` — `TODO: pass a callback for querying custom palette colors`, `TODO: hb_paint_funcs_set_custom_palette_color_func`
- `phaedra-font/src/rasterizer/freetype.rs:545,592` — `FIXME: gradient vectors are expressed as font units`, `FIXME: harfbuzz, in COLR.hh, pushes the inverse`

**Client (4):**
- `phaedra-client/src/client.rs:202` — `FIXME: We currently get a bunch of these`
- `phaedra-client/src/domain.rs:282` — `TODO: advice remote host of interesting workspaces`
- `phaedra-client/src/pane/clientpane.rs:480,549` — `TODO: decide how to handle key_up for mux client`, `FIXME: retrieve this from the remote`

**SSH (3):**
- `phaedra-ssh/src/sessioninner.rs:183,705` — `FIXME: overridden config path?`, `TODO: Move this somewhere to avoid re-allocating buffer`
- `phaedra-ssh/tests/e2e/sftp.rs:511` — `TODO: This fails even though the type is a symlink`

**Input Types (3):**
- `phaedra-input-types/src/lib.rs:1777,1920,1944` — `TODO: Hyper and Meta`, `FIXME: ideally we'd get the correct unshifted key`

**Surface (3):**
- `phaedra-surface/src/change.rs:68` — `TODO: check iterm rendering behavior`
- `phaedra-surface/src/line/line.rs:539,675,1006` — `FIXME: let's build a string`, `TODO: look back for cells hidden by`, `FIXME: we can skip the look-back`

**Termwiz (12):**
- `termwiz/src/input.rs:279,657,667` — `TODO: also respect self.application_keypad`, `TODO: we could report caps lock`, `TODO: do we want downs instead of ups?`
- `termwiz/src/lineedit/mod.rs:576,631` — `TODO: there's no way we can match anything`
- `termwiz/src/render/terminfo.rs:23,397,419,547,602,624` — multiple rendering TODOs
- `termwiz/src/terminal/windows.rs:735,742,755,781` — Windows terminal TODOs (dead code on macOS)
- `termwiz/src/widgets/mod.rs:342,453` — `TODO: synthesize double`, `TODO: garbage collect unreachable WidgetId's`

**Window (3):**
- `window/src/os/macos/window.rs:141,1792,1802` — `FIXME: CTRL-C normalization`, `FIXME: docs say to insert the text here`, `FIXME: returns NSArray`

---

## VIII. Residual Branding

### Remaining `wezterm` References (55 across 12 files)

**Needs fixing (10 refs — stray URLs):**
| File | Line | Content |
|------|-----:|---------|
| `phaedra-gui/src/commands.rs` | 1674 | `https://wezterm.org/` |
| `phaedra-gui/src/commands.rs` | 2140 | `https://wezterm.org/` |
| `phaedra-gui/src/update.rs` | 95 | `https://wezterm.org/changelog.html` |
| `phaedra-gui/src/update.rs` | 194 | `https://wezterm.org/changelog.html` |
| `phaedra-font/src/lib.rs` | 418 | `https://wezterm.org/config/fonts.html` |
| `phaedra-font/src/lib.rs` | 852 | `https://wezterm.org/config/fonts.html` |
| `mux/src/localpane.rs` | 269 | `https://wezterm.org/config/lua/config/exit_behavior.html` |
| `config/src/tls.rs` | 93 | `https://wezterm.org/config/lua/pane/get_metadata.html` |
| `config/src/ssh.rs` | 79 | `https://wezterm.org/config/lua/pane/get_metadata.html` |
| `config/src/unix.rs` | 61 | `https://wezterm.org/config/lua/pane/get_metadata.html` |

**Wrong org (4 refs — agent error):**
| File | Line | Has | Should Be |
|------|-----:|-----|-----------|
| `config/src/config.rs` | 1630 | `github.com/phaedra/phaedra` | `github.com/PaleRoses/phaedra` |
| `config/src/config.rs` | 2134 | `github.com/phaedra/phaedra` | `github.com/PaleRoses/phaedra` |
| `config/src/config.rs` | 2135 | `github.com/phaedra/phaedra` | `github.com/PaleRoses/phaedra` |
| `config/src/config.rs` | 2136 | `github.com/phaedra/phaedra` | `github.com/PaleRoses/phaedra` |

**Intentionally preserved (41 refs):**
- `config/src/lua.rs` (34) — Lua module named `wezterm` (breaking API change deferred)
- `config/src/scheme_data.rs` (6) — External theme repo URLs
- `phaedra-escape-parser/src/csi.rs` (1) — Protocol attribution

### Decision Pending: Lua Module Name

`local wezterm = require 'wezterm'` appears in every Lua config file in the WezTerm ecosystem. Renaming to `local phaedra = require 'phaedra'` breaks all existing configs. Options:

1. **Keep `wezterm`** — backward compatible, confusing branding
2. **Rename to `phaedra`** — clean break, forces config migration
3. **Support both** — `require 'phaedra'` returns the module, `require 'wezterm'` is an alias with deprecation warning

---

## IX. Crate Dependency Map

### Core (required for a terminal to exist)

```
phaedra-gui ─────────► window ──────► os/macos
    │                     │
    ├──► phaedra-font ────┤
    │        │            │
    ├──► mux ─────────────┤
    │    │                │
    │    ├──► term ───────┤
    │    │    │           │
    │    │    ├──► phaedra-escape-parser
    │    │    │           │
    │    │    ├──► phaedra-surface
    │    │    │    │
    │    │    │    ├──► phaedra-cell
    │    │    │    │
    │    │    │    └──► color-types
    │    │    │
    │    │    └──► vtparse
    │    │
    │    └──► config ─────► phaedra-dynamic
    │
    └──► termwiz
```

### Optional (can be extracted or removed)

```
phaedra-ssh (5,926 lines) ──► could become plugin
phaedra-mux-server (2,303 lines) ──► separate binary, optional
lua-api-crates (4,345 lines) ──► 16 micro-crates, consolidation candidate
```

### Utility (shared infrastructure)

```
rangeset, bintree, lfucache, filedescriptor, promise,
procinfo, base91, ratelim, frecency, tabout, umask,
phaedra-input-types, phaedra-blob-leases, env-bootstrap
```

---

*This document is a living artifact. Completed work should be collapsed to single-line summaries. Irrelevant sections should be deleted. The document reflects current state and remaining work, not historical record.*
