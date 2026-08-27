//! PP-OCRv6 tiny sur [burn](https://burn.dev) — détection DBNet + reconnaissance
//! CTC, sans Paddle, sans onnxruntime, sans OpenCV.
//!
//! Deux graphes générés par burn-onnx depuis les ONNX officiels de PaddlePaddle
//! (`generated/ppocrv6_tiny_det_onnx.rs`, `generated/ppocrv6_tiny_rec_onnx.rs`),
//! des poids `.bpk` chargés depuis un dossier (`det.bpk`, `rec.bpk`) et le
//! dictionnaire de 6904 caractères (`dict.txt`) extrait du `inference.yml` du
//! modèle de reconnaissance. Tout le pré/post-traitement de PaddleOCR est
//! réécrit ici en Rust pur, sur les [`OcrImage`] RGB du trait [`Ocr`] :
//!
//! * **det** : redimensionnement au multiple de 32 (`limit_type min`, 736 par
//!   défaut, plafond 4000), normalisation ImageNet **sur des canaux BGR** (c'est
//!   la convention PaddleOCR, on la reproduit à l'octet), forward, carte de
//!   probabilité `[1,1,H,W]` ; post-DB : binarisation, composantes connexes
//!   8-voisinage, boîte englobante axée, score = moyenne de la carte dans la
//!   boîte, unclip (`d = aire × ratio / périmètre`), retour aux pixels de l'image
//!   d'origine, tri en ordre de lecture ([`sort_reading_order`]).
//! * **rec** : crop du rectangle (rotation de 90° si `h/w ≥ 1.5`), hauteur 48,
//!   largeur `ceil(48·w/h)` plafonnée à `48 × max_wh_ratio` du lot, `(x/255 − 0.5)/0.5`
//!   en BGR, padding zéro à droite, lots triés par ratio, décodage CTC
//!   (blank = 0, dictionnaire à partir de 1, espace en dernier).
//!
//! Dette connue : la boîte est axée (pas de `minAreaRect`) — le texte incliné
//! est détecté mais recadré droit, donc mal lu. Les valeurs par défaut viennent
//! des `inference.yml` publiés avec les modèles (voir `generated/README.md`).
//!
//! # Exemple
//!
//! ```ignore
//! let ocr = BurnPpOcr::from_cache_dir(BurnPpOcr::default_cache_dir(), BurnDevice::default())?;
//! let image = OcrImage::decode(&std::fs::read("scan.png")?)?;
//! let out = ocr.recognize(&image)?;
//! println!("{}", out.text());
//! ```

use std::path::{Path, PathBuf};

use burn::prelude::*;
use image::imageops::{self, FilterType};
use image::RgbImage;

use crate::burn_device::BurnDevice;
use crate::ocr::{sort_reading_order, Ocr, OcrError, OcrImage, OcrLine, OcrOutput};
use crate::ppocrv6_tiny_det_onnx::Model as DetGraph;
use crate::ppocrv6_tiny_rec_onnx::Model as RecGraph;

/// Nom rendu par [`Ocr::name`].
pub const MODEL_NAME: &str = "ppocrv6-tiny";

/// Moyenne ImageNet, appliquée telle quelle aux canaux **B, G, R** (ordre PaddleOCR).
const DET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const DET_STD: [f32; 3] = [0.229, 0.224, 0.225];

/// Composante de moins de `MIN_SIZE` px de côté : rejetée avant le score
/// (`DBPostProcess.min_size`) ; après unclip le seuil monte à `MIN_SIZE + 2`.
const MIN_SIZE: f32 = 3.0;

/// Réglages du pipeline. Défauts = `inference.yml` de PP-OCRv6_tiny_det /
/// PP-OCRv6_tiny_rec et défauts PaddleX pour ce qui n'y figure pas.
#[derive(Debug, Clone, PartialEq)]
pub struct PpOcrOptions {
    /// `DetResizeForTest` : le plus petit côté est ramené à au moins cette valeur
    /// (`limit_type min`), puis H et W sont arrondis au multiple de 32.
    pub limit_side_len: u32,
    /// Plafond du plus grand côté après redimensionnement (`max_side_limit`).
    pub max_side_limit: u32,
    /// Binarisation de la carte de probabilité (`DBPostProcess.thresh`).
    pub det_thresh: f32,
    /// Score minimal d'une boîte : moyenne de la carte dedans (`box_thresh`).
    pub box_thresh: f32,
    /// `unclip_ratio` : `d = aire × ratio / périmètre`, ajouté de chaque côté.
    pub unclip_ratio: f32,
    /// Nombre maximal de composantes examinées (les plus grandes d'abord).
    pub max_candidates: usize,
    /// Taille des lots de reconnaissance (`rec_batch_num`).
    pub rec_batch: usize,
    /// Hauteur d'entrée du reconnaisseur — le graphe est entraîné à 48.
    pub rec_height: u32,
    /// Ratio largeur/hauteur minimal du lot : la largeur d'entrée vaut
    /// `rec_height × max(rec_max_ratio, max(w/h) du lot)` — 320/48 par défaut
    /// (`rec_image_shape 3,48,320`).
    pub rec_max_ratio: f32,
}

impl Default for PpOcrOptions {
    fn default() -> Self {
        Self {
            limit_side_len: 736,
            max_side_limit: 4000,
            det_thresh: 0.2,
            box_thresh: 0.4,
            unclip_ratio: 1.4,
            max_candidates: 3000,
            rec_batch: 6,
            rec_height: 48,
            rec_max_ratio: 320.0 / 48.0,
        }
    }
}

/// Entrée du détecteur, prête pour le graphe : `[1, 3, height, width]` en CHW,
/// canaux B, G, R normalisés.
#[derive(Debug, Clone, PartialEq)]
pub struct DetInput {
    pub data: Vec<f32>,
    pub width: usize,
    pub height: usize,
    /// `width / largeur d'origine`, `height / hauteur d'origine`.
    pub ratio_w: f32,
    pub ratio_h: f32,
}

/// Boîte de texte détectée, en pixels de l'image d'origine (`x1`, `y1` exclus),
/// avec le score DB (moyenne de la carte dans la boîte, avant unclip).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetBox {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
    pub score: f32,
}

impl DetBox {
    fn quad(&self) -> [[f32; 2]; 4] {
        let (x0, y0, x1, y1) = (self.x0 as f32, self.y0 as f32, self.x1 as f32, self.y1 as f32);
        [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    }
}

/// Entrée du reconnaisseur : `[batch, 3, height, width]`, un crop par ligne,
/// paddé de zéros à droite.
#[derive(Debug, Clone, PartialEq)]
pub struct RecInput {
    pub data: Vec<f32>,
    pub batch: usize,
    pub height: usize,
    pub width: usize,
}

/// Sortie du reconnaisseur pour un crop : `steps × classes` probabilités
/// (le graphe se termine par un softmax), ligne par pas de temps.
#[derive(Debug, Clone, PartialEq)]
pub struct RecLogits {
    pub steps: usize,
    pub classes: usize,
    pub data: Vec<f32>,
}

/// PP-OCRv6 tiny sur burn. Voir la doc du module.
pub struct BurnPpOcr {
    det: DetGraph,
    rec: RecGraph,
    dict: Vec<String>,
    device: Device,
    opts: PpOcrOptions,
}

impl BurnPpOcr {
    /// Construit depuis les octets des deux burnpacks et le texte du dictionnaire
    /// (une entrée par ligne, ordre du `inference.yml`).
    pub fn from_bytes(det: &[u8], rec: &[u8], dict: &str, device: BurnDevice) -> Result<Self, OcrError> {
        let dict = parse_dict(dict);
        if dict.is_empty() {
            return Err(OcrError::Model("empty character dictionary".into()));
        }
        let device = device.or_role(crate::burn_device::BurnRole::Ocr).resolve();
        let det = DetGraph::from_bytes(burn::tensor::Bytes::from_bytes_vec(det.to_vec()), &device);
        let rec = RecGraph::from_bytes(burn::tensor::Bytes::from_bytes_vec(rec.to_vec()), &device);
        Ok(Self { det, rec, dict, device, opts: PpOcrOptions::default() })
    }

    /// Lit les trois fichiers puis [`Self::from_bytes`].
    pub fn from_files(
        det_path: impl AsRef<Path>,
        rec_path: impl AsRef<Path>,
        dict_path: impl AsRef<Path>,
        device: BurnDevice,
    ) -> Result<Self, OcrError> {
        let read = |p: &Path| std::fs::read(p).map_err(|e| OcrError::Model(format!("read {}: {e}", p.display())));
        let det = read(det_path.as_ref())?;
        let rec = read(rec_path.as_ref())?;
        let dict = read(dict_path.as_ref())?;
        let dict = String::from_utf8(dict)
            .map_err(|e| OcrError::Model(format!("{}: not utf-8: {e}", dict_path.as_ref().display())))?;
        Self::from_bytes(&det, &rec, &dict, device)
    }

    /// `dir/det.bpk`, `dir/rec.bpk`, `dir/dict.txt`.
    pub fn from_cache_dir(dir: impl AsRef<Path>, device: BurnDevice) -> Result<Self, OcrError> {
        let dir = dir.as_ref();
        Self::from_files(dir.join("det.bpk"), dir.join("rec.bpk"), dir.join("dict.txt"), device)
    }

    /// `$RAG3WEAVER_PPOCR_DIR`, sinon `~/.cache/rag3weaver/ppocrv6-tiny`.
    pub fn default_cache_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("RAG3WEAVER_PPOCR_DIR") {
            return PathBuf::from(dir);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".cache/rag3weaver/ppocrv6-tiny")
    }

    pub fn options(&self) -> &PpOcrOptions {
        &self.opts
    }

    pub fn with_options(mut self, opts: PpOcrOptions) -> Self {
        self.opts = opts;
        self
    }

    pub fn with_limit_side_len(mut self, limit_side_len: u32) -> Self {
        self.opts.limit_side_len = limit_side_len;
        self
    }

    pub fn with_max_side_limit(mut self, max_side_limit: u32) -> Self {
        self.opts.max_side_limit = max_side_limit;
        self
    }

    pub fn with_det_thresh(mut self, det_thresh: f32) -> Self {
        self.opts.det_thresh = det_thresh;
        self
    }

    pub fn with_box_thresh(mut self, box_thresh: f32) -> Self {
        self.opts.box_thresh = box_thresh;
        self
    }

    pub fn with_unclip_ratio(mut self, unclip_ratio: f32) -> Self {
        self.opts.unclip_ratio = unclip_ratio;
        self
    }

    pub fn with_max_candidates(mut self, max_candidates: usize) -> Self {
        self.opts.max_candidates = max_candidates;
        self
    }

    pub fn with_rec_batch(mut self, rec_batch: usize) -> Self {
        self.opts.rec_batch = rec_batch.max(1);
        self
    }

    pub fn with_rec_max_ratio(mut self, rec_max_ratio: f32) -> Self {
        self.opts.rec_max_ratio = rec_max_ratio;
        self
    }

    /// Dictionnaire chargé (sans blank ni espace).
    pub fn dict(&self) -> &[String] {
        &self.dict
    }

    // ── Détection ──────────────────────────────────────────────────────────

    /// Pré-traitement du détecteur (resize + normalisation BGR), sans forward.
    pub fn det_input(&self, image: &OcrImage) -> Result<DetInput, OcrError> {
        det_input(image, &self.opts)
    }

    /// Forward du détecteur : carte de probabilité `height × width`, ligne par ligne.
    pub fn det_forward(&self, input: &DetInput) -> Result<Vec<f32>, OcrError> {
        let x = Tensor::<4>::from_data(
            TensorData::new(input.data.clone(), [1, 3, input.height, input.width]),
            &self.device,
        );
        let y = self.det.forward(x);
        let dims = y.dims();
        if dims != [1, 1, input.height, input.width] {
            return Err(OcrError::Model(format!("det output {dims:?}, expected [1, 1, {}, {}]", input.height, input.width)));
        }
        y.into_data().to_vec::<f32>().map_err(|e| OcrError::Model(format!("det map to_vec: {e:?}")))
    }

    /// Carte de probabilité du détecteur pour une image : `(carte, width, height)`
    /// aux dimensions redimensionnées (multiples de 32).
    pub fn det_map(&self, image: &OcrImage) -> Result<(Vec<f32>, usize, usize), OcrError> {
        let input = self.det_input(image)?;
        let map = self.det_forward(&input)?;
        Ok((map, input.width, input.height))
    }

    /// Détection complète : boîtes en pixels de l'image d'origine, en ordre de lecture.
    pub fn detect(&self, image: &OcrImage) -> Result<Vec<DetBox>, OcrError> {
        let input = self.det_input(image)?;
        let map = self.det_forward(&input)?;
        let map_boxes = boxes_from_map(&map, input.width, input.height, &self.opts);
        let mut lines: Vec<OcrLine> = map_boxes
            .iter()
            .map(|b| {
                let (x0, y0, x1, y1) = to_original(b, &input, image.width, image.height);
                let b = DetBox { x0, y0, x1, y1, score: b.score };
                OcrLine { text: String::new(), confidence: b.score, quad: b.quad() }
            })
            .filter(|l| l.quad[1][0] > l.quad[0][0] && l.quad[3][1] > l.quad[0][1])
            .collect();
        sort_reading_order(&mut lines);
        let boxes = lines
            .iter()
            .map(|l| DetBox {
                x0: l.quad[0][0] as u32,
                y0: l.quad[0][1] as u32,
                x1: l.quad[2][0] as u32,
                y1: l.quad[2][1] as u32,
                score: l.confidence,
            })
            .collect();
        Ok(boxes)
    }

    // ── Reconnaissance ─────────────────────────────────────────────────────

    /// Crop d'une boîte dans l'image d'origine, tourné de 90° si `h/w ≥ 1.5`.
    pub fn crop(image: &OcrImage, b: &DetBox) -> OcrImage {
        crop_rotate(image, b.x0, b.y0, b.x1, b.y1)
    }

    /// Pré-traitement d'un lot de crops (resize hauteur 48, normalisation BGR,
    /// padding), dans l'ordre donné. La largeur est celle du lot entier.
    pub fn rec_input(&self, crops: &[OcrImage]) -> Result<RecInput, OcrError> {
        rec_input(crops, &self.opts)
    }

    /// Forward du reconnaisseur sur un lot pré-traité.
    pub fn rec_forward(&self, input: &RecInput) -> Result<Vec<RecLogits>, OcrError> {
        if input.batch == 0 {
            return Ok(Vec::new());
        }
        let x = Tensor::<4>::from_data(
            TensorData::new(input.data.clone(), [input.batch, 3, input.height, input.width]),
            &self.device,
        );
        let y = self.rec.forward(x);
        let [batch, steps, classes] = y.dims();
        if batch != input.batch {
            return Err(OcrError::Model(format!("rec output batch {batch}, expected {}", input.batch)));
        }
        if classes != self.dict.len() + 2 {
            return Err(OcrError::Model(format!(
                "rec output has {classes} classes, dictionary has {} entries (+ blank + space)",
                self.dict.len()
            )));
        }
        let all = y.into_data().to_vec::<f32>().map_err(|e| OcrError::Model(format!("rec probs to_vec: {e:?}")))?;
        Ok(all
            .chunks_exact(steps * classes)
            .map(|c| RecLogits { steps, classes, data: c.to_vec() })
            .collect())
    }

    /// Probabilités CTC de chaque crop, tous dans un seul lot, dans l'ordre donné.
    pub fn rec_logits(&self, crops: &[OcrImage]) -> Result<Vec<RecLogits>, OcrError> {
        let input = self.rec_input(crops)?;
        self.rec_forward(&input)
    }

    /// Décodage CTC glouton : `(texte, confiance)`, `None` si aucun caractère.
    pub fn decode_ctc(&self, logits: &RecLogits) -> Option<(String, f32)> {
        ctc_decode(&logits.data, logits.steps, logits.classes, &self.dict)
    }

    /// Reconnaissance des crops par lots de `rec_batch`, triés par ratio w/h
    /// comme PaddleOCR ; résultat dans l'ordre des crops.
    fn recognize_crops(&self, crops: &[OcrImage]) -> Result<Vec<Option<(String, f32)>>, OcrError> {
        let mut order: Vec<usize> = (0..crops.len()).collect();
        order.sort_by(|&a, &b| {
            let ra = crops[a].width as f32 / crops[a].height.max(1) as f32;
            let rb = crops[b].width as f32 / crops[b].height.max(1) as f32;
            ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut results: Vec<Option<(String, f32)>> = vec![None; crops.len()];
        for chunk in order.chunks(self.opts.rec_batch.max(1)) {
            let batch: Vec<OcrImage> = chunk.iter().map(|&i| crops[i].clone()).collect();
            let logits = self.rec_logits(&batch)?;
            for (&i, l) in chunk.iter().zip(&logits) {
                results[i] = self.decode_ctc(l);
            }
        }
        Ok(results)
    }
}

impl Ocr for BurnPpOcr {
    fn recognize(&self, image: &OcrImage) -> Result<OcrOutput, OcrError> {
        if image.width == 0 || image.height == 0 {
            return Err(OcrError::Decode("empty image".into()));
        }
        if image.rgb.len() != image.width as usize * image.height as usize * 3 {
            return Err(OcrError::Decode("rgb buffer size does not match dimensions".into()));
        }
        let boxes = self.detect(image)?;
        let crops: Vec<OcrImage> = boxes.iter().map(|b| Self::crop(image, b)).collect();
        let texts = self.recognize_crops(&crops)?;
        let mut lines: Vec<OcrLine> = boxes
            .iter()
            .zip(texts)
            .filter_map(|(b, t)| t.map(|(text, confidence)| OcrLine { text, confidence, quad: b.quad() }))
            .collect();
        sort_reading_order(&mut lines);
        Ok(OcrOutput { width: image.width, height: image.height, lines })
    }

    fn name(&self) -> &str {
        MODEL_NAME
    }
}

// ── Fonctions pures (testables sans modèle) ───────────────────────────────

/// Une entrée par ligne ; seul le `\r` de fin est retiré (un caractère peut
/// être n'importe quoi, y compris un blanc). La ligne vide finale du fichier
/// n'est pas une entrée.
fn parse_dict(text: &str) -> Vec<String> {
    let mut entries: Vec<String> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l).to_string()).collect();
    if entries.last().is_some_and(|l| l.is_empty()) {
        entries.pop();
    }
    entries
}

/// Dimensions cibles du détecteur (`resize_image_type0`, `limit_type min`,
/// plafond `max_side_limit`), arrondies au multiple de 32, au moins 32.
pub fn det_resize_dims(width: u32, height: u32, limit_side_len: u32, max_side_limit: u32) -> (u32, u32) {
    let (w, h) = (width as f32, height as f32);
    let mut ratio = if w.min(h) < limit_side_len as f32 { limit_side_len as f32 / w.min(h) } else { 1.0 };
    if w.max(h) * ratio > max_side_limit as f32 {
        ratio = max_side_limit as f32 / w.max(h);
    }
    let round32 = |v: f32| (((v as u32) as f32 / 32.0).round() as u32 * 32).max(32);
    (round32(w * ratio), round32(h * ratio))
}

fn to_rgb_image(image: &OcrImage) -> Result<RgbImage, OcrError> {
    RgbImage::from_raw(image.width, image.height, image.rgb.clone())
        .ok_or_else(|| OcrError::Decode("rgb buffer size does not match dimensions".into()))
}

/// Resize (filtre triangle = bilinéaire) vers `(w, h)`, sans copie si déjà à la taille.
fn resize_rgb(image: &OcrImage, w: u32, h: u32) -> Result<RgbImage, OcrError> {
    let src = to_rgb_image(image)?;
    if (w, h) == (image.width, image.height) {
        return Ok(src);
    }
    Ok(imageops::resize(&src, w, h, FilterType::Triangle))
}

/// Normalisation d'un pixel RGB vers les trois canaux **B, G, R** :
/// `((x/255) - mean[c]) / std[c]`, c étant l'indice BGR.
#[inline]
fn normalize_bgr(rgb: [u8; 3], mean: &[f32; 3], std: &[f32; 3]) -> [f32; 3] {
    let bgr = [rgb[2], rgb[1], rgb[0]];
    let mut out = [0.0; 3];
    for c in 0..3 {
        out[c] = (bgr[c] as f32 / 255.0 - mean[c]) / std[c];
    }
    out
}

/// Écrit `img` (RGB) en CHW BGR normalisé dans `dst`, à l'offset `base`,
/// avec un stride de plan `plane` et une largeur de ligne `row_w`.
fn write_chw(dst: &mut [f32], base: usize, plane: usize, row_w: usize, img: &RgbImage, mean: &[f32; 3], std: &[f32; 3]) {
    for (x, y, p) in img.enumerate_pixels() {
        let v = normalize_bgr(p.0, mean, std);
        let i = base + y as usize * row_w + x as usize;
        dst[i] = v[0];
        dst[plane + i] = v[1];
        dst[2 * plane + i] = v[2];
    }
}

fn det_input(image: &OcrImage, opts: &PpOcrOptions) -> Result<DetInput, OcrError> {
    let (w, h) = det_resize_dims(image.width, image.height, opts.limit_side_len, opts.max_side_limit);
    let resized = resize_rgb(image, w, h)?;
    let (width, height) = (w as usize, h as usize);
    let plane = width * height;
    let mut data = vec![0.0f32; 3 * plane];
    write_chw(&mut data, 0, plane, width, &resized, &DET_MEAN, &DET_STD);
    Ok(DetInput {
        data,
        width,
        height,
        ratio_w: w as f32 / image.width as f32,
        ratio_h: h as f32 / image.height as f32,
    })
}

/// Boîte englobante d'une composante connexe de la carte binarisée, en
/// coordonnées de la carte (bornes incluses, comme les points d'un contour).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
    pub pixels: usize,
}

/// Composantes connexes (8-voisinage) de `mask` (`width × height`, ligne par
/// ligne), par BFS sur une file explicite. Ordre : rangée puis colonne du
/// premier pixel rencontré.
pub fn connected_components(mask: &[bool], width: usize, height: usize) -> Vec<Component> {
    let mut seen = vec![false; mask.len()];
    let mut out = Vec::new();
    let mut queue: Vec<usize> = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        queue.clear();
        queue.push(start);
        let (mut x0, mut y0, mut x1, mut y1, mut pixels) = (usize::MAX, usize::MAX, 0, 0, 0);
        while let Some(i) = queue.pop() {
            let (x, y) = (i % width, i / width);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
            pixels += 1;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                        continue;
                    }
                    let j = ny as usize * width + nx as usize;
                    if mask[j] && !seen[j] {
                        seen[j] = true;
                        queue.push(j);
                    }
                }
            }
        }
        out.push(Component { x0, y0, x1, y1, pixels });
    }
    out
}

/// Boîte candidate en coordonnées de la carte, bornes flottantes (`x1`, `y1`
/// inclus au sens des points de contour : largeur = `x1 - x0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub score: f32,
}

/// Unclip d'un rectangle : `d = aire × ratio / périmètre`, ajouté de chaque côté.
pub fn unclip_rect(x0: f32, y0: f32, x1: f32, y1: f32, ratio: f32) -> (f32, f32, f32, f32) {
    let (w, h) = (x1 - x0, y1 - y0);
    let perimeter = 2.0 * (w + h);
    if perimeter <= 0.0 {
        return (x0, y0, x1, y1);
    }
    let d = w * h * ratio / perimeter;
    (x0 - d, y0 - d, x1 + d, y1 + d)
}

/// Moyenne de la carte sur les pixels `[x0, x1] × [y0, y1]` (bornes arrondies
/// vers l'extérieur et bornées à la carte — `box_score_fast`).
pub fn box_score(map: &[f32], width: usize, height: usize, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let clampx = |v: f32| (v.max(0.0) as usize).min(width.saturating_sub(1));
    let clampy = |v: f32| (v.max(0.0) as usize).min(height.saturating_sub(1));
    let (xa, xb) = (clampx(x0.floor()), clampx(x1.ceil()));
    let (ya, yb) = (clampy(y0.floor()), clampy(y1.ceil()));
    let mut sum = 0.0f64;
    let mut n = 0usize;
    for y in ya..=yb {
        for x in xa..=xb {
            sum += map[y * width + x] as f64;
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        (sum / n as f64) as f32
    }
}

/// Post-traitement DB sur la carte de probabilité : boîtes axées, en
/// coordonnées de la carte, non triées.
pub fn boxes_from_map(map: &[f32], width: usize, height: usize, opts: &PpOcrOptions) -> Vec<MapBox> {
    let mask: Vec<bool> = map.iter().map(|&p| p > opts.det_thresh).collect();
    let mut comps = connected_components(&mask, width, height);
    comps.sort_by(|a, b| b.pixels.cmp(&a.pixels));
    comps.truncate(opts.max_candidates);

    let mut boxes = Vec::new();
    for c in comps {
        let (x0, y0, x1, y1) = (c.x0 as f32, c.y0 as f32, c.x1 as f32, c.y1 as f32);
        if (x1 - x0).min(y1 - y0) < MIN_SIZE {
            continue;
        }
        let score = box_score(map, width, height, x0, y0, x1, y1);
        if score < opts.box_thresh {
            continue;
        }
        let (ux0, uy0, ux1, uy1) = unclip_rect(x0, y0, x1, y1, opts.unclip_ratio);
        if (ux1 - ux0).min(uy1 - uy0) < MIN_SIZE + 2.0 {
            continue;
        }
        boxes.push(MapBox { x0: ux0, y0: uy0, x1: ux1, y1: uy1, score });
    }
    boxes
}

/// Coordonnées de la carte → pixels de l'image d'origine (arrondis, bornés à
/// `[0, W] × [0, H]`).
fn to_original(b: &MapBox, input: &DetInput, orig_w: u32, orig_h: u32) -> (u32, u32, u32, u32) {
    let sx = |v: f32| (v / input.ratio_w).round().clamp(0.0, orig_w as f32) as u32;
    let sy = |v: f32| (v / input.ratio_h).round().clamp(0.0, orig_h as f32) as u32;
    (sx(b.x0), sy(b.y0), sx(b.x1), sy(b.y1))
}

/// Rotation de 90° dans le sens trigonométrique (`np.rot90`) : `(w, h)` → `(h, w)`.
pub fn rotate90_ccw(image: &OcrImage) -> OcrImage {
    let (w, h) = (image.width as usize, image.height as usize);
    let mut rgb = vec![0u8; image.rgb.len()];
    // new[i][j] = old[j][w - 1 - i], new: h' = w lignes, w' = h colonnes
    for i in 0..w {
        for j in 0..h {
            let src = (j * w + (w - 1 - i)) * 3;
            let dst = (i * h + j) * 3;
            rgb[dst..dst + 3].copy_from_slice(&image.rgb[src..src + 3]);
        }
    }
    OcrImage { width: image.height, height: image.width, rgb }
}

/// Crop `[x0, x1) × [y0, y1)` (borné à l'image), puis rotation si `h/w ≥ 1.5`.
pub fn crop_rotate(image: &OcrImage, x0: u32, y0: u32, x1: u32, y1: u32) -> OcrImage {
    let x1 = x1.min(image.width);
    let y1 = y1.min(image.height);
    let x0 = x0.min(x1);
    let y0 = y0.min(y1);
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let stride = image.width as usize * 3;
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in y0 as usize..y1 as usize {
        let row = y * stride + x0 as usize * 3;
        rgb.extend_from_slice(&image.rgb[row..row + w * 3]);
    }
    let crop = OcrImage { width: w as u32, height: h as u32, rgb };
    if w > 0 && h as f32 / w as f32 >= 1.5 {
        rotate90_ccw(&crop)
    } else {
        crop
    }
}

/// Largeur d'entrée du lot : `rec_height × max(rec_max_ratio, max w/h)`, tronquée.
pub fn rec_batch_width(crops: &[OcrImage], opts: &PpOcrOptions) -> u32 {
    let mut ratio = opts.rec_max_ratio;
    for c in crops {
        if c.height > 0 {
            ratio = ratio.max(c.width as f32 / c.height as f32);
        }
    }
    ((opts.rec_height as f32 * ratio) as u32).max(1)
}

/// Largeur redimensionnée d'un crop : `ceil(rec_height × w/h)`, plafonnée à `img_w`.
pub fn rec_resized_width(crop_w: u32, crop_h: u32, rec_height: u32, img_w: u32) -> u32 {
    if crop_h == 0 || crop_w == 0 {
        return 1;
    }
    let ratio = crop_w as f32 / crop_h as f32;
    let w = (rec_height as f32 * ratio).ceil() as u32;
    w.clamp(1, img_w)
}

fn rec_input(crops: &[OcrImage], opts: &PpOcrOptions) -> Result<RecInput, OcrError> {
    let height = opts.rec_height as usize;
    let width = rec_batch_width(crops, opts) as usize;
    let plane = width * height;
    let mut data = vec![0.0f32; crops.len() * 3 * plane];
    for (b, crop) in crops.iter().enumerate() {
        if crop.width == 0 || crop.height == 0 {
            continue;
        }
        let rw = rec_resized_width(crop.width, crop.height, opts.rec_height, width as u32);
        let resized = resize_rgb(crop, rw, opts.rec_height)?;
        write_chw(&mut data, b * 3 * plane, plane, width, &resized, &[0.5; 3], &[0.5; 3]);
    }
    Ok(RecInput { data, batch: crops.len(), height, width })
}

/// Décodage CTC glouton de `steps × classes` probabilités : argmax par pas,
/// répétitions consécutives fusionnées, blank (0) retiré ; index `1..=N` →
/// `dict[index - 1]`, `N + 1` → espace. Confiance = moyenne des probabilités
/// des caractères gardés. `None` si aucun caractère.
pub fn ctc_decode(probs: &[f32], steps: usize, classes: usize, dict: &[String]) -> Option<(String, f32)> {
    let mut text = String::new();
    let mut sum = 0.0f32;
    let mut n = 0usize;
    let mut prev = usize::MAX;
    for t in 0..steps {
        let row = &probs[t * classes..(t + 1) * classes];
        let (idx, p) = row.iter().enumerate().fold((0usize, f32::NEG_INFINITY), |acc, (i, &v)| {
            if v > acc.1 {
                (i, v)
            } else {
                acc
            }
        });
        let repeated = idx == prev;
        prev = idx;
        if idx == 0 || repeated {
            continue;
        }
        if idx == classes - 1 {
            text.push(' ');
        } else if let Some(ch) = dict.get(idx - 1) {
            text.push_str(ch);
        } else {
            continue;
        }
        sum += p;
        n += 1;
    }
    if n == 0 {
        None
    } else {
        Some((text, sum / n as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(chars: &str) -> Vec<String> {
        chars.chars().map(|c| c.to_string()).collect()
    }

    #[test]
    fn dict_parsing_keeps_order_and_drops_trailing_newline() {
        let d = parse_dict("a\nb\r\n \nc\n");
        assert_eq!(d, ["a", "b", " ", "c"]);
        assert_eq!(parse_dict("").len(), 0);
    }

    #[test]
    fn det_resize_is_multiple_of_32_with_min_limit() {
        // 400×120, limite min 736 : ratio 6.133 → 2453×736 → 2464×736
        assert_eq!(det_resize_dims(400, 120, 736, 4000), (2464, 736));
        // déjà assez grand : inchangé sauf arrondi à 32
        assert_eq!(det_resize_dims(1000, 800, 736, 4000), (992, 800));
        // plafond : 100×20 → ratio 36.8 → 3680 > 640 → ratio 6.4 → 640×128
        assert_eq!(det_resize_dims(100, 20, 736, 640), (640, 128));
        // jamais sous 32
        assert_eq!(det_resize_dims(1, 1, 8, 4000), (32, 32));
    }

    #[test]
    fn bgr_normalisation_of_a_known_pixel() {
        let v = normalize_bgr([255, 0, 128], &DET_MEAN, &DET_STD);
        // canal 0 = B = 128 avec la moyenne 0.485
        assert!((v[0] - (128.0 / 255.0 - 0.485) / 0.229).abs() < 1e-6);
        assert!((v[1] - (0.0 - 0.456) / 0.224).abs() < 1e-6);
        assert!((v[2] - (1.0 - 0.406) / 0.225).abs() < 1e-6);
        // rec : (x/255 - 0.5) / 0.5
        let r = normalize_bgr([255, 0, 128], &[0.5; 3], &[0.5; 3]);
        assert!((r[0] - (128.0 / 255.0 - 0.5) / 0.5).abs() < 1e-6);
        assert_eq!(r[1], -1.0);
        assert_eq!(r[2], 1.0);
    }

    #[test]
    fn det_input_layout_is_chw_bgr() {
        // 32×32 uni (pas de resize) : chaque plan est constant
        let img = OcrImage::from_rgb(32, 32, [10u8, 20, 30].repeat(32 * 32)).unwrap();
        let input = det_input(&img, &PpOcrOptions { limit_side_len: 32, ..Default::default() }).unwrap();
        assert_eq!((input.width, input.height), (32, 32));
        assert_eq!(input.data.len(), 3 * 32 * 32);
        let expect = normalize_bgr([10, 20, 30], &DET_MEAN, &DET_STD);
        assert!(input.data[..1024].iter().all(|&v| (v - expect[0]).abs() < 1e-6));
        assert!(input.data[1024..2048].iter().all(|&v| (v - expect[1]).abs() < 1e-6));
        assert!(input.data[2048..].iter().all(|&v| (v - expect[2]).abs() < 1e-6));
        assert_eq!((input.ratio_w, input.ratio_h), (1.0, 1.0));
    }

    #[test]
    fn components_and_boxes_on_a_synthetic_map() {
        // 16×8 : un pixel isolé en haut à droite, un bloc 6×4 à gauche (fort),
        // un bloc 4×2 en bas à droite (faible)
        let (w, h) = (16, 8);
        let mut map = vec![0.0f32; w * h];
        map[15] = 0.8;
        for y in 2..6 {
            for x in 1..7 {
                map[y * w + x] = 0.9;
            }
        }
        for y in 6..8 {
            for x in 10..14 {
                map[y * w + x] = 0.3;
            }
        }
        let mask: Vec<bool> = map.iter().map(|&p| p > 0.2).collect();
        let comps = connected_components(&mask, w, h);
        assert_eq!(comps.len(), 3);
        assert_eq!(comps[0], Component { x0: 15, y0: 0, x1: 15, y1: 0, pixels: 1 });
        assert_eq!(comps[1], Component { x0: 1, y0: 2, x1: 6, y1: 5, pixels: 24 });
        assert_eq!(comps[2], Component { x0: 10, y0: 6, x1: 13, y1: 7, pixels: 8 });

        // 8-voisinage : deux pixels en diagonale forment une seule composante
        let diag = [true, false, false, true];
        assert_eq!(connected_components(&diag, 2, 2).len(), 1);

        let opts = PpOcrOptions { det_thresh: 0.2, box_thresh: 0.4, unclip_ratio: 1.4, ..Default::default() };
        let boxes = boxes_from_map(&map, w, h, &opts);
        // le bloc faible (score 0.3 < 0.4) et le pixel isolé (côté < 3) sont rejetés
        assert_eq!(boxes.len(), 1);
        let b = boxes[0];
        assert!((b.score - 0.9).abs() < 1e-6);
        // contour 1..6 × 2..5 : w=5, h=3, d = 15·1.4/16 = 1.3125
        assert!((b.x0 - (1.0 - 1.3125)).abs() < 1e-5 && (b.y0 - (2.0 - 1.3125)).abs() < 1e-5);
        assert!((b.x1 - (6.0 + 1.3125)).abs() < 1e-5 && (b.y1 - (5.0 + 1.3125)).abs() < 1e-5);
        // un bloc trop fin (2 lignes) est rejeté avant le score, comme PaddleOCR (min_size 3)
        let mut thin = vec![0.0f32; w * h];
        for y in 0..2 {
            for x in 0..10 {
                thin[y * w + x] = 0.9;
            }
        }
        assert!(boxes_from_map(&thin, w, h, &opts).is_empty());
    }

    #[test]
    fn unclip_adds_area_over_perimeter() {
        let (x0, y0, x1, y1) = unclip_rect(10.0, 10.0, 30.0, 20.0, 1.5);
        // aire 200, périmètre 60, d = 5
        assert_eq!((x0, y0, x1, y1), (5.0, 5.0, 35.0, 25.0));
        assert_eq!(unclip_rect(3.0, 3.0, 3.0, 3.0, 1.5), (3.0, 3.0, 3.0, 3.0));
    }

    #[test]
    fn box_score_is_the_mean_inside_bounds() {
        let map = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        assert!((box_score(&map, 4, 2, 1.0, 0.0, 2.0, 1.0) - 1.0).abs() < 1e-6);
        assert!((box_score(&map, 4, 2, 0.0, 0.0, 3.0, 1.0) - 0.5).abs() < 1e-6);
        // hors carte : borné
        assert!((box_score(&map, 4, 2, -5.0, -5.0, 50.0, 50.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ctc_decode_blank_repeats_and_space() {
        let d = dict("ab");
        // classes = 4 : blank, a, b, espace
        let rows: [[f32; 4]; 7] = [
            [0.1, 0.8, 0.1, 0.0], // a
            [0.1, 0.7, 0.2, 0.0], // a (répétition → fusionnée)
            [0.9, 0.1, 0.0, 0.0], // blank
            [0.1, 0.6, 0.3, 0.0], // a (après blank → nouveau)
            [0.0, 0.0, 0.0, 1.0], // espace
            [0.0, 0.1, 0.9, 0.0], // b
            [0.5, 0.1, 0.4, 0.0], // blank
        ];
        let probs: Vec<f32> = rows.iter().flatten().copied().collect();
        let (text, conf) = ctc_decode(&probs, 7, 4, &d).unwrap();
        assert_eq!(text, "aa b");
        assert!((conf - (0.8 + 0.6 + 1.0 + 0.9) / 4.0).abs() < 1e-6);
        // que du blank : rien
        let blanks = [1.0f32, 0.0, 0.0, 0.0].repeat(3);
        assert!(ctc_decode(&blanks, 3, 4, &d).is_none());
    }

    #[test]
    fn crop_and_rotate() {
        // 4×2, pixels numérotés
        let rgb: Vec<u8> = (0..8u8).flat_map(|i| [i, i, i]).collect();
        let img = OcrImage::from_rgb(4, 2, rgb).unwrap();
        let c = crop_rotate(&img, 1, 0, 3, 2);
        assert_eq!((c.width, c.height), (2, 2));
        assert_eq!(c.rgb.iter().step_by(3).copied().collect::<Vec<_>>(), [1, 2, 5, 6]);
        // hors bornes : borné, pas de panique
        let c = crop_rotate(&img, 3, 1, 10, 10);
        assert_eq!((c.width, c.height), (1, 1));
        assert_eq!(c.rgb, [7, 7, 7]);

        // portrait (h/w = 2 ≥ 1.5) → tourné : np.rot90 d'une colonne [[a],[b],[c],[d]] = [[a, b, c, d]]
        // en rot90 CCW, la première colonne devient la dernière ligne... vérifions sur 2×3
        let rgb: Vec<u8> = (0..6u8).flat_map(|i| [i, i, i]).collect();
        let tall = OcrImage::from_rgb(2, 3, rgb).unwrap(); // lignes [0 1] [2 3] [4 5]
        let r = crop_rotate(&tall, 0, 0, 2, 3);
        assert_eq!((r.width, r.height), (3, 2));
        // np.rot90 : new[i][j] = old[j][w-1-i] → new = [[1, 3, 5], [0, 2, 4]]
        assert_eq!(r.rgb.iter().step_by(3).copied().collect::<Vec<_>>(), [1, 3, 5, 0, 2, 4]);
        // paysage : pas tourné
        let wide = crop_rotate(&img, 0, 0, 4, 2);
        assert_eq!((wide.width, wide.height), (4, 2));
    }

    #[test]
    fn rec_input_dimensions_and_padding() {
        let opts = PpOcrOptions::default();
        // 100×20 (ratio 5) et 400×40 (ratio 10) : largeur de lot = 48 × 10 = 480
        let a = OcrImage::from_rgb(100, 20, vec![255; 100 * 20 * 3]).unwrap();
        let b = OcrImage::from_rgb(400, 40, vec![0; 400 * 40 * 3]).unwrap();
        assert_eq!(rec_batch_width(&[a.clone()], &opts), 320);
        assert_eq!(rec_batch_width(&[a.clone(), b.clone()], &opts), 480);
        assert_eq!(rec_resized_width(100, 20, 48, 480), 240);
        assert_eq!(rec_resized_width(400, 40, 48, 480), 480);
        assert_eq!(rec_resized_width(4000, 40, 48, 480), 480);

        let input = rec_input(&[a, b], &opts).unwrap();
        assert_eq!((input.batch, input.height, input.width), (2, 48, 480));
        assert_eq!(input.data.len(), 2 * 3 * 48 * 480);
        // crop blanc : 1.0 sur ses 240 colonnes, 0 (padding) au-delà
        let plane = 48 * 480;
        assert_eq!(input.data[0], 1.0);
        assert_eq!(input.data[239], 1.0);
        assert_eq!(input.data[240], 0.0);
        assert_eq!(input.data[2 * plane + 47 * 480 + 239], 1.0);
        // crop noir : -1.0 partout (pas de padding)
        let second = 3 * plane;
        assert_eq!(input.data[second], -1.0);
        assert_eq!(input.data[second + 479], -1.0);
        assert_eq!(input.data[second + 3 * plane - 1], -1.0);

        // lot vide
        let empty = rec_input(&[], &opts).unwrap();
        assert_eq!((empty.batch, empty.width), (0, 320));
    }

    #[test]
    fn options_defaults_match_inference_yml() {
        let o = PpOcrOptions::default();
        assert_eq!((o.limit_side_len, o.max_side_limit), (736, 4000));
        assert_eq!((o.det_thresh, o.box_thresh, o.unclip_ratio), (0.2, 0.4, 1.4));
        assert_eq!((o.max_candidates, o.rec_batch, o.rec_height), (3000, 6, 48));
        assert!((o.rec_max_ratio - 320.0 / 48.0).abs() < 1e-6);
    }

    #[test]
    fn default_cache_dir_honours_env() {
        let dir = BurnPpOcr::default_cache_dir();
        assert!(dir.ends_with("ppocrv6-tiny") || std::env::var("RAG3WEAVER_PPOCR_DIR").is_ok());
    }
}
