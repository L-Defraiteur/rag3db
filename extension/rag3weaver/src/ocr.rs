//! OCR en usage unitaire : une image entre, des lignes de texte (avec boîte
//! et confiance) sortent. Chantier 4 de l'ordre de Lucie (doc 41).
//!
//! Même doctrine que les embedders et le reranker : le trait [`Ocr`] est la
//! seule surface que le dataflow voit (service `"ocr"`, nœud
//! [`crate::dataflow::OcrNode`]) ; l'implémentation produit est un
//! détecteur + reconnaisseur PP-OCR sur burn (feature `burn-ocr`), le mock
//! sert aux tests du nœud. Aucune bibliothèque lourde : le décodage
//! PNG/JPEG passe par `image` (feature `ocr`), déjà dans l'arbre via burn.

use std::fmt;
use std::sync::Arc;

/// Erreurs d'OCR. Seul le message est contractuel.
#[derive(Debug, Clone, PartialEq)]
pub enum OcrError {
    /// L'image n'a pas pu être décodée (format inconnu, octets corrompus,
    /// ou décodage non compilé — feature `ocr`).
    Decode(String),
    /// Le modèle a échoué (chargement, inférence, post-traitement).
    Model(String),
}

impl fmt::Display for OcrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OcrError::Decode(m) => write!(f, "image decode error: {m}"),
            OcrError::Model(m) => write!(f, "ocr model error: {m}"),
        }
    }
}

impl std::error::Error for OcrError {}

/// Image décodée, RGB 8 bits entrelacé, ligne par ligne, sans padding.
/// C'est l'entrée de tous les [`Ocr`] : le décodage est fait une fois, en
/// amont, et ne dépend pas du modèle.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrImage {
    pub width: u32,
    pub height: u32,
    /// `width * height * 3` octets.
    pub rgb: Vec<u8>,
}

impl OcrImage {
    /// Construit depuis un tampon RGB déjà décodé. Vérifie la taille.
    pub fn from_rgb(width: u32, height: u32, rgb: Vec<u8>) -> Result<Self, OcrError> {
        let expected = width as usize * height as usize * 3;
        if rgb.len() != expected {
            return Err(OcrError::Decode(format!(
                "rgb buffer has {} bytes, expected {expected} for {width}x{height}",
                rgb.len()
            )));
        }
        Ok(Self { width, height, rgb })
    }

    /// Décode PNG / JPEG / WebP / GIF / BMP / TIFF (ce que `image` sait lire
    /// avec les décodeurs compilés) vers du RGB 8 bits.
    #[cfg(feature = "ocr")]
    pub fn decode(bytes: &[u8]) -> Result<Self, OcrError> {
        let img = image::load_from_memory(bytes).map_err(|e| OcrError::Decode(e.to_string()))?;
        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();
        Ok(Self { width, height, rgb: rgb.into_raw() })
    }

    /// Sans la feature `ocr`, aucun décodeur n'est compilé : le nœud accepte
    /// alors seulement des [`OcrImage`] déjà décodées.
    #[cfg(not(feature = "ocr"))]
    pub fn decode(_bytes: &[u8]) -> Result<Self, OcrError> {
        Err(OcrError::Decode(
            "image decoding not compiled (enable feature `ocr`)".into(),
        ))
    }
}

/// Une ligne reconnue : texte, confiance `[0, 1]`, quadrilatère en pixels
/// (ordre : haut-gauche, haut-droit, bas-droit, bas-gauche).
#[derive(Debug, Clone, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub confidence: f32,
    pub quad: [[f32; 2]; 4],
}

impl OcrLine {
    /// Ligne axée sur un rectangle `(x, y, w, h)` — pratique pour les mocks
    /// et les modèles sans rotation.
    pub fn rect(text: impl Into<String>, confidence: f32, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            text: text.into(),
            confidence,
            quad: [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
        }
    }

    /// Centre vertical et bord gauche, pour l'ordre de lecture.
    fn anchor(&self) -> (f32, f32) {
        let cy = self.quad.iter().map(|p| p[1]).sum::<f32>() / 4.0;
        let left = self.quad.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        (cy, left)
    }

    fn height(&self) -> f32 {
        let top = self.quad.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let bottom = self.quad.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
        (bottom - top).max(1.0)
    }
}

/// Résultat d'une passe d'OCR sur une image.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OcrOutput {
    pub width: u32,
    pub height: u32,
    /// Lignes **dans l'ordre de lecture** (voir [`sort_reading_order`]).
    pub lines: Vec<OcrLine>,
}

impl OcrOutput {
    /// Texte brut : une ligne par ligne reconnue, `\n` entre elles, sans
    /// terminateur — même convention que `_content` des chunks.
    pub fn text(&self) -> String {
        self.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n")
    }

    /// Confiance moyenne, `0.0` sans ligne.
    pub fn mean_confidence(&self) -> f32 {
        if self.lines.is_empty() {
            return 0.0;
        }
        self.lines.iter().map(|l| l.confidence).sum::<f32>() / self.lines.len() as f32
    }
}

/// Trie les lignes en ordre de lecture : de haut en bas, puis de gauche à
/// droite pour les lignes dont les centres verticaux sont à moins d'une
/// demi-hauteur l'un de l'autre (même « rangée »). Partagé par toutes les
/// implémentations pour que le texte sorte pareil quel que soit le modèle.
pub fn sort_reading_order(lines: &mut [OcrLine]) {
    lines.sort_by(|a, b| {
        let (ay, ax) = a.anchor();
        let (by, bx) = b.anchor();
        let tol = a.height().min(b.height()) * 0.5;
        if (ay - by).abs() <= tol {
            ax.partial_cmp(&bx).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
}

/// Reconnaisseur de texte. Une image décodée entre, des lignes sortent —
/// **déjà** en ordre de lecture (l'implémentation appelle
/// [`sort_reading_order`]).
pub trait Ocr: Send + Sync {
    fn recognize(&self, image: &OcrImage) -> Result<OcrOutput, OcrError>;

    /// Nom lisible (modèle), pour les diagnostics.
    fn name(&self) -> &str {
        "ocr"
    }
}

impl<T: Ocr + ?Sized> Ocr for Arc<T> {
    fn recognize(&self, image: &OcrImage) -> Result<OcrOutput, OcrError> {
        (**self).recognize(image)
    }
    fn name(&self) -> &str {
        (**self).name()
    }
}

/// OCR de test : rend des lignes fixées d'avance, quelle que soit l'image
/// (mais vérifie qu'elle est bien formée). Permet de tester le nœud, le
/// tri et le texte sans modèle.
#[derive(Debug, Clone, Default)]
pub struct MockOcr {
    pub lines: Vec<OcrLine>,
}

impl MockOcr {
    pub fn with_lines(lines: Vec<OcrLine>) -> Self {
        Self { lines }
    }
}

impl Ocr for MockOcr {
    fn recognize(&self, image: &OcrImage) -> Result<OcrOutput, OcrError> {
        if image.width == 0 || image.height == 0 {
            return Err(OcrError::Decode("empty image".into()));
        }
        let mut lines = self.lines.clone();
        sort_reading_order(&mut lines);
        Ok(OcrOutput { width: image.width, height: image.height, lines })
    }
    fn name(&self) -> &str {
        "mock-ocr"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(w: u32, h: u32) -> OcrImage {
        OcrImage::from_rgb(w, h, vec![255; (w * h * 3) as usize]).unwrap()
    }

    #[test]
    fn from_rgb_checks_size() {
        assert!(OcrImage::from_rgb(2, 2, vec![0; 12]).is_ok());
        let err = OcrImage::from_rgb(2, 2, vec![0; 11]).unwrap_err();
        assert!(matches!(err, OcrError::Decode(_)), "{err}");
    }

    #[test]
    fn reading_order_rows_then_columns() {
        // Deux rangées ; dans la première, la ligne de droite est légèrement
        // plus haute que celle de gauche (bruit de détection) : elle doit
        // quand même venir après.
        let mut lines = vec![
            OcrLine::rect("second row", 0.9, 10.0, 60.0, 100.0, 20.0),
            OcrLine::rect("right", 0.9, 150.0, 8.0, 60.0, 20.0),
            OcrLine::rect("left", 0.9, 10.0, 12.0, 100.0, 20.0),
        ];
        sort_reading_order(&mut lines);
        let texts: Vec<_> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["left", "right", "second row"]);
    }

    #[test]
    fn text_joins_lines_without_terminator() {
        let out = OcrOutput {
            width: 1,
            height: 1,
            lines: vec![OcrLine::rect("a", 1.0, 0.0, 0.0, 1.0, 1.0), OcrLine::rect("b", 0.5, 0.0, 2.0, 1.0, 1.0)],
        };
        assert_eq!(out.text(), "a\nb");
        assert!((out.mean_confidence() - 0.75).abs() < 1e-6);
        assert_eq!(OcrOutput::default().text(), "");
        assert_eq!(OcrOutput::default().mean_confidence(), 0.0);
    }

    #[test]
    fn mock_sorts_and_rejects_empty_image() {
        let ocr = MockOcr::with_lines(vec![
            OcrLine::rect("b", 1.0, 0.0, 30.0, 10.0, 10.0),
            OcrLine::rect("a", 1.0, 0.0, 0.0, 10.0, 10.0),
        ]);
        let out = ocr.recognize(&blank(4, 4)).unwrap();
        assert_eq!(out.text(), "a\nb");
        assert_eq!((out.width, out.height), (4, 4));
        let empty = OcrImage { width: 0, height: 0, rgb: vec![] };
        assert!(ocr.recognize(&empty).is_err());
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn decode_png_roundtrip() {
        let mut png = Vec::new();
        let img = image::RgbImage::from_fn(3, 2, |x, y| image::Rgb([x as u8, y as u8, 7]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let decoded = OcrImage::decode(&png).unwrap();
        assert_eq!((decoded.width, decoded.height), (3, 2));
        assert_eq!(&decoded.rgb[3..6], &[1, 0, 7]);
        assert!(matches!(OcrImage::decode(b"not an image"), Err(OcrError::Decode(_))));
    }

    #[cfg(not(feature = "ocr"))]
    #[test]
    fn decode_without_feature_says_so() {
        let err = OcrImage::decode(&[0u8; 8]).unwrap_err();
        assert!(err.to_string().contains("feature `ocr`"), "{err}");
    }
}
