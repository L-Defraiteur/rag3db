//! Embedders partagés par les suites E2E.
//!
//! Chemin produit = burn (wgpu). Les poids ne sont jamais dans git : ils se
//! chargent depuis `~/.cache/rag3weaver/{bge-m3,minilm,multilingual-minilm}/` ou
//! depuis les variables `RAG3WEAVER_BGE_M3_BPK` / `_TOKENIZER`, `RAG3WEAVER_MINILM_BPK` /
//! `_TOKENIZER`, `RAG3WEAVER_MULTILINGUAL_MINILM_BPK` / `_TOKENIZER` (voir
//! `generated/README.md` pour les récupérer depuis HF).
//!
//! candle n'apparaît plus dans les E2E : il reste l'oracle de parité, dans
//! `examples/*_reference.rs` et `examples/burn_*_vs_candle.rs`.
#![allow(dead_code)]

/// Serveur SSE local (std pur) pour les suites `openai_llm_*` : aucun réseau,
/// aucun secret. Sous la feature qui l'utilise, pour ne rien compiler ailleurs.
#[cfg(feature = "openai-llm")]
pub mod fake_sse;

#[cfg(feature = "burn-embedder")]
pub mod burn {
    use std::path::PathBuf;
    use std::sync::{Arc, LazyLock};

    use rag3weaver::burn_bge_m3_embedder::{BurnBgeM3Embedder, BurnDevice};
    use rag3weaver::burn_minilm_embedder::BurnMiniLmEmbedder;
    use rag3weaver::burn_multilingual_minilm_embedder::BurnMultilingualMiniLmEmbedder;
    use rag3weaver::burn_reranker::BurnMiniLmReranker;
    use rag3weaver::burn_xlmr_reranker::{BurnBgeRerankerV2M3, BurnMMiniLmReranker};
    use rag3weaver::embedder::{DualEmbedder, EmbedError, Embedder, SparseEmbedder};
    use rag3weaver::sparse_index::SparseVector;
    #[cfg(feature = "daemon")]
    use rag3weaver::daemon::DaemonEmbedder;

    fn artifact(env_var: &str, model_dir: &str, default_name: &str) -> PathBuf {
        let path = std::env::var(env_var).map(PathBuf::from).unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").expect("HOME"))
                .join(".cache/rag3weaver")
                .join(model_dir)
                .join(default_name)
        });
        if !path.exists() {
            panic!(
                "artefact {model_dir} introuvable : {}\n\
                 Définir {env_var}, ou le récupérer une fois — voir generated/README.md.",
                path.display()
            );
        }
        path
    }

    /// all-MiniLM-L6-v2 sur burn (384 d, dense). Chargé une fois par binaire.
    pub static MINILM: LazyLock<Arc<dyn Embedder>> = LazyLock::new(|| {
        let t0 = std::time::Instant::now();
        let bpk = artifact("RAG3WEAVER_MINILM_BPK", "minilm", "model.bpk");
        let tok = artifact("RAG3WEAVER_MINILM_TOKENIZER", "minilm", "tokenizer.json");
        eprintln!("▸ Loading all-MiniLM-L6-v2 on burn (wgpu) from {}...", bpk.display());
        let e = BurnMiniLmEmbedder::from_files(&bpk, &tok, BurnDevice::default())
            .expect("build BurnMiniLmEmbedder");
        eprintln!("  loaded in {:?}", t0.elapsed());
        Arc::new(e)
    });

    /// paraphrase-multilingual-MiniLM-L12-v2 sur burn (384 d, dense, multilingue —
    /// vocabulaire XLM-R sur un corps BERT 12 couches). 470 Mo ; chargé une fois par binaire.
    pub static MULTILINGUAL_MINILM: LazyLock<Arc<dyn Embedder>> = LazyLock::new(|| {
        let t0 = std::time::Instant::now();
        let bpk = artifact("RAG3WEAVER_MULTILINGUAL_MINILM_BPK", "multilingual-minilm", "model.bpk");
        let tok = artifact("RAG3WEAVER_MULTILINGUAL_MINILM_TOKENIZER", "multilingual-minilm", "tokenizer.json");
        eprintln!("▸ Loading paraphrase-multilingual-MiniLM-L12-v2 on burn (wgpu) from {}...", bpk.display());
        let e = BurnMultilingualMiniLmEmbedder::from_files(&bpk, &tok, BurnDevice::default())
            .expect("build BurnMultilingualMiniLmEmbedder");
        eprintln!("  loaded in {:?}", t0.elapsed());
        Arc::new(e)
    });

    /// **BGE-M3, ici ou ailleurs.**
    ///
    /// Le modèle fait 2,2 Go. Chaque binaire de test qui le charge paie
    /// quelques secondes seul — et beaucoup plus dès que plusieurs le font en
    /// même temps : une passe mesurée le 29 août 2026 rechargeait BGE-M3 sept
    /// fois, pour 2 047 s de chargement contre 1 111 s de tests. Sept processus
    /// tirant chacun 2,2 Go vers la même carte.
    ///
    /// D'où ce choix, invisible pour les sites d'appel : s'il y a un démon,
    /// on lui parle ; sinon on charge ici. Les deux formes implémentent les
    /// trois traits, donc `Arc<Bge>` se coerce en `Arc<dyn Embedder>`,
    /// `dyn DualEmbedder` ou `dyn SparseEmbedder` comme avant.
    pub enum Bge {
        /// Chargé dans ce processus.
        Local(BurnBgeM3Embedder),
        /// Servi par `rag3weaver-embeddings`.
        Distant(DaemonEmbedder),
    }

    impl Embedder for Bge {
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
            match self {
                Self::Local(e) => e.embed(texts),
                Self::Distant(e) => e.embed(texts),
            }
        }
        fn dim(&self) -> usize {
            match self {
                Self::Local(e) => Embedder::dim(e),
                Self::Distant(e) => Embedder::dim(e),
            }
        }
        fn is_mock(&self) -> bool {
            match self {
                Self::Local(e) => e.is_mock(),
                Self::Distant(e) => e.is_mock(),
            }
        }
        fn name(&self) -> &str {
            match self {
                Self::Local(e) => e.name(),
                Self::Distant(e) => e.name(),
            }
        }
    }

    impl DualEmbedder for Bge {
        fn embed_dual(
            &self,
            texts: &[String],
        ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
            match self {
                Self::Local(e) => e.embed_dual(texts),
                Self::Distant(e) => e.embed_dual(texts),
            }
        }
        fn dim(&self) -> usize {
            match self {
                Self::Local(e) => DualEmbedder::dim(e),
                Self::Distant(e) => DualEmbedder::dim(e),
            }
        }
    }

    impl SparseEmbedder for Bge {
        fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
            match self {
                Self::Local(e) => e.embed_sparse(texts),
                Self::Distant(e) => e.embed_sparse(texts),
            }
        }
    }

    /// BGE-M3 (1024 d dense + creux appris, un seul forward, multilingue),
    /// partagé par toutes les suites. Voir [`Bge`] pour le local/distant.
    pub static BGE_M3: LazyLock<Arc<Bge>> = LazyLock::new(|| Arc::new(bge_m3()));

    fn bge_m3() -> Bge {
        #[cfg(feature = "daemon")]
        if std::env::var_os("RAG3WEAVER_SANS_DEMON").is_none() {
            match bge_m3_par_le_demon() {
                Ok(d) => return Bge::Distant(d),
                // **Jamais bloquant.** Un démon absent, une carte prise, un
                // binaire pas construit : on charge ici et on le dit. Le démon
                // est une économie, pas une dépendance.
                Err(raison) => eprintln!("▸ démon d'embedding écarté : {raison}"),
            }
        }
        Bge::Local(bge_m3_ici())
    }

    fn bge_m3_ici() -> BurnBgeM3Embedder {
        let t0 = std::time::Instant::now();
        let bpk = artifact("RAG3WEAVER_BGE_M3_BPK", "bge-m3", "model.bpk");
        let tok = artifact("RAG3WEAVER_BGE_M3_TOKENIZER", "bge-m3", "tokenizer.json");
        eprintln!("▸ Loading BGE-M3 on burn (wgpu) from {}...", bpk.display());
        let bytes = std::fs::read(&bpk).expect("read burnpack");
        let e = BurnBgeM3Embedder::from_bytes(&bytes, &tok, BurnDevice::default())
            .expect("build BurnBgeM3Embedder");
        eprintln!("  loaded in {:?}", t0.elapsed());
        e
    }

    /// S'attacher au démon, ou le lancer. L'adresse est fixe **exprès** : c'est
    /// ce qui fait que le binaire de test suivant retrouve celui d'avant.
    #[cfg(feature = "daemon")]
    fn bge_m3_par_le_demon() -> Result<DaemonEmbedder, String> {
        let adresse = std::env::var("RAG3WEAVER_EMBEDDINGS_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7878".to_string());
        let serveur = DaemonEmbedder::serveur(&adresse, env!("CARGO_BIN_EXE_rag3weaver-embeddings"))
            .journal_dans(std::env::temp_dir().join("rag3weaver-demons"));

        let t0 = std::time::Instant::now();
        let d = DaemonEmbedder::assurer(&serveur).map_err(|e| e.to_string())?;
        let id = d.identite().clone();

        // **On vérifie ce qu'on trouve.** Un démon d'une version précédente
        // peut répondre sur ce port sans servir tout ce qu'on attend, et ses
        // 404 arriveraient au milieu d'une suite plutôt qu'ici.
        if id.modele != "bge-m3" || id.dim != 1024 || !id.dual || !id.sparse || id.factice {
            return Err(format!("le démon de {adresse} ne sert pas ce qu'on attend : {id:?}"));
        }
        eprintln!("▸ BGE-M3 par le démon sur {adresse} (attaché en {:?})", t0.elapsed());
        Ok(d)
    }

    /// cross-encoder/ms-marco-MiniLM-L-6-v2 sur burn (un logit par paire
    /// requête/passage, anglais). 90 Mo ; chargé une fois par binaire.
    pub static MSMARCO_RERANKER: LazyLock<Arc<BurnMiniLmReranker>> = LazyLock::new(|| {
        let t0 = std::time::Instant::now();
        let bpk = artifact("RAG3WEAVER_MSMARCO_BPK", "msmarco-minilm", "model.bpk");
        let tok = artifact("RAG3WEAVER_MSMARCO_TOKENIZER", "msmarco-minilm", "tokenizer.json");
        eprintln!("▸ Loading ms-marco-MiniLM-L-6-v2 on burn (wgpu) from {}...", bpk.display());
        let r = BurnMiniLmReranker::from_files(&bpk, &tok, BurnDevice::default())
            .expect("build BurnMiniLmReranker");
        eprintln!("  loaded in {:?}", t0.elapsed());
        Arc::new(r)
    });

    /// cross-encoder/mmarco-mMiniLMv2-L12-H384-v1 sur burn (XLM-RoBERTa, un logit
    /// par paire, multilingue — français inclus). 470 Mo ; chargé une fois par binaire.
    pub static MMARCO_RERANKER: LazyLock<Arc<BurnMMiniLmReranker>> = LazyLock::new(|| {
        let t0 = std::time::Instant::now();
        let bpk = artifact("RAG3WEAVER_MMARCO_BPK", "mmarco-mminilm", "model.bpk");
        let tok = artifact("RAG3WEAVER_MMARCO_TOKENIZER", "mmarco-mminilm", "tokenizer.json");
        eprintln!("▸ Loading mmarco-mMiniLMv2-L12-H384-v1 on burn (wgpu) from {}...", bpk.display());
        let r = BurnMMiniLmReranker::from_files(&bpk, &tok, BurnDevice::default())
            .expect("build BurnMMiniLmReranker");
        eprintln!("  loaded in {:?}", t0.elapsed());
        Arc::new(r)
    });

    /// BAAI/bge-reranker-v2-m3 sur burn (XLM-RoBERTa 24 couches, multilingue).
    /// 2,2 Go, quelques secondes d'I/O ; chargé une fois par binaire.
    pub static BGE_RERANKER: LazyLock<Arc<BurnBgeRerankerV2M3>> = LazyLock::new(|| {
        let t0 = std::time::Instant::now();
        let bpk = artifact("RAG3WEAVER_BGE_RERANKER_BPK", "bge-reranker-v2-m3", "model.bpk");
        let tok = artifact("RAG3WEAVER_BGE_RERANKER_TOKENIZER", "bge-reranker-v2-m3", "tokenizer.json");
        eprintln!("▸ Loading bge-reranker-v2-m3 on burn (wgpu) from {}...", bpk.display());
        let r = BurnBgeRerankerV2M3::from_files(&bpk, &tok, BurnDevice::default())
            .expect("build BurnBgeRerankerV2M3");
        eprintln!("  loaded in {:?}", t0.elapsed());
        Arc::new(r)
    });
}

#[cfg(feature = "burn-ocr")]
pub mod burn_ocr {
    use std::sync::{Arc, LazyLock};

    use rag3weaver::burn_device::BurnDevice;
    use rag3weaver::burn_ppocr::BurnPpOcr;

    /// PP-OCRv6 tiny (det + rec) sur burn. 6 Mo ; chargé une fois par binaire.
    /// Dossier : `RAG3WEAVER_PPOCR_DIR` ou `~/.cache/rag3weaver/ppocrv6-tiny`
    /// (`det.bpk`, `rec.bpk`, `dict.txt` — voir generated/README.md).
    pub static PPOCR: LazyLock<Arc<BurnPpOcr>> = LazyLock::new(|| {
        let t0 = std::time::Instant::now();
        let dir = BurnPpOcr::default_cache_dir();
        for name in ["det.bpk", "rec.bpk", "dict.txt"] {
            let path = dir.join(name);
            if !path.exists() {
                panic!(
                    "artefact PP-OCRv6 tiny introuvable : {}\n\
                     Définir RAG3WEAVER_PPOCR_DIR, ou le récupérer une fois — voir generated/README.md.",
                    path.display()
                );
            }
        }
        eprintln!("▸ Loading PP-OCRv6 tiny on burn (wgpu) from {}...", dir.display());
        let ocr = BurnPpOcr::from_cache_dir(&dir, BurnDevice::default()).expect("build BurnPpOcr");
        eprintln!("  loaded in {:?}", t0.elapsed());
        Arc::new(ocr)
    });
}
