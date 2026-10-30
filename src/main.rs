#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// `mi_option_purge_delay` in mimalloc's `mi_option_t` (same index in v2 and v3).
const MI_OPTION_PURGE_DELAY: std::ffi::c_int = 15;

extern "C" {
    fn mi_option_set(option: std::ffi::c_int, value: std::ffi::c_long);
}

fn main() -> std::process::ExitCode {
    // With the default delayed purge a long-lived server kept a scan's freed pages.
    unsafe { mi_option_set(MI_OPTION_PURGE_DELAY, 0) };
    sensez::run_cli().unwrap_or_else(|err| {
        eprintln!("Error: {err:?}");
        std::process::ExitCode::from(2)
    })
}
