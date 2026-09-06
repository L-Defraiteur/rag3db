//! **Granite embedding, 107m et 278m, multilingues, sur burn.**
//!
//! Deux encodeurs XLM-RoBERTa d'IBM (Apache-2.0), entraînés texte **et code**,
//! français compris : `granite-embedding-107m-multilingual` (6 couches × 384,
//! 384 dim) et `granite-embedding-278m-multilingual` (12 × 768, 768 dim). Le
//! même vocabulaire que BGE-M3, quatre à quinze fois moins de calcul. C'est le
//! premier étage d'indexation que l'on cherchait le 6 septembre 2026 : la
//! classe de bge-base (le modèle de ragforge), mais multilingue — un
//! vocabulaire ne se calcule pas, seul le corps coûte.
//!
//! Graphes générés par burn-onnx depuis les `model.onnx` des dépôts HF
//! (`generated/granite_{107m,278m}_onnx.rs`) : le `forward` rend
//! `last_hidden_state` et un `pooler_output` (`tanh(Linear(CLS))`) qu'on
//! **ignore** — la fiche des modèles fait `hidden[:, 0]` puis une
//! normalisation L2, et c'est ce qu'on fait ici. Tokenizer XLM-R, `<pad>` = 1,
//! 512 jetons.
//!
//! Un seul fichier pour les deux, sur le motif de `burn_xlmr_reranker.rs` :
//! un trait qui abstrait le graphe, une structure générique, une macro pour
//! les deux types publics.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use burn::prelude::*;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::burn_device::{BurnDevice, BurnRole};
use crate::embedder::{EmbedError, Embedder};

/// `<pad>` de XLM-R. Le `config.json` dit 0, le tokenizer dit 1 : c'est le
/// tokenizer qui pose les ids.
const PAD_TOKEN_ID: u32 = 1;
/// `max_position_embeddings` = 514, moins l'offset XLM-R de 2.
const MAX_SEQ_LEN: usize = 512;

/// Ce qu'un graphe Granite généré sait faire, et sa dimension.
pub trait GraniteGraph: Send + Sync + 'static {
    const DIM: usize;
    const NOM: &'static str;
    const NOM_LONG: &'static str;
    /// Le graphe chargé dans la précision voulue (voir `burn_device::charger_burnpack`).
    fn charger(device: &Device, weights: &[u8], precision: Option<burn::tensor::FloatDType>) -> Result<Self, String>
    where
        Self: Sized;
    /// `[B, S, DIM]`, les hidden states par jeton.
    fn hidden(&self, input_ids: Tensor<2, Int>, attention_mask: Tensor<2, Int>) -> Tensor<3>;
}

macro_rules! granite_graph {
    ($ty:ident, $module:ident, $dim:expr, $nom:expr, $nom_long:expr) => {
        pub struct $ty(crate::$module::Model);

        impl GraniteGraph for $ty {
            const DIM: usize = $dim;
            const NOM: &'static str = $nom;
            const NOM_LONG: &'static str = $nom_long;
            fn charger(device: &Device, weights: &[u8], precision: Option<burn::tensor::FloatDType>) -> Result<Self, String> {
                crate::burn_device::charger_burnpack(crate::$module::Model::new(device), weights, $nom, precision).map(Self)
            }
            fn hidden(&self, input_ids: Tensor<2, Int>, attention_mask: Tensor<2, Int>) -> Tensor<3> {
                let (hidden, _pooler) = self.0.forward(input_ids, attention_mask);
                hidden
            }
        }
    };
}

granite_graph!(
    Granite107mGraph,
    granite_107m_onnx,
    384,
    "granite-107m",
    "ibm-granite/granite-embedding-107m-multilingual (burn)"
);
granite_graph!(
    Granite278mGraph,
    granite_278m_onnx,
    768,
    "granite-278m",
    "ibm-granite/granite-embedding-278m-multilingual (burn)"
);

/// L'embarqueur, générique sur le graphe.
pub struct GraniteEmbedder<G: GraniteGraph> {
    graph: G,
    tokenizer: Mutex<Tokenizer>,
    troncatures: AtomicUsize,
    device: Device,
}

impl<G: GraniteGraph> GraniteEmbedder<G> {
    pub fn from_bytes(weights: &[u8], tokenizer_path: impl AsRef<Path>, device: BurnDevice) -> Result<Self, EmbedError> {
        let device = device.or_role(BurnRole::Embedder).resolve();

        let mut tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token: "<pad>".to_string(),
            pad_id: PAD_TOKEN_ID,
            ..Default::default()
        }));
        tokenizer
            .with_truncation(Some(TruncationParams { max_length: MAX_SEQ_LEN, ..Default::default() }))
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer truncation: {e}")))?;

        // **Les poids dans la précision de la carte**, comme BGE-M3 : sans
        // adaptateur ils restent f32 et le graphe entier calcule en f32,
        // quoi que la carte ait pour défaut (le MiniLM multilingue, chargé par
        // `from_bytes`, ne profite pas de Flex32 pour cette raison).
        let graph = G::charger(&device, weights, crate::burn_device::float_dtype_voulu()).map_err(EmbedError::ProviderError)?;

        Ok(Self { graph, tokenizer: Mutex::new(tokenizer), troncatures: AtomicUsize::new(0), device })
    }

    pub fn from_files(weights: impl AsRef<Path>, tokenizer_path: impl AsRef<Path>, device: BurnDevice) -> Result<Self, EmbedError> {
        let weights = std::fs::read(weights.as_ref())
            .map_err(|e| EmbedError::ProviderError(format!("lecture de {} : {e}", weights.as_ref().display())))?;
        Self::from_bytes(&weights, tokenizer_path, device)
    }

    /// Le nom long, celui de la fiche du modèle.
    pub fn name(&self) -> &'static str {
        G::NOM_LONG
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let encodings = {
            let tokenizer = self.tokenizer.lock().map_err(|_| EmbedError::ProviderError("tokenizer lock poisoned".into()))?;
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            tokenizer.encode_batch(refs, true).map_err(|e| EmbedError::ProviderError(format!("tokenize: {e}")))?
        };
        let tronquees = encodings.iter().filter(|e| !e.get_overflowing().is_empty()).count();
        if tronquees > 0 {
            self.troncatures.fetch_add(tronquees, Ordering::Relaxed);
        }

        let batch = encodings.len();
        let seq = encodings[0].get_ids().len();
        let mut ids = Vec::with_capacity(batch * seq);
        let mut mask = Vec::with_capacity(batch * seq);
        for e in &encodings {
            ids.extend(e.get_ids().iter().map(|&x| x as i32));
            mask.extend(e.get_attention_mask().iter().map(|&x| x as i32));
        }
        let input_ids = Tensor::<2, Int>::from_data(TensorData::new(ids, [batch, seq]), &self.device);
        let attention_mask = Tensor::<2, Int>::from_data(TensorData::new(mask, [batch, seq]), &self.device);

        // CLS puis L2, comme la fiche (`hidden[:, 0]`, `F.normalize`).
        let hidden = self.graph.hidden(input_ids, attention_mask);
        let cls: Tensor<2> = hidden.slice([0..batch, 0..1]).squeeze_dim(1);
        let norms = cls.clone().powf_scalar(2.0).sum_dim(1).sqrt();
        let normalized = cls / norms;

        // Rendu en f32 quelle que soit la précision de calcul (voir BGE-M3).
        let data = normalized.cast(burn::tensor::FloatDType::F32).to_data().convert::<f32>();
        let dim = data.shape[1];
        let flat: Vec<f32> = data.try_to_vec().map_err(|e| EmbedError::ProviderError(format!("granite to_vec: {e:?}")))?;
        Ok((0..batch).map(|i| flat[i * dim..(i + 1) * dim].to_vec()).collect())
    }
}

impl<G: GraniteGraph> Embedder for GraniteEmbedder<G> {
    fn name(&self) -> &str {
        G::NOM
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        self.embed_batch(texts)
    }

    fn dim(&self) -> usize {
        G::DIM
    }

    fn troncatures(&self) -> Option<(usize, usize)> {
        Some((self.troncatures.load(Ordering::Relaxed), MAX_SEQ_LEN))
    }

    /// 128 séquences de 512. L'attention est fusionnée (masque booléen, voir
    /// `patch_attention.py`), donc les scores ne sont plus matérialisés — sauf
    /// par l'autotune, qui essaie aussi la voie naïve : à 256 séquences elle
    /// demande 3,2 Go d'un tenseur, au-dessus de la limite d'un tampon wgpu,
    /// et le serveur panique avant de se rattraper (6 septembre 2026). À 128
    /// c'est 1,6 Go, sous la limite, et la carte est saturée de toute façon.
    fn budget_conseille(&self) -> Option<(usize, usize)> {
        Some((128, MAX_SEQ_LEN))
    }
}

/// granite-embedding-107m-multilingual : 6 couches × 384, 384 dim.
pub type BurnGranite107m = GraniteEmbedder<Granite107mGraph>;
/// granite-embedding-278m-multilingual : 12 couches × 768, 768 dim.
pub type BurnGranite278m = GraniteEmbedder<Granite278mGraph>;
