//! Platform-specific ownership guard shared by tool adapters.
pub fn terminate_with_parent(command: &mut tokio::process::Command) {
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
    #[cfg(not(target_os = "linux"))]
    let _ = command;
}
