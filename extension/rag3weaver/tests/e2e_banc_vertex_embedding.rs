//! **Vertex AI en face du modèle local**, sur les lots du banc `e2e_banc_bge_m3`.
//!
//! La question (6 septembre 2026, Lucie) : si l'embarqueur local est loin du
//! débit d'un quota Vertex à peine utilisé, on a encore un problème sérieux.
//! `text-embedding-005` (768 dim), jusqu'à 250 textes par appel, `N` appels en
//! vol. Jetons comptés par Vertex lui-même (`statistics.token_count`).
//!
//! ```sh
//! GOOGLE_APPLICATION_CREDENTIALS=../../.vault/vertex-sa.json GOOGLE_CLOUD_PROJECT=lr-hub-472010 \
//!   cargo test --test e2e_banc_vertex_embedding -- --ignored --nocapture
//! ```
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn texte(mots: usize, graine: usize) -> String {
    let vocabulaire = ["fn", "catalogue", "gabarit", "search", "embedding", "ingestion", "relation", "chunk", "scope", "budget", "régime", "démon", "lot", "vector", "index", "table", "uuid", "drain", "flush", "dialect"];
    (0..mots).map(|i| vocabulaire[(i * 7 + graine * 13) % vocabulaire.len()]).collect::<Vec<_>>().join(" ")
}

struct Vertex {
    url: String,
    jeton: String,
    client: reqwest::blocking::Client,
}

impl Vertex {
    fn depuis_env() -> Option<Self> {
        let projet = std::env::var("GOOGLE_CLOUD_PROJECT").ok()?;
        let lieu = std::env::var("VERTEX_EMBED_LOCATION").unwrap_or_else(|_| "us-central1".into());
        let modele = std::env::var("VERTEX_EMBED_MODEL").unwrap_or_else(|_| "text-embedding-005".into());
        let jeton = rag3weaver::gcp_auth::TokenSource::from_env().ok()?.token().ok()?;
        Some(Self {
            url: format!("https://{lieu}-aiplatform.googleapis.com/v1/projects/{projet}/locations/{lieu}/publishers/google/models/{modele}:predict"),
            jeton,
            client: reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(120)).build().ok()?,
        })
    }

    /// Un appel : jusqu'à 250 textes. Rend (vecteurs, jetons comptés par Vertex).
    fn embed(&self, textes: &[String]) -> Result<(Vec<Vec<f32>>, usize), String> {
        let corps = serde_json::json!({
            "instances": textes.iter().map(|t| serde_json::json!({"content": t, "task_type": "RETRIEVAL_DOCUMENT"})).collect::<Vec<_>>(),
            "parameters": {"autoTruncate": true},
        });
        let r = self.client.post(&self.url).bearer_auth(&self.jeton).json(&corps).send().map_err(|e| e.to_string())?;
        let statut = r.status();
        let v: serde_json::Value = r.json().map_err(|e| e.to_string())?;
        if !statut.is_success() {
            return Err(format!("{statut}: {}", v));
        }
        let mut vecs = Vec::new();
        let mut jetons = 0usize;
        for p in v["predictions"].as_array().ok_or("pas de predictions")? {
            let e = &p["embeddings"];
            vecs.push(e["values"].as_array().ok_or("pas de values")?.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect());
            jetons += e["statistics"]["token_count"].as_u64().unwrap_or(0) as usize;
        }
        Ok((vecs, jetons))
    }
}

/// Embarque `textes` par appels de `par_appel`, `en_vol` appels en parallèle.
fn debit(v: &Arc<Vertex>, textes: &[String], par_appel: usize, en_vol: usize) -> (f64, usize, usize) {
    let lots: Vec<Vec<String>> = textes.chunks(par_appel).map(|c| c.to_vec()).collect();
    let file = Arc::new(Mutex::new(lots.into_iter()));
    let total = Arc::new(Mutex::new((0usize, 0usize, Vec::<String>::new())));
    let t = Instant::now();
    let mut fils = Vec::new();
    for _ in 0..en_vol {
        let (v, file, total) = (v.clone(), file.clone(), total.clone());
        fils.push(std::thread::spawn(move || loop {
            let Some(lot) = file.lock().unwrap().next() else { break };
            match v.embed(&lot) {
                Ok((vecs, jetons)) => {
                    let mut t = total.lock().unwrap();
                    t.0 += vecs.len();
                    t.1 += jetons;
                }
                Err(e) => total.lock().unwrap().2.push(e),
            }
        }));
    }
    for f in fils {
        f.join().unwrap();
    }
    let s = t.elapsed().as_secs_f64();
    let t = total.lock().unwrap();
    for e in t.2.iter().take(3) {
        eprintln!("[vertex] erreur : {e}");
    }
    (s, t.0, t.1)
}

#[test]
#[ignore]
fn vertex_jetons_par_seconde() {
    let Some(v) = Vertex::depuis_env() else {
        eprintln!("[vertex] pas d'accès (GOOGLE_CLOUD_PROJECT / GOOGLE_APPLICATION_CREDENTIALS) — sauté");
        return;
    };
    let v = Arc::new(v);
    let en_vol: usize = std::env::var("VERTEX_EN_VOL").ok().and_then(|s| s.parse().ok()).unwrap_or(8);
    eprintln!("[vertex] {} — {en_vol} appels en vol", v.url);

    // Les mêmes lots que le banc local, un appel par lot (≤ 250 textes).
    for (lot, mots) in [(8, 100), (32, 100), (64, 100), (32, 300), (8, 900), (2, 900)] {
        let textes: Vec<String> = (0..lot).map(|i| texte(mots, i)).collect();
        let (s, n, jetons) = debit(&v, &textes, 250, 1);
        eprintln!("[vertex] lot={lot:>2} × {mots} mots ({jetons:>5} jetons Vertex, {n} vecteurs) : un appel, {:.0} ms → {:>6.0} jetons/s", s * 1000.0, jetons as f64 / s);
    }

    // Débit soutenu : les 12 formes inédites du banc (192 textes), par appels
    // de 16 comme le fait notre ingestion, puis de 250, avec `en_vol` en parallèle.
    let corpus: Vec<String> = (0..12).flat_map(|i| (0..16).map(move |j| texte(50 + i * 17, j))).collect();
    for par_appel in [16, 250] {
        let (s, n, jetons) = debit(&v, &corpus, par_appel, en_vol);
        eprintln!("[vertex] corpus 192 textes par appels de {par_appel:>3}, {en_vol} en vol : {jetons} jetons, {n} vecteurs, {s:.2} s → {:.0} jetons/s", jetons as f64 / s);
    }
    // Plus gros : 1 000 textes de 150 mots, comme un drain.
    let gros: Vec<String> = (0..1000).map(|i| texte(150, i)).collect();
    let (s, n, jetons) = debit(&v, &gros, 250, en_vol);
    eprintln!("[vertex] 1 000 textes de 150 mots par appels de 250, {en_vol} en vol : {jetons} jetons, {n} vecteurs, {s:.2} s → {:.0} jetons/s", jetons as f64 / s);
}
