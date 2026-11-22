//! The witnesses of [`super::login_shell_path`], with a fake login shell.

use super::login_shell_path;

fn faux_shell(d: &std::path::Path, corps: &str) -> std::path::PathBuf {
    let p = d.join("faux-shell");
    std::fs::write(&p, format!("#!/bin/sh\n{corps}\n")).unwrap();
    std::fs::set_permissions(&p, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    p
}

#[cfg(unix)]
#[test]
fn the_login_path_comes_first_and_the_hosts_stays_after_without_a_double() {
    let d = tempfile::tempdir().unwrap();
    let shell = faux_shell(d.path(), "echo HOME=/home/x; echo PATH=/home/x/.cargo/bin:/usr/bin:/home/x/.nvm/bin");
    assert_eq!(login_shell_path(&shell, "/usr/local/bin:/usr/bin").as_deref(), Some("/home/x/.cargo/bin:/usr/bin:/home/x/.nvm/bin:/usr/local/bin"));
}

#[cfg(unix)]
#[test]
fn a_shell_that_does_not_answer_in_time_changes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let shell = faux_shell(d.path(), "sleep 30");
    let debut = std::time::Instant::now();
    assert_eq!(login_shell_path(&shell, "/usr/bin"), None);
    assert!(debut.elapsed() < std::time::Duration::from_secs(10), "bounded in time");
}
