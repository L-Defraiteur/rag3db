//! `OcrNode` — le nœud OCR minimal (doc 41, chantier 4) : une image entre
//! (octets encodés ou [`OcrImage`] déjà décodée), le texte et les lignes
//! sortent. Le modèle est un service (`"ocr"`, `Arc<dyn Ocr>`), comme
//! `"embedder"` — le nœud ne charge rien lui-même.

use std::sync::Arc;

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};
use crate::ocr::{Ocr, OcrImage, OcrOutput};

/// Clé du service OCR dans le [`super::ServiceRegistry`].
pub const OCR_SERVICE: &str = "ocr";

/// **Input** : `image` — `Vec<u8>` (PNG/JPEG…, décodé par [`OcrImage::decode`])
/// ou `OcrImage` (déjà décodée). PortType::Image.
///
/// **Outputs** : `text` — `String` (lignes jointes par `\n`, PortType::Text) ;
/// `ocr` — [`OcrOutput`] complet (lignes, boîtes, confiances, PortType::Ocr).
///
/// **Config** : `min_confidence` (défaut `0.0`) — les lignes sous ce seuil
/// sont écartées avant la sortie.
///
/// **Métriques** : `ocr_lines`, `ocr_dropped`, `ocr_ms`.
pub struct OcrNode {
    node_name: String,
    min_confidence: f32,
}

impl OcrNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), min_confidence: 0.0 }
    }

    pub fn with_min_confidence(mut self, min_confidence: f32) -> Self {
        self.min_confidence = min_confidence;
        self
    }

    /// Lit le port `image` sous ses deux formes acceptées.
    fn take_image(ctx: &mut NodeContext) -> Result<OcrImage, String> {
        let pv = ctx.take_input("image").ok_or("OcrNode: missing 'image' input")?;
        if let PortValue::Data(ref arc) = pv {
            if arc.is::<OcrImage>() {
                return take_or_clone::<OcrImage>(pv).ok_or_else(|| "OcrNode: bad OcrImage payload".to_string());
            }
        }
        let bytes = take_or_clone::<Vec<u8>>(pv)
            .ok_or("OcrNode: 'image' must carry Vec<u8> (encoded image) or OcrImage")?;
        OcrImage::decode(&bytes).map_err(|e| format!("OcrNode: {e}"))
    }
}

impl Node for OcrNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "OcrNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::ocr_nodes::OcrNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::ocr_nodes::OcrNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let ocr = ctx
            .service::<Arc<dyn Ocr>>(OCR_SERVICE)
            .cloned()
            .ok_or("OcrNode: 'ocr' service not found")?;
        let image = Self::take_image(ctx)?;

        let started = std::time::Instant::now();
        let mut out: OcrOutput = ocr
            .recognize(&image)
            .map_err(|e| format!("OcrNode ({}): {e}", ocr.name()))?;
        let ms = started.elapsed().as_secs_f64() * 1000.0;

        let before = out.lines.len();
        if self.min_confidence > 0.0 {
            out.lines.retain(|l| l.confidence >= self.min_confidence);
        }
        let dropped = before - out.lines.len();

        ctx.metric("ocr_lines", out.lines.len() as f64);
        ctx.metric("ocr_dropped", dropped as f64);
        ctx.metric("ocr_ms", ms);
        ctx.info(&format!(
            "OcrNode ({}): {}x{}, {} lines ({dropped} dropped < {:.2}), {ms:.1} ms",
            ocr.name(),
            image.width,
            image.height,
            out.lines.len(),
            self.min_confidence
        ));

        ctx.set_output("text", PortValue::new(out.text()));
        ctx.set_output("ocr", PortValue::new(out));
        Ok(())
    }
}

// ─── Factory ─────────────────────────────────────────────────────────────────

pub struct OcrNodeFactory;

impl NodeFactory for OcrNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let min_confidence = match config.get("min_confidence") {
            None | Some(serde_json::Value::Null) => 0.0,
            Some(v) => v
                .as_f64()
                .filter(|c| (0.0..=1.0).contains(c))
                .ok_or("OcrNode: 'min_confidence' must be a number in [0, 1]")? as f32,
        };
        Ok(Box::new(OcrNode::new(name).with_min_confidence(min_confidence)))
    }

    fn node_type(&self) -> &'static str {
        "OcrNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "OcrNode",
            description: "Recognizes text in an image via the 'ocr' service (text + lines with boxes)",
            inputs: vec![PortDef { name: "image", port_type: PortType::Image, required: true }],
            outputs: vec![
                PortDef { name: "text", port_type: PortType::Text, required: false },
                PortDef { name: "ocr", port_type: PortType::Ocr, required: false },
            ],
            config_params: vec![ConfigParam {
                name: "min_confidence",
                param_type: ConfigParamType::Float,
                required: false,
                default: Some(serde_json::json!(0.0)),
                description: "Drop lines whose confidence is below this threshold [0, 1]",
                choices: None,
                json_schema: None,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::services::ServiceRegistry;
    use crate::ocr::{MockOcr, OcrLine};

    fn services_with(ocr: Arc<dyn Ocr>) -> Arc<ServiceRegistry> {
        let mut s = ServiceRegistry::new();
        s.register(OCR_SERVICE, ocr);
        Arc::new(s)
    }

    fn mock() -> Arc<dyn Ocr> {
        Arc::new(MockOcr::with_lines(vec![
            OcrLine::rect("low", 0.2, 0.0, 40.0, 50.0, 10.0),
            OcrLine::rect("hello", 0.95, 0.0, 0.0, 50.0, 10.0),
            OcrLine::rect("world", 0.9, 60.0, 1.0, 50.0, 10.0),
        ]))
    }

    fn image() -> OcrImage {
        OcrImage::from_rgb(8, 8, vec![0; 8 * 8 * 3]).unwrap()
    }

    #[test]
    fn decoded_image_in_text_and_lines_out() {
        let mut node = OcrNode::new("ocr");
        let mut ctx = NodeContext::with_services(services_with(mock()));
        ctx.set_input("image", PortValue::new(image()));
        node.execute(&mut ctx).unwrap();

        let mut outputs = ctx.drain_outputs();
        let text = outputs.remove("text").and_then(take_or_clone::<String>).unwrap();
        assert_eq!(text, "hello\nworld\nlow");
        let out = outputs.remove("ocr").and_then(take_or_clone::<OcrOutput>).unwrap();
        assert_eq!(out.lines.len(), 3);
        assert_eq!((out.width, out.height), (8, 8));
        let metrics = ctx.drain_metrics();
        assert_eq!(metrics["ocr_lines"], 3.0);
        assert_eq!(metrics["ocr_dropped"], 0.0);
        assert!(metrics.contains_key("ocr_ms"));
    }

    #[test]
    fn min_confidence_drops_lines() {
        let mut node = OcrNode::new("ocr").with_min_confidence(0.5);
        let mut ctx = NodeContext::with_services(services_with(mock()));
        ctx.set_input("image", PortValue::new(image()));
        node.execute(&mut ctx).unwrap();
        let text = ctx.drain_outputs().remove("text").and_then(take_or_clone::<String>).unwrap();
        assert_eq!(text, "hello\nworld");
        assert_eq!(ctx.drain_metrics()["ocr_dropped"], 1.0);
    }

    #[test]
    fn missing_service_or_input_is_an_error() {
        let mut node = OcrNode::new("ocr");
        let mut ctx = NodeContext::new();
        ctx.set_input("image", PortValue::new(image()));
        assert!(node.execute(&mut ctx).unwrap_err().contains("'ocr' service"));

        let mut ctx = NodeContext::with_services(services_with(mock()));
        assert!(node.execute(&mut ctx).unwrap_err().contains("missing 'image'"));

        let mut ctx = NodeContext::with_services(services_with(mock()));
        ctx.set_input("image", PortValue::new(42u32));
        assert!(node.execute(&mut ctx).unwrap_err().contains("Vec<u8>"));
    }

    #[test]
    fn encoded_bytes_go_through_decode() {
        let mut node = OcrNode::new("ocr");
        let mut ctx = NodeContext::with_services(services_with(mock()));
        ctx.set_input("image", PortValue::new(b"definitely not an image".to_vec()));
        let err = node.execute(&mut ctx).unwrap_err();
        assert!(err.contains("image decode error"), "{err}");
    }

    #[test]
    fn factory_validates_min_confidence() {
        let f = OcrNodeFactory;
        assert!(f.create("a", &serde_json::json!({})).is_ok());
        assert!(f.create("a", &serde_json::json!({"min_confidence": 0.3})).is_ok());
        assert!(f.create("a", &serde_json::json!({"min_confidence": 1.5})).is_err());
        assert!(f.create("a", &serde_json::json!({"min_confidence": "x"})).is_err());
        let schema = f.schema();
        assert_eq!(schema.node_type, "OcrNode");
        assert_eq!(schema.inputs[0].port_type, PortType::Image);
        assert_eq!(schema.outputs.len(), 2);
    }
}
