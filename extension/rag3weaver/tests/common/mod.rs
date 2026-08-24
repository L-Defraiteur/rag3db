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

#[cfg(feature = "burn-embedder")]
pub mod burn {
    use std::path::PathBuf;
    use std::sync::{Arc, LazyLock};

    use rag3weaver::burn_bge_m3_embedder::{BurnBgeM3Embedder, BurnDevice};
    use rag3weaver::burn_minilm_embedder::BurnMiniLmEmbedder;
    use rag3weaver::burn_multilingual_minilm_embedder::BurnMultilingualMiniLmEmbedder;
    use rag3weaver::burn_reranker::BurnMiniLmReranker;
    use rag3weaver::burn_xlmr_reranker::{BurnBgeRerankerV2M3, BurnMMiniLmReranker};
    use rag3weaver::embedder::Embedder;

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

    /// BGE-M3 sur burn (1024 d dense + sparse appris, un seul forward,
    /// multilingue). 2,2 Go, quelques secondes d'I/O ; chargé une fois par binaire.
    pub static BGE_M3: LazyLock<Arc<BurnBgeM3Embedder>> = LazyLock::new(|| {
        let t0 = std::time::Instant::now();
        let bpk = artifact("RAG3WEAVER_BGE_M3_BPK", "bge-m3", "model.bpk");
        let tok = artifact("RAG3WEAVER_BGE_M3_TOKENIZER", "bge-m3", "tokenizer.json");
        eprintln!("▸ Loading BGE-M3 on burn (wgpu) from {}...", bpk.display());
        let bytes = std::fs::read(&bpk).expect("read burnpack");
        let e = BurnBgeM3Embedder::from_bytes(&bytes, &tok, BurnDevice::default())
            .expect("build BurnBgeM3Embedder");
        eprintln!("  loaded in {:?}", t0.elapsed());
        Arc::new(e)
    });

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
