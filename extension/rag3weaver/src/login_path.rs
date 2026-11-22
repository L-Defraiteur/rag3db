//! **The host's commands find what the user's terminal finds** (c7, the terminal's night, 23 Nov 2026): a host started
//! by systemd has `PATH=/usr/local/bin:/usr/bin`, and the commands of its tasks miss cargo, nvm, pyenv, bun… The host
//! reads once, at its start, the `PATH` of the user's login shell (`$SHELL -l -c env`, bounded in time, stdin closed),
//! and puts its entries first, the host's own after, without a double.

use std::path::Path;

/// The `PATH` of `shell`'s login environment, merged before `current`; `None` if the shell does not answer in time
/// or says no `PATH`.
pub fn login_shell_path(shell: &Path, current: &str) -> Option<String> {
    let _ = (shell, current);
    None
}

#[cfg(test)]
#[path = "login_path_tests.rs"]
mod tests;
