//! **The host's commands find what the user's terminal finds** (c7, the terminal's night, 23 Nov 2026): a host started
//! by systemd has `PATH=/usr/local/bin:/usr/bin`, and the commands of its tasks miss cargo, nvm, pyenv, bun… The host
//! reads once, at its start, the `PATH` of the user's login shell (`$SHELL -l -c env`, bounded in time, stdin closed),
//! and puts its entries first, the host's own after, without a double.

use std::path::Path;

/// The `PATH` of `shell`'s login environment, merged before `current`; `None` if the shell does not answer in time
/// or says no `PATH`.
pub fn login_shell_path(shell: &Path, current: &str) -> Option<String> {
    use std::process::{Command, Stdio};
    let mut enfant = Command::new(shell).args(["-l", "-c", "env"]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut sortie = enfant.stdout.take()?;
    let lu = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut sortie, &mut s);
        s
    });
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match enfant.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < fin => std::thread::sleep(std::time::Duration::from_millis(20)),
            _ => {
                let _ = enfant.kill();
                let _ = enfant.wait();
                return None;
            }
        }
    }
    let env = lu.join().ok()?;
    let login = env.lines().find_map(|l| l.strip_prefix("PATH="))?.trim();
    if login.is_empty() {
        return None;
    }
    let mut vus: Vec<&str> = Vec::new();
    for e in login.split(':').chain(current.split(':')) {
        if !e.is_empty() && !vus.contains(&e) {
            vus.push(e);
        }
    }
    Some(vus.join(":"))
}

/// At the host's start, before any thread: its `PATH` becomes the login shell's, merged (`$SHELL`, else none).
pub fn adopt_the_login_path() {
    let (Some(shell), current) = (std::env::var_os("SHELL"), std::env::var("PATH").unwrap_or_default()) else { return };
    if let Some(p) = login_shell_path(Path::new(&shell), &current) {
        if p != current {
            eprintln!("rag3weaver : PATH du shell de connexion ({}) adopté pour les commandes", Path::new(&shell).display());
            std::env::set_var("PATH", p);
        }
    }
}

#[cfg(test)]
#[path = "login_path_tests.rs"]
mod tests;
