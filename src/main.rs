#[cfg(not(windows))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// `mi_option_purge_delay` in mimalloc's `mi_option_t` (same index in v2 and v3).
#[cfg(not(windows))]
const MI_OPTION_PURGE_DELAY: std::ffi::c_int = 15;

#[cfg(not(windows))]
extern "C" {
    fn mi_option_set(option: std::ffi::c_int, value: std::ffi::c_long);
    fn mi_malloc(size: usize) -> *mut std::ffi::c_void;
    fn mi_calloc(count: usize, size: usize) -> *mut std::ffi::c_void;
    fn mi_realloc(ptr: *mut std::ffi::c_void, size: usize) -> *mut std::ffi::c_void;
    fn mi_free(ptr: *mut std::ffi::c_void);
}

fn main() -> std::process::ExitCode {
    #[cfg(not(windows))]
    unsafe {
        mi_option_set(MI_OPTION_PURGE_DELAY, 0);
        tree_sitter::set_allocator(
            Some(mi_malloc),
            Some(mi_calloc),
            Some(mi_realloc),
            Some(mi_free),
        );
    };
    sensez::run_cli().unwrap_or_else(|err| {
        eprintln!("Error: {err:?}");
        std::process::ExitCode::from(2)
    })
}
