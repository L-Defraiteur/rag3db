//! **The fake host of the script witnesses listens before it says where** (the pusher's tty step, 23 November 2026:
//! `plain_stays_attached` met « Connection refused », the fake host had written its `daemon.json` before binding its
//! port). Asked for port 0, it takes a free port, binds it, then writes the address it listens on; whoever reads the
//! file can connect at once. Two fake hosts never share a port.
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn start(dir: &std::path::Path) -> (Child, PathBuf) {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mesures/faux_hote.py");
    let dj = dir.join("daemon.json");
    let child = Command::new("python3")
        .arg(&script)
        .arg("0")
        .arg(dir.join("faux.jsonl"))
        .arg(&dj)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("python3 runs the fake host");
    (child, dj)
}

/// Reads the address as soon as the file holds a whole JSON document.
fn address(dj: &std::path::Path) -> String {
    let t0 = Instant::now();
    loop {
        if let Ok(s) = std::fs::read_to_string(dj) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                return v["address"].as_str().expect("an address").to_string();
            }
        }
        assert!(t0.elapsed() < Duration::from_secs(30), "no daemon.json in 30 s");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn the_fake_host_listens_before_writing_its_address() {
    let d1 = tempfile_dir("a");
    let d2 = tempfile_dir("b");
    let (mut h1, dj1) = start(&d1);
    let (mut h2, dj2) = start(&d2);
    let (a1, a2) = (address(&dj1), address(&dj2));
    // Connected at once, without any retry: the file is written only once the port listens.
    let c1 = std::net::TcpStream::connect(&a1);
    let c2 = std::net::TcpStream::connect(&a2);
    let _ = h1.kill();
    let _ = h2.kill();
    assert!(!a1.ends_with(":0") && a1 != a2, "two fake hosts, two free ports: {a1} {a2}");
    assert!(c1.is_ok() && c2.is_ok(), "the address is written before the port listens: {c1:?} {c2:?}");
}

fn tempfile_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tty-fake-host-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
