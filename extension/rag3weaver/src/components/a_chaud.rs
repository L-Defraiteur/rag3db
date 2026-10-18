//! **La montre des composants** (marche 3, lot 4) : `components/` et
//! `libs.json` d'un backend, balayés par le [`Scanner`] de la montre de
//! fichiers — la même liste, les mêmes dates douteuses, le même hash —, sans
//! la porte de l'index (une déclaration est une donnée que l'index écarte).
//! Chaque lot de changements part sur le bus du backend en
//! [`Event::ComponentsChanged`], sujet `components`, que le flux de l'IDE
//! relaie à la page d'aperçu.
//!
//! Montée à la demande, par l'aperçu : un backend que personne ne regarde ne
//! balaie rien.
//!
//! [`Scanner`]: crate::watch::Scanner
//! [`Event::ComponentsChanged`]: crate::events::Event::ComponentsChanged

use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use super::LIBS_FILE;

/// Le dossier des déclarations, au niveau du backend.
#[cfg(feature = "code")]
const COMPONENTS_DIR: &str = "components";

/// La montre en service ; l'arrêter la lâche (le fil est réveillé et attendu).
pub struct ComponentsWatch {
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for ComponentsWatch {
    fn drop(&mut self) {
        {
            let (arret, cv) = &*self.stop;
            *arret.lock().unwrap_or_else(|e| e.into_inner()) = true;
            cv.notify_all();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Ce que la montre regarde : les fichiers de `components/` (les dossiers à
/// point exclus) et `libs.json`. Le magasin `.libs/` n'en est pas : il ne
/// change que par `libs add`, qui réécrit `libs.json`.
#[cfg(feature = "code")]
struct ComponentFiles {
    dir: std::path::PathBuf,
}

#[cfg(feature = "code")]
impl crate::code_tools::FileSource for ComponentFiles {
    fn cursor(&self) -> String {
        format!("components:{}", self.dir.display())
    }

    fn list(&self) -> Result<Vec<String>, String> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let kind = entry.file_type()?;
                if kind.is_dir() {
                    walk(&entry.path(), base, out)?;
                } else if kind.is_file() {
                    out.push(crate::workspace_path::relative_or_slash(base, &entry.path()));
                }
            }
            Ok(())
        }
        let mut out = Vec::new();
        let components = self.dir.join(COMPONENTS_DIR);
        if components.is_dir() {
            walk(&components, &self.dir, &mut out).map_err(|e| format!("{}: {e}", components.display()))?;
        }
        if self.dir.join(LIBS_FILE).is_file() {
            out.push(LIBS_FILE.to_string());
        }
        out.sort();
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Option<String>, String> {
        match std::fs::read_to_string(self.dir.join(path)) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{path}: {e}")),
        }
    }
}

/// Un changement qui touche une déclaration (et pas seulement le code d'un
/// composant) : un `.component.json`, un graphe de clic, `libs.json`.
pub fn is_declaration(path: &str) -> bool {
    let script = [".tsx", ".jsx", ".ts", ".js"].iter().any(|e| path.ends_with(e));
    path == LIBS_FILE || !script
}

/// Lance la montre des composants de `backend_dir` sur `bus`. La photographie
/// de départ est prise avant le retour : un fichier changé juste après est vu.
#[cfg(feature = "code")]
pub fn spawn(backend_dir: &Path, bus: crate::events::EventBus) -> Result<ComponentsWatch, String> {
    use std::time::{Duration, Instant};
    let config = crate::watch::WatchConfig::default();
    let source = Arc::new(ComponentFiles { dir: backend_dir.to_path_buf() });
    let mut scanner = crate::watch::Scanner::without_gate(backend_dir.to_path_buf(), source, &config)?;
    let every = Duration::from_millis(config.every_ms.max(10));
    let stop = Arc::new((Mutex::new(false), Condvar::new()));
    let arret = stop.clone();
    let thread = std::thread::Builder::new()
        .name("montre-composants".to_string())
        .spawn(move || loop {
            {
                let (fini, cv) = &*arret;
                let garde = fini.lock().unwrap_or_else(|e| e.into_inner());
                let (garde, _) = cv.wait_timeout_while(garde, every, |f| !*f).unwrap_or_else(|e| e.into_inner());
                if *garde {
                    return;
                }
            }
            let changes = match scanner.scan(Instant::now()) {
                Ok(c) => c,
                Err(e) => {
                    bus.emit(crate::events::Event::Warning {
                        context: "montre des composants".to_string(),
                        message: e,
                    });
                    continue;
                }
            };
            if changes.is_empty() {
                continue;
            }
            let paths: Vec<String> = changes.into_iter().map(|c| c.path).collect();
            let declarations = paths.iter().any(|p| is_declaration(p));
            let at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            bus.emit(crate::events::Event::ComponentsChanged { paths, declarations, at_ms });
        })
        .map_err(|e| format!("montre des composants : {e}"))?;
    Ok(ComponentsWatch { stop, thread: Some(thread) })
}

/// Sans la feature `code`, pas de [`crate::watch::Scanner`] : la montre se
/// refuse en le disant.
#[cfg(not(feature = "code"))]
pub fn spawn(_backend_dir: &Path, _bus: crate::events::EventBus) -> Result<ComponentsWatch, String> {
    Err("la montre des composants demande la feature `code`".to_string())
}
