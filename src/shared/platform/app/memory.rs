/// Hands the freed pages of the process heap back to the kernel once the app
/// has been idle for a while. Browsing a library allocates and drops enough
/// transient data that the allocator would otherwise keep holding it.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub fn trim() {
    // SAFETY: malloc_trim only walks the allocator's own bookkeeping.
    unsafe {
        libc::malloc_trim(0);
    }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
pub fn trim() {}
