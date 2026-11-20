//! **The embedder checks itself at start** (20 November 2026). An adapter is not
//! trusted, it is checked : the Intel iGPU of one machine (ARL, anv) computed wrong
//! vectors as soon as it computed a batch, with no error (cosine 0.59 to 0.87),
//! while an AMD APU computed right. At start, the texts of a reference are embedded
//! **alone**, then **in the batches the service computes** (`lot_max` texts, cut
//! under the byte budget), and compared to the vectors of the original ONNX model.
//! Above the model's tolerance, the card is refused, and the reason is said.
//!
//! The reference lives beside the weights (`reference.json` next to `model.bpk`),
//! with the sha256 of the weights it covers : a changed model is never compared to
//! the reference of another one ; it is then `not_verified`, said, not refused.

use serde::{Deserialize, Serialize};

use crate::embedder::{budget_batches, EmbedError};

/// The reference's file name, beside the weights.
pub const REFERENCE_FILE: &str = "reference.json";

/// **The vectors of the original model**, for a set of texts (onnxruntime, CPU, f32).
#[derive(Debug, Clone, Deserialize)]
pub struct Reference {
    /// The sha256 of the `model.bpk` this reference covers.
    pub weights_sha256: String,
    /// The largest absolute gap accepted, alone or in a batch.
    pub tolerance: f32,
    #[serde(alias = "phrases")]
    pub texts: Vec<String>,
    #[serde(alias = "vecteurs")]
    pub vectors: Vec<Vec<f32>>,
}

/// What the check concluded.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Verified,
    Refused,
    NotVerified,
}

/// **The check, as `/sante` says it.**
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelfCheck {
    pub state: CheckState,
    /// The largest absolute gap to the reference, each text alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alone_gap: Option<f32>,
    /// The largest absolute gap to the reference, in the service's batches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_gap: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f32>,
    /// The texts embedded in batches (`lot_max`, the reference's texts cycled).
    #[serde(default)]
    pub batch_texts: usize,
    #[serde(default)]
    pub duration_ms: u64,
    /// Why it is refused or not verified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Eq for SelfCheck {}

impl SelfCheck {
    fn not_verified(reason: String) -> Self {
        Self { state: CheckState::NotVerified, alone_gap: None, batch_gap: None, tolerance: None, batch_texts: 0, duration_ms: 0, reason: Some(reason) }
    }

    /// `Err` with the reason when the card is refused ; a check that could not be
    /// made is said on the error output, and passes.
    pub fn into_result(self, model: &str) -> Result<Self, String> {
        match self.state {
            CheckState::Refused => Err(format!("{model} : carte refusée par l'autovérification : {}", self.reason.clone().unwrap_or_default())),
            CheckState::NotVerified => {
                eprintln!("  autovérification de {model} : non faite ({})", self.reason.clone().unwrap_or_default());
                Ok(self)
            }
            CheckState::Verified => {
                eprintln!(
                    "  autovérification de {model} : juste (écart seul {:.1e}, en lot {:.1e}, tolérance {:.1e}, {} textes en lot, {} ms)",
                    self.alone_gap.unwrap_or(0.0), self.batch_gap.unwrap_or(0.0), self.tolerance.unwrap_or(0.0), self.batch_texts, self.duration_ms
                );
                Ok(self)
            }
        }
    }
}

/// The reference beside these weights, if there is one.
pub fn read_reference(weights: &std::path::Path) -> Result<Option<Reference>, String> {
    let path = weights.with_file_name(REFERENCE_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{} : {e}", path.display())),
    };
    serde_json::from_str(&text).map(Some).map_err(|e| format!("{} illisible : {e}", path.display()))
}

/// The sha256 of these bytes, in hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// **Texts embedded as the service cuts a request** : by length, in batches of
/// `lot_max` at most under `char_budget` bytes, the vectors back in the order given.
pub fn embed_in_service_batches(
    texts: &[String],
    lot_max: usize,
    char_budget: usize,
    embed: impl Fn(&[String]) -> Result<Vec<Vec<f32>>, EmbedError>,
) -> Result<Vec<Vec<f32>>, EmbedError> {
    let mut order: Vec<usize> = (0..texts.len()).collect();
    order.sort_by_key(|&i| texts[i].len());
    let sorted: Vec<String> = order.iter().map(|&i| texts[i].clone()).collect();
    let lens: Vec<usize> = sorted.iter().map(|t| t.len()).collect();
    let mut out: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
    for range in budget_batches(&lens, lot_max, char_budget) {
        let batch = embed(&sorted[range.clone()])?;
        if batch.len() != range.len() {
            return Err(EmbedError::ProviderError(format!("{} vecteurs pour {} textes", batch.len(), range.len())));
        }
        for (k, v) in range.zip(batch) {
            out[order[k]] = Some(v);
        }
    }
    Ok(out.into_iter().map(|v| v.unwrap_or_default()).collect())
}

fn largest_gap(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0f32, |m, g| if g.is_nan() { f32::INFINITY } else { m.max(g) })
}

/// **The check.** `alone` embeds texts one by one ; `batch` embeds a list the way the
/// service does (its own batches) ; `weights_sha256` is computed only when a reference
/// is there.
pub fn check(
    alone: impl Fn(&[String]) -> Result<Vec<Vec<f32>>, EmbedError>,
    batch: impl Fn(&[String]) -> Result<Vec<Vec<f32>>, EmbedError>,
    reference: Option<&Reference>,
    weights_sha256: impl FnOnce() -> Result<String, String>,
    lot_max: usize,
) -> SelfCheck {
    let start = std::time::Instant::now();
    let Some(reference) = reference else {
        return SelfCheck::not_verified(format!("pas de {REFERENCE_FILE} à côté des poids"));
    };
    if reference.texts.is_empty() || reference.texts.len() != reference.vectors.len() {
        return SelfCheck::not_verified(format!("{REFERENCE_FILE} : {} textes pour {} vecteurs", reference.texts.len(), reference.vectors.len()));
    }
    match weights_sha256() {
        Ok(sha) if sha == reference.weights_sha256 => {}
        Ok(sha) => {
            return SelfCheck::not_verified(format!(
                "{REFERENCE_FILE} couvre d'autres poids (sha256 {} ; ces poids : {})",
                &reference.weights_sha256[..reference.weights_sha256.len().min(12)],
                &sha[..12]
            ))
        }
        Err(e) => return SelfCheck::not_verified(format!("empreinte des poids : {e}")),
    }
    let n = reference.texts.len();
    let refused = |reason: String, alone_gap: Option<f32>, batch_gap: Option<f32>, batch_texts: usize| SelfCheck {
        state: CheckState::Refused,
        alone_gap,
        batch_gap,
        tolerance: Some(reference.tolerance),
        batch_texts,
        duration_ms: start.elapsed().as_millis() as u64,
        reason: Some(reason),
    };
    // Each text alone.
    let (mut alone_gap, mut alone_at) = (0f32, 0usize);
    for (i, text) in reference.texts.iter().enumerate() {
        let v = match alone(std::slice::from_ref(text)) {
            Ok(mut v) if v.len() == 1 => v.remove(0),
            Ok(v) => return refused(format!("{} vecteurs pour un texte seul", v.len()), None, None, 0),
            Err(e) => return refused(format!("texte {i} seul : {e}"), None, None, 0),
        };
        let gap = largest_gap(&v, &reference.vectors[i]);
        if gap > alone_gap || gap.is_infinite() {
            (alone_gap, alone_at) = (gap, i);
        }
    }
    // The same texts, cycled to `lot_max`, in the service's batches.
    let indices: Vec<usize> = (0..lot_max.max(n.min(2))).map(|k| k % n).collect();
    let texts: Vec<String> = indices.iter().map(|&i| reference.texts[i].clone()).collect();
    let vectors = match batch(&texts) {
        Ok(v) if v.len() == texts.len() => v,
        Ok(v) => return refused(format!("{} vecteurs pour un lot de {}", v.len(), texts.len()), Some(alone_gap), None, texts.len()),
        Err(e) => return refused(format!("en lot : {e}"), Some(alone_gap), None, texts.len()),
    };
    let (mut batch_gap, mut batch_at) = (0f32, 0usize);
    for (v, &i) in vectors.iter().zip(&indices) {
        let gap = largest_gap(v, &reference.vectors[i]);
        if gap > batch_gap || gap.is_infinite() {
            (batch_gap, batch_at) = (gap, i);
        }
    }
    let tolerance = reference.tolerance;
    let reason = if alone_gap > tolerance {
        Some(format!("texte {alone_at} seul : écart {alone_gap:.2e} à la référence, au-dessus de {tolerance:.1e}"))
    } else if batch_gap > tolerance {
        Some(format!("en lot de {} : écart {batch_gap:.2e} à la référence (texte {batch_at}), au-dessus de {tolerance:.1e}", texts.len()))
    } else {
        None
    };
    SelfCheck {
        state: if reason.is_some() { CheckState::Refused } else { CheckState::Verified },
        alone_gap: Some(alone_gap),
        batch_gap: Some(batch_gap),
        tolerance: Some(tolerance),
        batch_texts: texts.len(),
        duration_ms: start.elapsed().as_millis() as u64,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model : a vector drawn from the text ; `wrong_in_batch` shifts every vector of
    /// a batch of more than one text, like the Intel iGPU.
    fn model(wrong_in_batch: bool) -> impl Fn(&[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        move |texts: &[String]| {
            Ok(texts.iter().map(|t| {
                let mut v = vec![t.len() as f32 / 100.0, (t.bytes().map(u32::from).sum::<u32>() % 97) as f32 / 97.0, 0.5];
                if wrong_in_batch && texts.len() > 1 {
                    v[0] += 0.1;
                }
                v
            }).collect())
        }
    }

    fn reference(sha: &str) -> Reference {
        let texts: Vec<String> = ["un texte court", "a somewhat longer text in another language", &"long ".repeat(200)].iter().map(|s| s.to_string()).collect();
        let vectors = model(false)(&texts).unwrap();
        Reference { weights_sha256: sha.into(), tolerance: 5e-4, texts, vectors }
    }

    /// **A card that computes wrong in a batch is refused, and the reason says where** ;
    /// a right one is verified ; no reference, or the reference of other weights, is
    /// not verified (said, not refused).
    #[test]
    fn a_card_that_computes_wrong_in_a_batch_is_refused() {
        let r = reference("abc");
        let sha = || Ok("abc".to_string());
        let wrong = model(true);
        let c = check(&wrong, |t: &[String]| embed_in_service_batches(t, 32, 8192, &wrong), Some(&r), sha, 32);
        assert_eq!(c.state, CheckState::Refused, "{c:?}");
        assert!(c.reason.as_deref().unwrap_or("").contains("en lot de 32"), "{c:?}");
        assert!(c.batch_gap.unwrap() > 0.09 && c.alone_gap == Some(0.0) && c.batch_texts == 32, "{c:?}");
        assert!(c.clone().into_result("granite-278m").unwrap_err().contains("carte refusée par l'autovérification"));

        let right = model(false);
        let c = check(&right, |t: &[String]| embed_in_service_batches(t, 32, 8192, &right), Some(&r), sha, 32);
        assert_eq!((c.state, c.alone_gap, c.batch_gap), (CheckState::Verified, Some(0.0), Some(0.0)), "{c:?}");

        let c = check(&wrong, &wrong, None, sha, 32);
        assert_eq!(c.state, CheckState::NotVerified, "{c:?}");
        let c = check(&wrong, &wrong, Some(&r), || Ok("other".to_string()), 32);
        assert!(c.state == CheckState::NotVerified && c.reason.as_deref().unwrap_or("").contains("d'autres poids"), "{c:?}");
    }

    /// The service's batches : by length, under the byte budget, the order kept.
    #[test]
    fn texts_are_embedded_in_the_service_batches_and_come_back_in_order() {
        let texts: Vec<String> = ["ccc", "a", "bb", "dddd"].iter().map(|s| s.to_string()).collect();
        let sizes = std::sync::Mutex::new(Vec::new());
        let out = embed_in_service_batches(&texts, 2, 5, |b: &[String]| {
            sizes.lock().unwrap().push(b.iter().map(|t| t.len()).collect::<Vec<_>>());
            Ok(b.iter().map(|t| vec![t.len() as f32]).collect())
        }).unwrap();
        assert_eq!(out, vec![vec![3.0], vec![1.0], vec![2.0], vec![4.0]]);
        assert_eq!(*sizes.lock().unwrap(), vec![vec![1, 2], vec![3], vec![4]]);
    }
}
