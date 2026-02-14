pub fn phaedra_version() -> &'static str {
    // See build.rs
    env!("PHAEDRA_CI_TAG")
}

pub fn phaedra_target_triple() -> &'static str {
    // See build.rs
    env!("PHAEDRA_TARGET_TRIPLE")
}
