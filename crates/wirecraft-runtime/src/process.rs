//! Platform-specific process setup shared by tool adapters.

/// Ties a tool to this process and keeps it out of sight. The desktop's backend has no
/// console on Windows, where each console tool it starts would otherwise open a window.
pub fn supervise(command: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(target_os = "linux")]
    {
        // SAFETY: this hook uses only async-signal-safe syscalls between fork and exec.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() == 1 {
                    return Err(std::io::Error::other("Backend parent exited"));
                }
                Ok(())
            });
        }
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    let _ = command;
}
