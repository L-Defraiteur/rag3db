//! **The host says its epoch** (ticket of 24 Nov 2026, « après une relance de l'hôte, le terminal ne voit plus rien »: a
//! new host numbers its stream from 1 again, and a terminal resuming by `Last-Event-ID` waited for numbers of the old
//! series). Each host draws an epoch when it starts, gives it in `/sante`, and opens each subscription to its stream
//! with a frame `event: host` carrying it, without an `id:` so that the series is not shifted. Two hosts started one
//! after the other give two epochs. A module of its own.

use crate::daemon_host::DaemonHost;
use serde_json::Value;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

impl Write for Buffer {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The host's epoch by `/sante`, and the first frame of a subscription to its stream.
fn epoch_and_first_frame(host: &DaemonHost) -> (String, String) {
    let (code, sante) = host.handle("GET", "/sante", None, b"");
    assert_eq!(code, 200, "{sante}");
    let epoch = sante["epoch"].as_str().unwrap_or_default().to_string();
    let buffer = Buffer(Arc::new(Mutex::new(Vec::new())));
    host.ide_flux().abonner_une_fenetre(Box::new(buffer.clone()), None, None);
    let until = Instant::now() + Duration::from_secs(5);
    let text = loop {
        let t = String::from_utf8_lossy(&buffer.0.lock().unwrap()).to_string();
        if t.contains("event: host") || Instant::now() > until {
            break t;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let frame = text.split("\n\n").find(|f| f.starts_with("event:")).unwrap_or_default().to_string();
    (epoch, frame)
}

#[test]
fn each_host_says_its_epoch_in_sante_and_first_in_its_stream() {
    let first = DaemonHost::new();
    let (epoch, frame) = epoch_and_first_frame(&first);
    assert!(!epoch.is_empty(), "/sante gives the host's epoch");
    assert!(frame.starts_with("event: host\n"), "the first frame of the stream is the host's: {frame:?}");
    assert!(!frame.lines().any(|l| l.starts_with("id:")), "without an id, the series is not shifted: {frame:?}");
    let data: Value = frame.lines().find_map(|l| l.strip_prefix("data: ")).and_then(|d| serde_json::from_str(d).ok()).unwrap_or_default();
    assert_eq!(data["epoch"].as_str(), Some(epoch.as_str()), "the same epoch as /sante: {frame:?}");
    assert_eq!(data["pid"].as_u64(), Some(u64::from(std::process::id())), "{frame:?}");
    let (second, _) = epoch_and_first_frame(&DaemonHost::new());
    assert!(!second.is_empty() && second != epoch, "a host started after it has another epoch: {epoch} then {second}");
}
