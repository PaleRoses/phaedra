use crate::bell::{AudibleBell, BellConfig, EasingFunction, VisualBell};
use crate::cache_config::CacheConfig;
use crate::color::{ColorSchemeFile, Palette, TabBarStyle};
use crate::color_config::ColorConfig;
use crate::cursor::CursorConfig;
use crate::domain_config::DomainConfig;
use crate::font::StyleRule;
use crate::font_config::FontConfig;
use crate::gpu_config::GpuConfig;
use crate::key_input_config::KeyInputConfig;
use crate::keyassignment::{KeyAssignment, KeyTable, KeyTableEntry, KeyTables, MouseEventTrigger};
use crate::launch_config::LaunchConfig;
use crate::lua::make_lua_context;
use crate::mux_config::MuxConfig;
use crate::mouse_config::MouseConfig;
use crate::runtime_config::RuntimeConfig;
use crate::scroll::ScrollConfig;
use crate::ssh::SshDomain;
use crate::tab_bar::TabBarConfig;
use crate::terminal_feature_config::TerminalFeatureConfig;
use crate::text_config::TextConfig;
use crate::units::Dimension;
use crate::update_check::UpdateConfig;
use crate::window_config::WindowConfig;
use crate::{
    default_config_with_overrides_applied, LoadedConfig, MouseEventTriggerMods, CONFIG_DIRS,
    CONFIG_FILE_OVERRIDE, CONFIG_OVERRIDES, CONFIG_SKIP, HOME_DIR,
};
use anyhow::Context;
use luahelper::impl_lua_conversion_dynamic;
use mlua::FromLua;
use portable_pty::CommandBuilder;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use termwiz::surface::CursorShape;
use phaedra_config_derive::ConfigMeta;
use phaedra_dynamic::{FromDynamic, ToDynamic};
use phaedra_input_types::Modifiers;
use phaedra_term::TerminalSize;

#[derive(Debug, Clone, FromDynamic, ToDynamic, ConfigMeta)]
pub struct Config {
    #[dynamic(default)]
    pub font_config: FontConfig,

    #[dynamic(default)]
    pub text: TextConfig,

    #[dynamic(default)]
    pub cursor: CursorConfig,

    #[dynamic(default)]
    pub window_config: WindowConfig,

    #[dynamic(default)]
    pub gpu: GpuConfig,

    #[dynamic(default)]
    pub color_config: ColorConfig,

    #[dynamic(default)]
    pub tab_bar: TabBarConfig,

    #[dynamic(default)]
    pub scroll: ScrollConfig,

    #[dynamic(default)]
    pub launch: LaunchConfig,

    #[dynamic(default)]
    pub terminal_features: TerminalFeatureConfig,

    #[dynamic(default)]
    pub domain: DomainConfig,

    #[dynamic(default)]
    pub mux: MuxConfig,

    #[dynamic(default)]
    pub key_input: KeyInputConfig,

    #[dynamic(default)]
    pub mouse: MouseConfig,

    #[dynamic(default)]
    pub runtime: RuntimeConfig,

    #[dynamic(default)]
    pub update_check: UpdateConfig,

    #[dynamic(default)]
    pub cache: CacheConfig,

    #[dynamic(default)]
    pub bell: BellConfig,
}
impl_lua_conversion_dynamic!(Config);

impl Default for Config {
    fn default() -> Self {
        // Ask FromDynamic to provide the defaults based on the attributes
        // specified in the struct so that we don't have to repeat
        // the same thing in a different form down here
        Config::from_dynamic(
            &phaedra_dynamic::Value::Object(Default::default()),
            Default::default(),
        )
        .unwrap()
    }
}

impl Config {
    pub fn load() -> LoadedConfig {
        Self::load_with_overrides(&phaedra_dynamic::Value::default())
    }

    /// It is relatively expensive to parse all the ssh config files,
    /// so we defer producing the default list until someone explicitly
    /// asks for it
    pub fn ssh_domains(&self) -> Vec<SshDomain> {
        if let Some(domains) = &self.domain.ssh_domains {
            domains.clone()
        } else {
            SshDomain::default_domains()
        }
    }

    pub fn update_ulimit(&self) -> anyhow::Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::resource::{getrlimit, rlim_t, setrlimit, Resource};
            use std::convert::TryInto;

            let (no_file_soft, no_file_hard) = getrlimit(Resource::RLIMIT_NOFILE)?;

            let ulimit_nofile: rlim_t = self.runtime.ulimit_nofile.try_into().with_context(|| {
                format!(
                    "ulimit_nofile value {} is out of range for this system",
                    self.runtime.ulimit_nofile
                )
            })?;

            if no_file_soft < ulimit_nofile {
                setrlimit(
                    Resource::RLIMIT_NOFILE,
                    ulimit_nofile.min(no_file_hard),
                    no_file_hard,
                )
                .with_context(|| {
                    format!(
                        "raise RLIMIT_NOFILE from {no_file_soft} to ulimit_nofile {}",
                        ulimit_nofile
                    )
                })?;
            }
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            use nix::sys::resource::{getrlimit, rlim_t, setrlimit, Resource};
            use std::convert::TryInto;

            let (nproc_soft, nproc_hard) = getrlimit(Resource::RLIMIT_NPROC)?;

            let ulimit_nproc: rlim_t = self.runtime.ulimit_nproc.try_into().with_context(|| {
                format!(
                    "ulimit_nproc value {} is out of range for this system",
                    self.runtime.ulimit_nproc
                )
            })?;

            if nproc_soft < ulimit_nproc {
                setrlimit(
                    Resource::RLIMIT_NPROC,
                    ulimit_nproc.min(nproc_hard),
                    nproc_hard,
                )
                .with_context(|| {
                    format!(
                        "raise RLIMIT_NPROC from {nproc_soft} to ulimit_nproc {}",
                        ulimit_nproc
                    )
                })?;
            }
        }

        Ok(())
    }

    pub fn load_with_overrides(overrides: &phaedra_dynamic::Value) -> LoadedConfig {
        // Note that the directories crate has methods for locating project
        // specific config directories, but only returns one of them, not
        // multiple.  In addition, it spawns a lot of subprocesses,
        // so we do this bit "by-hand"

        let mut paths = vec![PathPossibility::optional(HOME_DIR.join(".phaedra.lua"))];
        for dir in CONFIG_DIRS.iter() {
            paths.push(PathPossibility::optional(dir.join("phaedra.lua")))
        }

        if cfg!(windows) {
            // On Windows, a common use case is to maintain a thumb drive
            // with a set of portable tools that don't need to be installed
            // to run on a target system.  In that scenario, the user would
            // like to run with the config from their thumbdrive because
            // either the target system won't have any config, or will have
            // the config of another user.
            // So we prioritize that here: if there is a config in the same
            // dir as the executable that will take precedence.
            if let Ok(exe_name) = std::env::current_exe() {
                if let Some(exe_dir) = exe_name.parent() {
                    paths.insert(0, PathPossibility::optional(exe_dir.join("phaedra.lua")));
                }
            }
        }
        if let Some(path) = std::env::var_os("PHAEDRA_CONFIG_FILE") {
            log::trace!("Note: PHAEDRA_CONFIG_FILE is set in the environment");
            paths.insert(0, PathPossibility::required(path.into()));
        }

        if let Some(path) = CONFIG_FILE_OVERRIDE.lock().unwrap().as_ref() {
            log::trace!("Note: config file override is set");
            paths.insert(0, PathPossibility::required(path.clone()));
        }

        for path_item in &paths {
            if CONFIG_SKIP.load(Ordering::Relaxed) {
                break;
            }

            match Self::try_load(path_item, overrides) {
                Err(err) => {
                    return LoadedConfig {
                        config: Err(err),
                        file_name: Some(path_item.path.clone()),
                        lua: None,
                        warnings: vec![],
                    }
                }
                Ok(None) => continue,
                Ok(Some(loaded)) => return loaded,
            }
        }

        // We didn't find (or were asked to skip) a phaedra.lua file, so
        // update the environment to make it simpler to understand this
        // state.
        std::env::remove_var("PHAEDRA_CONFIG_FILE");
        std::env::remove_var("PHAEDRA_CONFIG_DIR");

        match Self::try_default() {
            Err(err) => LoadedConfig {
                config: Err(err),
                file_name: None,
                lua: None,
                warnings: vec![],
            },
            Ok(cfg) => cfg,
        }
    }

    pub fn try_default() -> anyhow::Result<LoadedConfig> {
        let (config, warnings) =
            phaedra_dynamic::Error::capture_warnings(|| -> anyhow::Result<Config> {
                Ok(default_config_with_overrides_applied()?.compute_extra_defaults(None))
            });

        Ok(LoadedConfig {
            config: Ok(config?),
            file_name: None,
            lua: Some(make_lua_context(Path::new(""))?),
            warnings,
        })
    }

    fn try_load(
        path_item: &PathPossibility,
        overrides: &phaedra_dynamic::Value,
    ) -> anyhow::Result<Option<LoadedConfig>> {
        let p = path_item.path.as_path();
        log::trace!("consider config: {}", p.display());
        let mut file = match std::fs::File::open(p) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound if !path_item.is_required => return Ok(None),
                _ => anyhow::bail!("Error opening {}: {}", p.display(), err),
            },
        };

        let mut s = String::new();
        file.read_to_string(&mut s)?;
        let lua = make_lua_context(p)?;

        let (config, warnings) =
            phaedra_dynamic::Error::capture_warnings(|| -> anyhow::Result<Config> {
                let cfg: Config;

                let config: mlua::Value = smol::block_on(
                    // Skip a potential BOM that Windows software may have placed in the
                    // file. Note that we can't catch this happening for files that are
                    // imported via the lua require function.
                    lua.load(s.trim_start_matches('\u{FEFF}'))
                        .set_name(p.to_string_lossy())
                        .eval_async(),
                )?;
                let config = Config::apply_overrides_to(&lua, config)?;
                let config = Config::apply_overrides_obj_to(&lua, config, overrides)?;
                cfg = Config::from_lua(config, &lua).with_context(|| {
                    format!(
                        "Error converting lua value returned by script {} to Config struct",
                        p.display()
                    )
                })?;
                cfg.check_consistency()?;

                // Compute but discard the key bindings here so that we raise any
                // problems earlier than we use them.
                let _ = cfg.key_bindings();

                std::env::set_var("PHAEDRA_CONFIG_FILE", p);
                if let Some(dir) = p.parent() {
                    std::env::set_var("PHAEDRA_CONFIG_DIR", dir);
                }
                Ok(cfg)
            });
        let cfg = config?;

        Ok(Some(LoadedConfig {
            config: Ok(cfg.compute_extra_defaults(Some(p))),
            file_name: Some(p.to_path_buf()),
            lua: Some(lua),
            warnings,
        }))
    }

    pub(crate) fn apply_overrides_obj_to<'l>(
        lua: &'l mlua::Lua,
        mut config: mlua::Value<'l>,
        overrides: &phaedra_dynamic::Value,
    ) -> anyhow::Result<mlua::Value<'l>> {
        // config may be a table, or it may be a config builder.
        // We'll leave it up to lua to call the appropriate
        // index function as managing that from Rust is a PITA.
        let setter: mlua::Function = lua
            .load(
                r#"
                    return function(config, key, value)
                        config[key] = value;
                        return config;
                    end
                    "#,
            )
            .eval()?;

        match overrides {
            phaedra_dynamic::Value::Object(obj) => {
                for (key, value) in obj {
                    let key = luahelper::dynamic_to_lua_value(lua, key.clone())?;
                    let value = luahelper::dynamic_to_lua_value(lua, value.clone())?;
                    config = setter.call((config, key, value))?;
                }
                Ok(config)
            }
            _ => Ok(config),
        }
    }

    pub(crate) fn apply_overrides_to<'l>(
        lua: &'l mlua::Lua,
        mut config: mlua::Value<'l>,
    ) -> anyhow::Result<mlua::Value<'l>> {
        let overrides = CONFIG_OVERRIDES.lock().unwrap();
        for (key, value) in &*overrides {
            if value == "nil" {
                // Literal nil as the value is the same as not specifying the value.
                // We special case this here as we want to explicitly check for
                // the value evaluating as nil, as can happen in the case where the
                // user specifies something like: `--config term=xterm`.
                // The RHS references a global that doesn't exist and evaluates as
                // nil. We want to raise this as an error.
                continue;
            }
            let literal = value.escape_debug();
            let code = format!(
                r#"
                local phaedra = require 'phaedra';
                local value = {value};
                if value == nil then
                    error("{literal} evaluated as nil. Check for missing quotes or other syntax issues")
                end
                config.{key} = value;
                return config;
                "#,
            );
            let chunk = lua.load(&code);
            let chunk = chunk.set_name(format!("--config {}={}", key, value));
            lua.globals().set("config", config.clone())?;
            log::debug!("Apply {}={} to config", key, value);
            config = chunk.eval()?;
        }
        Ok(config)
    }

    /// Check for logical conflicts in the config
    pub fn check_consistency(&self) -> anyhow::Result<()> {
        self.check_domain_consistency()?;
        Ok(())
    }

    fn check_domain_consistency(&self) -> anyhow::Result<()> {
        let mut domains = HashMap::new();

        let mut check_domain = |name: &str, kind: &str| {
            if let Some(exists) = domains.get(name) {
                anyhow::bail!(
                    "{kind} with name \"{name}\" conflicts with \
                     another existing {exists} with the same name"
                );
            }
            domains.insert(name.to_string(), kind.to_string());
            Ok(())
        };

        for d in &self.domain.unix_domains {
            check_domain(&d.name, "unix domain")?;
        }
        if let Some(domains) = &self.domain.ssh_domains {
            for d in domains {
                check_domain(&d.name, "ssh domain")?;
            }
        }
        for d in &self.domain.exec_domains {
            check_domain(&d.name, "exec domain")?;
        }
        for d in &self.domain.tls_clients {
            check_domain(&d.name, "tls domain")?;
        }
        Ok(())
    }

    pub fn default_config() -> Self {
        Self::default().compute_extra_defaults(None)
    }

    pub fn key_bindings(&self) -> KeyTables {
        let mut tables = KeyTables::default();

        for k in &self.key_input.keys {
            let (key, mods) = k
                .key
                .key
                .resolve(self.key_input.key_map_preference)
                .normalize_shift(k.key.mods);
            tables.default.insert(
                (key, mods),
                KeyTableEntry {
                    action: k.action.clone(),
                },
            );
        }

        for (name, keys) in &self.key_input.key_tables {
            let mut table = KeyTable::default();
            for k in keys {
                let (key, mods) = k
                    .key
                    .key
                    .resolve(self.key_input.key_map_preference)
                    .normalize_shift(k.key.mods);
                table.insert(
                    (key, mods),
                    KeyTableEntry {
                        action: k.action.clone(),
                    },
                );
            }
            tables.by_name.insert(name.to_string(), table);
        }

        tables
    }

    pub fn mouse_bindings(
        &self,
    ) -> HashMap<(MouseEventTrigger, MouseEventTriggerMods), KeyAssignment> {
        let mut map = HashMap::new();

        for m in &self.mouse.mouse_bindings {
            map.insert((m.event.clone(), m.mods), m.action.clone());
        }

        map
    }

    pub fn visual_bell(&self) -> &VisualBell {
        &self.bell.visual_bell
    }

    pub fn audible_bell(&self) -> &AudibleBell {
        &self.bell.audible_bell
    }

    pub fn check_for_updates(&self) -> bool {
        self.update_check.check_for_updates
    }

    pub fn check_for_updates_interval_seconds(&self) -> u64 {
        self.update_check.check_for_updates_interval_seconds
    }

    pub fn scrollback_lines(&self) -> usize {
        self.scroll.scrollback_lines
    }

    pub fn enable_scroll_bar(&self) -> bool {
        self.scroll.enable_scroll_bar
    }

    pub fn min_scroll_bar_height(&self) -> Dimension {
        self.scroll.min_scroll_bar_height
    }

    pub fn scroll_to_bottom_on_input(&self) -> bool {
        self.scroll.scroll_to_bottom_on_input
    }

    pub fn alternate_buffer_wheel_scroll_speed(&self) -> u8 {
        self.scroll.alternate_buffer_wheel_scroll_speed
    }

    pub fn cursor_thickness(&self) -> Option<Dimension> {
        self.cursor.cursor_thickness
    }

    pub fn cursor_blink_rate(&self) -> u64 {
        self.cursor.cursor_blink_rate
    }

    pub fn cursor_blink_ease_in(&self) -> EasingFunction {
        self.cursor.cursor_blink_ease_in
    }

    pub fn cursor_blink_ease_out(&self) -> EasingFunction {
        self.cursor.cursor_blink_ease_out
    }

    pub fn default_cursor_style(&self) -> DefaultCursorStyle {
        self.cursor.default_cursor_style
    }

    pub fn force_reverse_video_cursor(&self) -> bool {
        self.cursor.force_reverse_video_cursor
    }

    pub fn reverse_video_cursor_min_contrast(&self) -> f32 {
        self.cursor.reverse_video_cursor_min_contrast
    }

    pub fn xcursor_theme(&self) -> Option<&str> {
        self.cursor.xcursor_theme.as_deref()
    }

    pub fn xcursor_size(&self) -> Option<u32> {
        self.cursor.xcursor_size
    }

    pub fn tab_bar_style(&self) -> &TabBarStyle {
        &self.tab_bar.tab_bar_style
    }

    pub fn enable_tab_bar(&self) -> bool {
        self.tab_bar.enable_tab_bar
    }

    pub fn use_fancy_tab_bar(&self) -> bool {
        self.tab_bar.use_fancy_tab_bar
    }

    pub fn tab_bar_at_bottom(&self) -> bool {
        self.tab_bar.tab_bar_at_bottom
    }

    pub fn mouse_wheel_scrolls_tabs(&self) -> bool {
        self.tab_bar.mouse_wheel_scrolls_tabs
    }

    pub fn show_tab_index_in_tab_bar(&self) -> bool {
        self.tab_bar.show_tab_index_in_tab_bar
    }

    pub fn show_tabs_in_tab_bar(&self) -> bool {
        self.tab_bar.show_tabs_in_tab_bar
    }

    pub fn show_new_tab_button_in_tab_bar(&self) -> bool {
        self.tab_bar.show_new_tab_button_in_tab_bar
    }

    pub fn show_close_tab_button_in_tabs(&self) -> bool {
        self.tab_bar.show_close_tab_button_in_tabs
    }

    pub fn tab_and_split_indices_are_zero_based(&self) -> bool {
        self.tab_bar.tab_and_split_indices_are_zero_based
    }

    pub fn tab_max_width(&self) -> usize {
        self.tab_bar.tab_max_width
    }

    pub fn hide_tab_bar_if_only_one_tab(&self) -> bool {
        self.tab_bar.hide_tab_bar_if_only_one_tab
    }

    pub fn switch_to_last_active_tab_when_closing_tab(&self) -> bool {
        self.tab_bar.switch_to_last_active_tab_when_closing_tab
    }

    pub fn disable_default_mouse_bindings(&self) -> bool {
        self.mouse.disable_default_mouse_bindings
    }

    pub fn bypass_mouse_reporting_modifiers(&self) -> Modifiers {
        self.mouse.bypass_mouse_reporting_modifiers
    }

    pub fn selection_word_boundary(&self) -> &str {
        self.mouse.selection_word_boundary.as_str()
    }

    pub fn quick_select_patterns(&self) -> &[String] {
        self.mouse.quick_select_patterns.as_slice()
    }

    pub fn quick_select_alphabet(&self) -> &str {
        self.mouse.quick_select_alphabet.as_str()
    }

    pub fn quick_select_remove_styling(&self) -> bool {
        self.mouse.quick_select_remove_styling
    }

    pub fn disable_default_quick_select_patterns(&self) -> bool {
        self.mouse.disable_default_quick_select_patterns
    }

    pub fn hide_mouse_cursor_when_typing(&self) -> bool {
        self.mouse.hide_mouse_cursor_when_typing
    }

    pub fn swallow_mouse_click_on_pane_focus(&self) -> bool {
        self.mouse.swallow_mouse_click_on_pane_focus
    }

    pub fn swallow_mouse_click_on_window_focus(&self) -> bool {
        self.mouse.swallow_mouse_click_on_window_focus
    }

    pub fn pane_focus_follows_mouse(&self) -> bool {
        self.mouse.pane_focus_follows_mouse
    }

    pub fn quote_dropped_files(&self) -> DroppedFileQuoting {
        self.mouse.quote_dropped_files
    }

    /// In some cases we need to compute expanded values based
    /// on those provided by the user.  This is where we do that.
    pub fn compute_extra_defaults(&self, config_path: Option<&Path>) -> Self {
        let mut cfg = self.clone();

        // Convert any relative font dirs to their config file relative locations
        if let Some(config_dir) = config_path.as_ref().and_then(|p| p.parent()) {
            for font_dir in &mut cfg.font_config.font_dirs {
                if !font_dir.is_absolute() {
                    let dir = config_dir.join(&font_dir);
                    *font_dir = dir;
                }
            }

            if let Some(path) = &self.gpu.webgpu_shader {
                if !path.is_absolute() {
                    cfg.gpu.webgpu_shader.replace(config_dir.join(path));
                }
            }
        }

        // Add some reasonable default font rules
        let reduced = self.font_config.font.reduce_first_font_to_family();

        let italic = reduced.make_italic();

        let bold = reduced.make_bold();
        let bold_italic = bold.make_italic();

        let half_bright = reduced.make_half_bright();
        let half_bright_italic = half_bright.make_italic();

        cfg.font_config.font_rules.push(StyleRule {
            italic: Some(true),
            intensity: Some(phaedra_term::Intensity::Half),
            font: half_bright_italic,
            ..Default::default()
        });

        cfg.font_config.font_rules.push(StyleRule {
            italic: Some(false),
            intensity: Some(phaedra_term::Intensity::Half),
            font: half_bright,
            ..Default::default()
        });

        cfg.font_config.font_rules.push(StyleRule {
            italic: Some(false),
            intensity: Some(phaedra_term::Intensity::Bold),
            font: bold,
            ..Default::default()
        });

        cfg.font_config.font_rules.push(StyleRule {
            italic: Some(true),
            intensity: Some(phaedra_term::Intensity::Bold),
            font: bold_italic,
            ..Default::default()
        });

        cfg.font_config.font_rules.push(StyleRule {
            italic: Some(true),
            intensity: Some(phaedra_term::Intensity::Normal),
            font: italic,
            ..Default::default()
        });

        // Load any additional color schemes into the color_schemes map
        cfg.load_color_schemes(&cfg.compute_color_scheme_dirs())
            .ok();

        if let Some(scheme) = cfg.color_config.color_scheme.as_ref() {
            match cfg.resolve_color_scheme() {
                None => {
                    log::error!(
                        "Your configuration specifies color_scheme=\"{}\" \
                        but that scheme was not found",
                        scheme
                    );
                }
                Some(p) => {
                    cfg.color_config.resolved_palette = p.clone();
                }
            }
        }

        if let Some(colors) = &cfg.color_config.colors {
            cfg.color_config.resolved_palette = cfg.color_config.resolved_palette.overlay_with(colors);
        }

        cfg
    }

    fn compute_color_scheme_dirs(&self) -> Vec<PathBuf> {
        let mut paths = self.color_config.color_scheme_dirs.clone();
        for dir in CONFIG_DIRS.iter() {
            paths.push(dir.join("colors"));
        }
        if cfg!(windows) {
            // See commentary re: portable tools above!
            if let Ok(exe_name) = std::env::current_exe() {
                if let Some(exe_dir) = exe_name.parent() {
                    paths.insert(0, exe_dir.join("colors"));
                }
            }
        }
        paths
    }

    fn load_color_schemes(&mut self, paths: &[PathBuf]) -> anyhow::Result<()> {
        fn extract_scheme_name(name: &str) -> Option<&str> {
            if name.ends_with(".toml") {
                let len = name.len();
                Some(&name[..len - 5])
            } else {
                None
            }
        }

        fn load_scheme(path: &Path) -> anyhow::Result<ColorSchemeFile> {
            let s = std::fs::read_to_string(path)?;
            ColorSchemeFile::from_toml_str(&s).context("parsing TOML")
        }

        for colors_dir in paths {
            if let Ok(dir) = std::fs::read_dir(colors_dir) {
                for entry in dir {
                    if let Ok(entry) = entry {
                        if let Some(name) = entry.file_name().to_str() {
                            if let Some(scheme_name) = extract_scheme_name(name) {
                                if self.color_config.color_schemes.contains_key(scheme_name) {
                                    // This scheme has already been defined
                                    continue;
                                }

                                let path = entry.path();
                                match load_scheme(&path) {
                                    Ok(scheme) => {
                                        let name = scheme
                                            .metadata
                                            .name
                                            .unwrap_or_else(|| scheme_name.to_string());
                                        log::trace!(
                                            "Loaded color scheme `{}` from {}",
                                            name,
                                            path.display()
                                        );
                                        self.color_config.color_schemes.insert(name, scheme.colors);
                                    }
                                    Err(err) => {
                                        log::error!(
                                            "Color scheme in `{}` failed to load: {:#}",
                                            path.display(),
                                            err
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn resolve_color_scheme(&self) -> Option<&Palette> {
        let scheme_name = self.color_config.color_scheme.as_ref()?;

        if let Some(palette) = self.color_config.color_schemes.get(scheme_name) {
            Some(palette)
        } else {
            crate::COLOR_SCHEMES.get(scheme_name)
        }
    }

    pub fn initial_size(&self, dpi: u32, cell_pixel_dims: Option<(usize, usize)>) -> TerminalSize {
        // If we aren't passed the actual values, guess at a plausible
        // default set of pixel dimensions.
        // This is based on "typical" 10 point font at "normal"
        // pixel density.
        // This will get filled in by the gui layer, but there is
        // an edge case where we emit an iTerm image escape in
        // the software update banner through the mux layer before
        // the GUI has had a chance to update the pixel dimensions
        // when running under X11.
        // This is a bit gross.
        let (cell_pixel_width, cell_pixel_height) = cell_pixel_dims.unwrap_or((8, 16));

        TerminalSize {
            rows: self.window_config.initial_rows as usize,
            cols: self.window_config.initial_cols as usize,
            pixel_width: cell_pixel_width * self.window_config.initial_cols as usize,
            pixel_height: cell_pixel_height * self.window_config.initial_rows as usize,
            dpi,
        }
    }

    pub fn build_prog(
        &self,
        prog: Option<Vec<&OsStr>>,
        default_prog: Option<&Vec<String>>,
        default_cwd: Option<&PathBuf>,
    ) -> anyhow::Result<CommandBuilder> {
        let mut cmd = match prog {
            Some(args) => {
                let mut args = args.iter();
                let mut cmd = CommandBuilder::new(args.next().expect("executable name"));
                cmd.args(args);
                cmd
            }
            None => {
                if let Some(prog) = default_prog {
                    let mut args = prog.iter();
                    let mut cmd = CommandBuilder::new(args.next().expect("executable name"));
                    cmd.args(args);
                    cmd
                } else {
                    CommandBuilder::new_default_prog()
                }
            }
        };

        self.apply_cmd_defaults(&mut cmd, None, default_cwd);

        Ok(cmd)
    }

    pub fn apply_cmd_defaults(
        &self,
        cmd: &mut CommandBuilder,
        default_prog: Option<&Vec<String>>,
        default_cwd: Option<&PathBuf>,
    ) {
        // Apply `default_cwd` only if `cwd` is not already set, allows `--cwd`
        // option to take precedence
        if let (None, Some(cwd)) = (cmd.get_cwd(), default_cwd) {
            cmd.cwd(cwd);
        }

        if let Some(default_prog) = default_prog {
            if cmd.is_default_prog() {
                cmd.replace_default_prog(default_prog);
            }
        }

        // Augment WSLENV so that TERM related environment propagates
        // across the win32/wsl boundary
        let mut wsl_env = std::env::var("WSLENV").ok();

        // If we are running as an appimage, we will have "$APPIMAGE"
        // and "$APPDIR" set in the phaedra process. These will be
        // propagated to the child processes. Since some apps (including
        // phaedra) use these variables to detect if they are running in
        // an appimage, those child processes will be misconfigured.
        // Ensure that they are unset.
        // https://docs.appimage.org/packaging-guide/environment-variables.html#id2
        cmd.env_remove("APPIMAGE");
        cmd.env_remove("APPDIR");
        cmd.env_remove("OWD");

        for (k, v) in &self.launch.set_environment_variables {
            if k == "WSLENV" {
                wsl_env.replace(v.clone());
            } else {
                cmd.env(k, v);
            }
        }

        if wsl_env.is_some() || cfg!(windows) {
            let mut wsl_env = wsl_env.unwrap_or_default();
            if !wsl_env.is_empty() {
                wsl_env.push(':');
            }
            wsl_env.push_str("TERM:COLORTERM:TERM_PROGRAM:TERM_PROGRAM_VERSION");
            cmd.env("WSLENV", wsl_env);
        }

        #[cfg(unix)]
        cmd.umask(umask::UmaskSaver::saved_umask());
        cmd.env("TERM", &self.launch.term);
        cmd.env("COLORTERM", "truecolor");
        // TERM_PROGRAM and TERM_PROGRAM_VERSION are an emerging
        // de-facto standard for identifying the terminal.
        cmd.env("TERM_PROGRAM", "Phaedra");
        cmd.env("TERM_PROGRAM_VERSION", crate::phaedra_version());
    }
}

pub fn running_under_wsl() -> bool {
    false
}

pub(crate) fn compute_cache_dir() -> anyhow::Result<PathBuf> {
    if let Some(runtime) = dirs_next::cache_dir() {
        return Ok(runtime.join("phaedra"));
    }

    Ok(crate::HOME_DIR.join(".local/share/phaedra"))
}

pub(crate) fn compute_data_dir() -> anyhow::Result<PathBuf> {
    if let Some(runtime) = dirs_next::data_dir() {
        return Ok(runtime.join("phaedra"));
    }

    Ok(crate::HOME_DIR.join(".local/share/phaedra"))
}

pub(crate) fn compute_runtime_dir() -> anyhow::Result<PathBuf> {
    if let Some(runtime) = dirs_next::runtime_dir() {
        return Ok(runtime.join("phaedra"));
    }

    Ok(crate::HOME_DIR.join(".local/share/phaedra"))
}

pub fn pki_dir() -> anyhow::Result<PathBuf> {
    compute_runtime_dir().map(|d| d.join("pki"))
}

pub fn default_read_timeout() -> Duration {
    Duration::from_secs(60)
}

pub fn default_write_timeout() -> Duration {
    Duration::from_secs(60)
}

pub fn default_local_echo_threshold_ms() -> Option<u64> {
    Some(100)
}

#[derive(FromDynamic, ToDynamic, Clone, Copy, Debug, Default)]
pub enum DefaultCursorStyle {
    BlinkingBlock,
    #[default]
    SteadyBlock,
    BlinkingUnderline,
    SteadyUnderline,
    BlinkingBar,
    SteadyBar,
}

impl DefaultCursorStyle {
    pub fn effective_shape(self, shape: CursorShape) -> CursorShape {
        match shape {
            CursorShape::Default => match self {
                Self::BlinkingBlock => CursorShape::BlinkingBlock,
                Self::SteadyBlock => CursorShape::SteadyBlock,
                Self::BlinkingUnderline => CursorShape::BlinkingUnderline,
                Self::SteadyUnderline => CursorShape::SteadyUnderline,
                Self::BlinkingBar => CursorShape::BlinkingBar,
                Self::SteadyBar => CursorShape::SteadyBar,
            },
            _ => shape,
        }
    }
}

const fn default_one_cell() -> Dimension {
    Dimension::Cells(1.)
}

const fn default_half_cell() -> Dimension {
    Dimension::Cells(0.5)
}

#[derive(FromDynamic, ToDynamic, Clone, Copy, Debug)]
pub struct WindowPadding {
    #[dynamic(try_from = "crate::units::PixelUnit", default = "default_one_cell")]
    pub left: Dimension,
    #[dynamic(try_from = "crate::units::PixelUnit", default = "default_half_cell")]
    pub top: Dimension,
    #[dynamic(try_from = "crate::units::PixelUnit", default = "default_one_cell")]
    pub right: Dimension,
    #[dynamic(try_from = "crate::units::PixelUnit", default = "default_half_cell")]
    pub bottom: Dimension,
}

impl Default for WindowPadding {
    fn default() -> Self {
        Self {
            left: default_one_cell(),
            right: default_one_cell(),
            top: default_half_cell(),
            bottom: default_half_cell(),
        }
    }
}

#[derive(FromDynamic, ToDynamic, Clone, Copy, Debug, Default)]
pub struct WindowContentAlignment {
    pub horizontal: HorizontalWindowContentAlignment,
    pub vertical: VerticalWindowContentAlignment,
}

#[derive(Debug, FromDynamic, ToDynamic, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalWindowContentAlignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, FromDynamic, ToDynamic, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalWindowContentAlignment {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(FromDynamic, ToDynamic, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewlineCanon {
    // FIXME: also allow deserialziing from bool
    None,
    LineFeed,
    CarriageReturn,
    CarriageReturnAndLineFeed,
}

#[derive(FromDynamic, ToDynamic, Clone, Copy, Debug, Default)]
pub enum WindowCloseConfirmation {
    #[default]
    AlwaysPrompt,
    NeverPrompt,
    // TODO: something smart where we see whether the
    // running programs are stateful
}

struct PathPossibility {
    path: PathBuf,
    is_required: bool,
}
impl PathPossibility {
    pub fn required(path: PathBuf) -> PathPossibility {
        PathPossibility {
            path,
            is_required: true,
        }
    }
    pub fn optional(path: PathBuf) -> PathPossibility {
        PathPossibility {
            path,
            is_required: false,
        }
    }
}

/// Behavior when the program spawned by phaedra terminates
#[derive(Debug, FromDynamic, ToDynamic, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitBehavior {
    /// Close the associated pane
    #[default]
    Close,
    /// Close the associated pane if the process was successful
    CloseOnCleanExit,
    /// Hold the pane until it is explicitly closed
    Hold,
}

#[derive(Debug, FromDynamic, ToDynamic, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitBehaviorMessaging {
    #[default]
    Verbose,
    Brief,
    Terse,
    None,
}

#[derive(Debug, FromDynamic, ToDynamic, Clone, Copy, PartialEq, Eq)]
pub enum DroppedFileQuoting {
    /// No quoting is performed, the file name is passed through as-is
    None,
    /// Backslash escape only spaces, leaving all other characters as-is
    SpacesOnly,
    /// Use POSIX style shell word escaping
    Posix,
    /// Use Windows style shell word escaping
    Windows,
    /// Always double quote the file name
    WindowsAlwaysQuoted,
}

impl Default for DroppedFileQuoting {
    fn default() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::SpacesOnly
        }
    }
}

impl DroppedFileQuoting {
    pub fn escape(self, s: &str) -> String {
        match self {
            Self::None => s.to_string(),
            Self::SpacesOnly => s.replace(" ", "\\ "),
            // https://docs.rs/shlex/latest/shlex/fn.quote.html
            Self::Posix => shlex::try_quote(s)
                .unwrap_or_else(|_| "".into())
                .into_owned(),
            Self::Windows => {
                let chars_need_quoting = [' ', '\t', '\n', '\x0b', '\"'];
                if s.chars().any(|c| chars_need_quoting.contains(&c)) {
                    format!("\"{}\"", s)
                } else {
                    s.to_string()
                }
            }
            Self::WindowsAlwaysQuoted => format!("\"{}\"", s),
        }
    }
}

#[derive(Debug, ToDynamic, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoldBrightening {
    /// Bold doesn't influence palette selection
    No,
    /// Bold Shifts palette from 0-7 to 8-15 and preserves bold font
    #[default]
    BrightAndBold,
    /// Bold Shifts palette from 0-7 to 8-15 and removes bold intensity
    BrightOnly,
}

impl FromDynamic for BoldBrightening {
    fn from_dynamic(
        value: &phaedra_dynamic::Value,
        options: phaedra_dynamic::FromDynamicOptions,
    ) -> Result<Self, phaedra_dynamic::Error> {
        match String::from_dynamic(value, options) {
            Ok(s) => match s.as_str() {
                "No" => Ok(Self::No),
                "BrightAndBold" => Ok(Self::BrightAndBold),
                "BrightOnly" => Ok(Self::BrightOnly),
                s => Err(phaedra_dynamic::Error::Message(format!(
                    "`{s}` is not valid, use one of `No`, `BrightAndBold` or `BrightOnly`"
                ))),
            },
            Err(err) => match bool::from_dynamic(value, options) {
                Ok(true) => Ok(Self::BrightAndBold),
                Ok(false) => Ok(Self::No),
                Err(_) => Err(err),
            },
        }
    }
}

#[derive(Debug, FromDynamic, ToDynamic, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImePreeditRendering {
    /// IME preedit is rendered by Phaedra itself
    #[default]
    Builtin,
    /// IME preedit is rendered by system
    System,
}

#[derive(Debug, FromDynamic, ToDynamic, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationHandling {
    #[default]
    AlwaysShow,
    NeverShow,
    SuppressFromFocusedPane,
    SuppressFromFocusedTab,
    SuppressFromFocusedWindow,
}

pub(crate) fn validate_domain_name(name: &str) -> Result<(), String> {
    if name == "local" {
        Err(format!(
            "\"{name}\" is a built-in domain and cannot be redefined"
        ))
    } else if name == "" {
        Err("the empty string is an invalid domain name".to_string())
    } else {
        Ok(())
    }
}
