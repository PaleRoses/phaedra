use crate::*;

pub trait BellObserver {
    fn bell(&self) -> &BellConfig;
}

pub trait UpdateCheckObserver {
    fn update_check(&self) -> &UpdateConfig;
}

pub trait ScrollObserver {
    fn scroll(&self) -> &ScrollConfig;
}

pub trait CursorObserver {
    fn cursor(&self) -> &CursorConfig;
}

pub trait TabBarObserver {
    fn tab_bar(&self) -> &TabBarConfig;
}

pub trait MouseObserver {
    fn mouse(&self) -> &MouseConfig;
}

pub trait LaunchObserver {
    fn launch(&self) -> &LaunchConfig;
}

pub trait DomainObserver {
    fn domain(&self) -> &DomainConfig;
}

pub trait KeyInputObserver {
    fn key_input(&self) -> &KeyInputConfig;
}

pub trait FontConfigObserver {
    fn font_config(&self) -> &FontConfig;
}

pub trait ColorConfigObserver {
    fn color_config(&self) -> &ColorConfig;
}

pub trait WindowConfigObserver {
    fn window_config(&self) -> &WindowConfig;
}

pub trait TextObserver {
    fn text(&self) -> &TextConfig;
}

pub trait GpuObserver {
    fn gpu(&self) -> &GpuConfig;
}

pub trait CacheObserver {
    fn cache(&self) -> &CacheConfig;
}

pub trait TerminalFeaturesObserver {
    fn terminal_features(&self) -> &TerminalFeatureConfig;
}

pub trait MuxObserver {
    fn mux_config(&self) -> &MuxConfig;
}

pub trait RuntimeObserver {
    fn runtime(&self) -> &RuntimeConfig;
}

pub trait FullConfigObserver:
    BellObserver
    + UpdateCheckObserver
    + ScrollObserver
    + CursorObserver
    + TabBarObserver
    + MouseObserver
    + LaunchObserver
    + DomainObserver
    + KeyInputObserver
    + FontConfigObserver
    + ColorConfigObserver
    + WindowConfigObserver
    + TextObserver
    + GpuObserver
    + CacheObserver
    + TerminalFeaturesObserver
    + MuxObserver
    + RuntimeObserver
{
}
