# Phaedra — Spring 2026 Roadmap

> *Φαίδρα — The bright one rises from WezTerm's foundations.*

**Repo:** `github.com/PaleRoses/phaedra`
**Bundle ID:** `dev.phaedra.terminal`
**Platform:** macOS (exclusive)
**Renderer:** wgpu (WebGPU, single backend)

---

## Completed Work

| Phase | Description | Status | Lines Changed |
|-------|-------------|--------|--------------|
| 1.0 | Fork setup, rename crates, rebrand binaries | Done | ~2,000 |
| 2.0 | Kill Glium — collapse RenderContext enum to struct, delete OpenGL backend | Done | -2,300 |
| — | Tier 0 cleanup — bitflags v1→v2, syn v1→v2, macOS framework unpinning | Done | ~500 |
| — | Platform purge — delete ALL Windows support (code, assets, CI, deps) | Done | -136K code, -44.4MB assets |
| — | Platform purge — delete ALL Linux support (X11, Wayland, CI, packaging) | Done | -400K code, -30 CI workflows |
| — | Platform purge — delete phaedra-uds crate, replace with std Unix sockets | Done | -1 crate |
| — | Config cleanup — remove show_update_window, legacy background compat shim | Done | ~200 |
| — | Config cleanup — remove enable_wayland, prefer_egl, kde_window_background_blur | Done | ~60 |
| — | Config cleanup — remove running_under_wsl() + Lua binding | Done | ~40 |
| — | Branding — repoint update checker to PaleRoses/phaedra | Done | ~10 |
| — | Branding — update all Cargo.toml authors, bundle ID, README, PRIVACY | Done | ~80 |
| — | Branding — rename wezterm→phaedra across .rs source files | Done | ~200 |
| — | Branding — delete WezTerm changelog, dead docs, dead CI, dead assets | Done | -2,500+ |
| — | Config paths — .wezterm.lua → .phaedra.lua, WEZTERM_CONFIG_FILE → PHAEDRA_CONFIG_FILE | Done | ~20 |
| — | Serial port excision — delete pty/src/serial.rs, config/src/serial.rs, all wiring | Done | -314 |

**Current state:** 393K lines total, ~158K logic, macOS-only, wgpu-only, `cargo check` clean.

---

## Spring 2026 Tracks

Five parallel tracks. Non-overlapping crate boundaries. All unblocked.

```
Track A ──── Config Decomposition ──────────────────────────────►
Track B ──── VT Core Extraction ────────────────────────────────►
Track C ──── GUI Decomposition ─────────────────────────────────►
Track D ──── Custom Shader Support ──────────► (shorter)
Track E ──── macOS Native UI ───────────────────────────────────►
```

---

### Track A: Config Decomposition

**Goal:** Transform the 253-field Config god object into a composed, coalgebraic system with typed observation traits and optic-based introspection.

**Reference:** `docs/architecture/functional.md` — Sections I-VI

| Step | Description | Crates Touched | Est. Effort | Depends On |
|------|-------------|----------------|-------------|------------|
| A.1 | Factor Bell sub-record (2 fields) | config | 1 day | — |
| A.2 | Factor Update sub-record (2 fields) | config, phaedra-gui | 1 day | — |
| A.3 | Factor Scroll sub-record (4 fields) | config, term, phaedra-gui | 1 day | — |
| A.4 | Factor Cursor sub-record (10 fields) | config, phaedra-gui | 2 days | — |
| A.5 | Factor TabBar sub-record (11 fields) | config, phaedra-gui | 2 days | — |
| A.6 | Factor Mouse sub-record (11 fields) | config, phaedra-gui | 2 days | — |
| A.7 | Factor Launch sub-record (10 fields) | config, mux, phaedra-gui | 2 days | — |
| A.8 | Factor Domain sub-record (9 fields) | config, mux, phaedra-gui, phaedra-ssh | 3 days | — |
| A.9 | Factor Keys sub-record (17 fields) | config, phaedra-gui, window | 3 days | — |
| A.10 | Factor Font sub-record (21 fields) | config, phaedra-font, phaedra-gui | 3 days | — |
| A.11 | Factor Color sub-record (16 fields) | config, phaedra-gui, term | 3 days | — |
| A.12 | Factor Window sub-record (18 fields) | config, window, phaedra-gui | 3 days | — |
| A.13 | Factor Text sub-record (18 fields) | config, term, phaedra-gui, phaedra-font | 3 days | — |
| A.14 | Sub-cluster Performance/Runtime (28 fields) into GPU, Cache, Protocol, Debug, Runtime | config | 2 days | A.1-A.13 |
| A.15 | Define observation traits per domain | config, all consumers | 5 days | A.1-A.14 |
| A.16 | Add per-domain generation counters | config | 2 days | A.15 |
| A.17 | Wire Lua metatables for flat namespace compat | config | 3 days | A.15 |
| A.18 | Implement PartialConfig + hylomorphic loading | config | 5 days | A.15 |
| A.19 | Extend ConfigMeta with domain tags + lens paths | config/derive | 3 days | A.15 |

**Milestones:**
| Date | Milestone |
|------|-----------|
| March 7 | A.1-A.6 complete — trivial sub-records extracted |
| March 21 | A.7-A.13 complete — all domains factored |
| April 4 | A.14-A.16 complete — observation traits + generation tracking |
| April 18 | A.17-A.19 complete — Lua compat, hylomorphic loading, extended ConfigMeta |

---

### Track B: VT Core Extraction

**Goal:** Extract a pure, `no_std`-compatible VT terminal core that returns algebraic effects instead of performing I/O. Compiles to WASM.

**Reference:** Plan Phase 2A

| Step | Description | Crates Touched | Est. Effort | Depends On |
|------|-------------|----------------|-------------|------------|
| B.1 | Feature-gate FromDynamic/ToDynamic derives | term, phaedra-input-types, color-types | 3 days | — |
| B.2 | Define TerminalEffect enum (~25 variants) | term | 3 days | — |
| B.3 | Convert Performer to return effects | term | 5 days | B.2 |
| B.4 | Extract termwiz shim types | term | 1 day | B.3 |
| B.5 | Define TerminalOutput trait | term | 1 day | B.3 |
| B.6 | Feature-gate ThreadedWriter behind std | term | 1 day | B.5 |
| B.7 | Add no_std + alloc, swap HashMap → hashbrown | term | 2 days | B.1, B.4-B.6 |

**Verification gates:**
| Gate | Command |
|------|---------|
| Compiles without Dynamic | `cargo check -p phaedra-term --no-default-features` |
| WASM target | `cargo check -p phaedra-term --target wasm32-unknown-unknown --no-default-features --features alloc` |
| Existing tests pass | `cargo test -p phaedra-term` |

**Milestones:**
| Date | Milestone |
|------|-----------|
| March 14 | B.1-B.2 complete — feature gates + effect enum |
| March 28 | B.3 complete — Performer returns effects |
| April 11 | B.4-B.7 complete — no_std + WASM verified |

---

### Track C: GUI Decomposition

**Goal:** Break the phaedra-gui monolith into composable crates. Extract rendering, unicode data, and introduce command pattern.

**Reference:** Plan Phase 2B

| Step | Description | Crates Touched | Est. Effort | Depends On |
|------|-------------|----------------|-------------|------------|
| C.1 | Extract unicode_names.rs to phaedra-unicode-data crate | phaedra-gui | 1 day | — |
| C.2 | Extract phaedra-render-wgpu crate | phaedra-gui | 5 days | Phase 2.0 (done) |
| C.3 | Introduce RenderCommand enum | phaedra-render-wgpu, phaedra-gui | 3 days | C.2 |
| C.4 | Define TermWindowAccessor trait, slim TermWindow | phaedra-gui | 5 days | C.2 |
| C.5 | Extract input handler with command pattern | phaedra-gui | 3 days | C.4 |

**Milestones:**
| Date | Milestone |
|------|-----------|
| March 7 | C.1 complete — build times measurably faster |
| March 21 | C.2 complete — rendering in dedicated crate |
| April 4 | C.3-C.4 complete — RenderCommand enum + TermWindowAccessor trait |
| April 18 | C.5 complete — input handler extracted |

---

### Track D: Custom Shader Support

**Goal:** Enable Ghostty-compatible GLSL custom shaders with hot-reload. Port 10+ community shaders that work unmodified.

**Reference:** Plan Phase 3

| Step | Description | Crates Touched | Est. Effort | Depends On |
|------|-------------|----------------|-------------|------------|
| D.1 | Enable GLSL in wgpu via naga | phaedra-gui | 1 day | — |
| D.2 | Build Shadertoy compatibility layer (iResolution, iTime, iChannel0, mainImage) | phaedra-gui | 3 days | D.1 |
| D.3 | Extend PostProcessUniform with cursor pos, time delta, mouse state | phaedra-gui | 2 days | D.2 |
| D.4 | Implement shader chaining via ping-pong texture buffers | phaedra-gui | 3 days | D.3 |
| D.5 | Hot-reload via notify::Watcher + compatibility testing with 10+ Ghostty shaders | phaedra-gui | 3 days | D.4 |

**Verification gate:** Load a Ghostty community `.glsl` shader — renders correctly unmodified. Hot-reload edit — <500ms.

**Milestones:**
| Date | Milestone |
|------|-----------|
| March 14 | D.1-D.2 complete — GLSL shaders load and render |
| March 28 | D.3-D.5 complete — full shader pipeline with hot-reload |

---

### Track E: macOS Native UI

**Goal:** Native macOS feel — transparent titlebar, context menus, quick terminal dropdown.

**Reference:** Plan Phase 4

| Step | Description | Crates Touched | Est. Effort | Depends On |
|------|-------------|----------------|-------------|------------|
| E.1 | Transparent titlebar (setTitlebarAppearsTransparent, config key) | window, config | 2 days | — |
| E.2 | Native context menus (rightMouseDown → NSMenu → KeyAssignment) | window, phaedra-gui | 3 days | E.1 |
| E.3 | Quick terminal — NSPanel + global hotkey via CGEvent tap | window, phaedra-gui | 5 days | E.1 |
| E.4 | Slide animation for quick terminal show/hide | window | 2 days | E.3 |
| E.5 | Native tab bar integration (NSTabGroup or custom) | window, phaedra-gui | 5 days | E.1 |

**Milestones:**
| Date | Milestone |
|------|-----------|
| March 7 | E.1 complete — transparent titlebar |
| March 21 | E.2 complete — right-click context menus |
| April 11 | E.3-E.4 complete — quick terminal with animation |
| April 25 | E.5 complete — native tab bar |

---

## Cross-Track Dependencies

```
                    ┌─────────────────────────────────────────────┐
                    │           Phase 2.0 (Kill Glium)            │
                    │                  COMPLETE                    │
                    └──────┬──────────┬──────────┬───────────┬────┘
                           │          │          │           │
                    ┌──────▼──┐ ┌─────▼────┐ ┌──▼─────┐ ┌──▼─────┐
                    │ Track A │ │ Track B  │ │Track C │ │Track D │
                    │ Config  │ │ VT Core  │ │  GUI   │ │Shaders │
                    │ Decomp  │ │ Extract  │ │ Decomp │ │        │
                    └──────┬──┘ └─────┬────┘ └──┬─────┘ └──┬─────┘
                           │          │          │           │
                    ┌──────▼──────────▼──────────▼───────────▼────┐
                    │          Phase 5: Port Traits               │
                    │     (after A.15 + B.3 + C.4 stabilize)     │
                    └──────────────────┬──────────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────────┐
                    │          Phase 6: Config + Release          │
                    │     Lua config keys, shader gallery,        │
                    │     Homebrew tap, signed .app bundle         │
                    └─────────────────────────────────────────────┘

Track E (macOS Native UI) runs independently — no cross-track deps.
```

---

## Timeline Overview

```
         MARCH 2026                    APRIL 2026                    MAY 2026
   W1    W2    W3    W4         W1    W2    W3    W4         W1    W2    W3    W4
┌─────┬─────┬─────┬─────┐ ┌─────┬─────┬─────┬─────┐ ┌─────┬─────┬─────┬─────┐
│     │     │     │     │ │     │     │     │     │ │     │     │     │     │
│ A.1 │ A.7 │ A.10│ A.14│ │ A.15│A.15 │ A.17│ A.19│ │  Port Traits (Phase 5) │
│-A.6 │-A.9 │-A.13│     │ │     │     │-A.18│     │ │                       │
│     │     │     │     │ │     │     │     │     │ │                       │
├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤
│ B.1 │ B.2 │ B.3 │ B.3 │ │ B.4 │ B.7 │     │     │ │                       │
│     │     │     │     │ │-B.6 │     │     │     │ │                       │
│     │     │     │     │ │     │     │     │     │ │                       │
├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤
│ C.1 │ C.2 │ C.2 │ C.3 │ │ C.4 │ C.4 │ C.5 │     │ │                       │
│     │     │     │     │ │     │     │     │     │ │                       │
│     │     │     │     │ │     │     │     │     │ │                       │
├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤
│ D.1 │ D.2 │ D.3 │ D.4 │ │ D.5 │     │     │     │ │                       │
│     │     │     │-D.5 │ │     │     │     │     │ │                       │
│     │     │     │     │ │     │     │     │     │ │                       │
├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤ ├─────┼─────┼─────┼─────┤
│ E.1 │ E.2 │ E.2 │ E.3 │ │ E.3 │ E.4 │ E.5 │ E.5 │ │                       │
│     │     │     │     │ │     │     │     │     │ │                       │
│     │     │     │     │ │     │     │     │     │ │                       │
└─────┴─────┴─────┴─────┘ └─────┴─────┴─────┴─────┘ └─────┴─────┴─────┴─────┘
```

---

## Verification Protocol

Each track has verification gates that must pass before the next step begins.

### Track A Gates
| Gate | Command | Passes After |
|------|---------|-------------|
| Sub-record compiles | `cargo check -p config` | Each A.x step |
| Consumers updated | `cargo check` (full workspace) | Each A.x step |
| Lua compat preserved | Manual: load existing .phaedra.lua, verify no breakage | A.17 |
| Hot-reload surgical | Manual: change font_size, verify only font reload triggers | A.16 |

### Track B Gates
| Gate | Command | Passes After |
|------|---------|-------------|
| No Dynamic dep | `cargo check -p phaedra-term --no-default-features` | B.1 |
| Effects returned | `cargo test -p phaedra-term` | B.3 |
| WASM target | `cargo check -p phaedra-term --target wasm32-unknown-unknown --no-default-features --features alloc` | B.7 |

### Track C Gates
| Gate | Command | Passes After |
|------|---------|-------------|
| Build time improved | `time cargo check` before/after C.1 | C.1 |
| Render crate isolated | `cargo check -p phaedra-render-wgpu` | C.2 |
| Full workspace | `cargo build --release` | Each C.x step |

### Track D Gates
| Gate | Command | Passes After |
|------|---------|-------------|
| GLSL loads | Manual: load a .glsl shader file | D.2 |
| Ghostty compat | Manual: load 10+ Ghostty community shaders unmodified | D.5 |
| Hot-reload latency | Manual: edit shader, verify <500ms reload | D.5 |

### Track E Gates
| Gate | Command | Passes After |
|------|---------|-------------|
| Titlebar transparent | Manual: visual verification on macOS | E.1 |
| Context menu | Manual: right-click in terminal area | E.2 |
| Quick terminal | Manual: global hotkey summons/dismisses panel | E.3 |

---

## Cleanup Backlog

Items that can be addressed opportunistically, not blocking any track.

| Item | Priority | Effort | Notes |
|------|----------|--------|-------|
| Fix 10 stray `wezterm.org` URLs | High | 30 min | Agent dispatch, single file scope |
| Fix 4 wrong-org issue links | High | 10 min | `github.com/phaedra/phaedra` → `github.com/PaleRoses/phaedra` |
| Decide Lua module name (`wezterm` → `phaedra`) | Medium | — | Blocks on user decision; 34 refs in config/src/lua.rs |
| Strip ~2,000 noise comments | Low | 1 hour | Agent dispatch, mechanical |
| Remove 4 dead Windows TODOs in termwiz | Low | 10 min | `termwiz/src/terminal/windows.rs` — dead code on macOS |
| Kill `running_under_wsl()` compat shim | Low | 10 min | Currently `fn running_under_wsl() -> bool { false }` in config.rs |
| Remove flatpak/AppImage references in comments | Low | 30 min | ~5 comments reference flatpak constraints irrelevant to macOS |
| Consolidate 16 lua-api micro-crates | Medium | 2 days | Several under 100 lines; could merge into 3-4 crates |
| Evaluate SSH client as plugin extraction | Low | — | 5,926 lines, tightly coupled; future consideration |

---

## Quality Metrics

### Current Baselines (February 2026)

| Metric | Value |
|--------|-------|
| Total Rust lines | 393K |
| Logic lines (excl. data/FFI) | 158K |
| Executable statements | ~35K |
| Test functions | 350 across 61 files |
| `.unwrap()` calls | 42+ in core code |
| God objects | 3 (Config, Tab, Mux) |
| Global mutable singletons | 1 (Mux) + 12 lazy_statics (Config) |
| TODO/FIXME markers | 107 |
| `cargo check` time | ~3 seconds (incremental) |
| Platform targets | 1 (macOS) |
| Render backends | 1 (wgpu) |

### Spring 2026 Targets

| Metric | Target |
|--------|--------|
| Config fields on root struct | <20 (sub-records hold the rest) |
| God objects | 1 remaining (Tab — deferred to summer) |
| `.unwrap()` calls in core | <10 |
| VT core compiles to WASM | Yes |
| Custom GLSL shaders | 10+ Ghostty-compatible |
| Incremental build time | <2 seconds (after C.1 unicode extraction) |
| macOS native features | Transparent titlebar, context menus, quick terminal |

---

*This roadmap is a living document. Completed milestones should be collapsed to single lines. Dates are targets, not commitments. The cathedral is built one stone at a time, but every stone is placed with intention.*
