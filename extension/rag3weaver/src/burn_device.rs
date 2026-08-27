//! Choix du périphérique burn (wgpu), partagé par les embedders, les rerankers
//! et l'OCR. Compilé dès qu'une feature burn est active (`burn-embedder` ou
//! `burn-ocr`) ; les modules historiques le ré-exportent sous leur ancien chemin
//! (`crate::burn_bge_m3_embedder::BurnDevice`).

use burn::prelude::*;

/// Which GPU burn should run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BurnDevice {
    /// Best available device (discrete GPU if present).
    #[default]
    Default,
    /// Nth discrete GPU — useful for sharding across several cards.
    DiscreteGpu(usize),
    /// Integrated GPU.
    IntegratedGpu(usize),
    /// CPU fallback. Correct but slow; handy for reproducible reference output.
    Cpu,
}

/// À quoi sert un modèle. Le rôle décide de la carte, parce qu'un LLM et un
/// embedder n'ont pas du tout la même empreinte : Qwen3-Coder-30B occupe
/// 32 Go en permanence, BGE-M3 en prend 2,2 le temps d'un passage. Les mettre
/// sur la même carte marche jusqu'au jour où ça ne marche plus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BurnRole {
    Embedder,
    Reranker,
    Ocr,
    Llm,
}

impl BurnRole {
    /// La variable d'environnement qui décide pour ce rôle.
    pub fn env_var(self) -> &'static str {
        match self {
            Self::Embedder => "RAG3WEAVER_BURN_DEVICE_EMBEDDER",
            Self::Reranker => "RAG3WEAVER_BURN_DEVICE_RERANKER",
            Self::Ocr => "RAG3WEAVER_BURN_DEVICE_OCR",
            Self::Llm => "RAG3WEAVER_BURN_DEVICE_LLM",
        }
    }
}

impl BurnDevice {
    /// Lit une carte : `default`, `cpu`, `gpu:1`, `igpu:0`.
    ///
    /// `gpu:1` est la **deuxième carte discrète**, dans l'ordre que wgpu
    /// énumère — pas forcément celui de `rocm-smi` ni de `nvidia-smi`. À
    /// vérifier une fois par machine, et c'est pour ça que le choix est dit à
    /// voix haute plutôt que deviné.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if let Some(n) = s.strip_prefix("gpu:") {
            return n.parse().map(Self::DiscreteGpu).map_err(|_| format!("device '{s}' : après 'gpu:' il faut un entier"));
        }
        if let Some(n) = s.strip_prefix("igpu:") {
            return n.parse().map(Self::IntegratedGpu).map_err(|_| format!("device '{s}' : après 'igpu:' il faut un entier"));
        }
        match s {
            "default" | "" => Ok(Self::Default),
            "cpu" => Ok(Self::Cpu),
            other => Err(format!("device '{other}' inconnu (default | cpu | gpu:N | igpu:N)")),
        }
    }

    /// La carte d'un rôle, depuis l'environnement.
    ///
    /// La variable du rôle l'emporte sur `RAG3WEAVER_BURN_DEVICE`, qui vaut
    /// pour tous. Une valeur illisible n'est pas une raison de s'arrêter :
    /// on le dit et on prend le défaut — un modèle qui tourne sur la mauvaise
    /// carte reste préférable à un modèle qui ne tourne pas.
    pub fn for_role(role: BurnRole) -> Self {
        let raw = std::env::var(role.env_var())
            .or_else(|_| std::env::var("RAG3WEAVER_BURN_DEVICE"))
            .unwrap_or_default();
        if raw.is_empty() {
            return Self::Default;
        }
        match Self::parse(&raw) {
            Ok(d) => {
                // Dire ce qu'on a choisi : sans ça, placer trois modèles sur
                // deux cartes se fait à l'aveugle.
                eprintln!("[rag3weaver] {:?} sur {} ({})", role, raw, role.env_var());
                d
            }
            Err(e) => {
                eprintln!("[rag3weaver] {} : {e} — carte par défaut", role.env_var());
                Self::Default
            }
        }
    }

    /// Shared with the other burn embedders in this crate.
    pub(crate) fn resolve(self) -> Device {
        match self {
            BurnDevice::Default => Device::default(),
            BurnDevice::DiscreteGpu(i) => Device::wgpu(DeviceKind::DiscreteGpu(i)),
            BurnDevice::IntegratedGpu(i) => Device::wgpu(DeviceKind::IntegratedGpu(i)),
            BurnDevice::Cpu => Device::wgpu(DeviceKind::Cpu),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_is_read_from_a_short_form() {
        assert_eq!(BurnDevice::parse("gpu:1"), Ok(BurnDevice::DiscreteGpu(1)));
        assert_eq!(BurnDevice::parse("igpu:0"), Ok(BurnDevice::IntegratedGpu(0)));
        assert_eq!(BurnDevice::parse("cpu"), Ok(BurnDevice::Cpu));
        assert_eq!(BurnDevice::parse(" default "), Ok(BurnDevice::Default));
        assert_eq!(BurnDevice::parse(""), Ok(BurnDevice::Default));
        assert!(BurnDevice::parse("gpu:x").is_err());
        assert!(BurnDevice::parse("carte 2").is_err());
    }

    #[test]
    fn each_role_has_its_own_variable() {
        let vars: Vec<&str> = [BurnRole::Embedder, BurnRole::Reranker, BurnRole::Ocr, BurnRole::Llm]
            .iter()
            .map(|r| r.env_var())
            .collect();
        // Quatre rôles, quatre variables distinctes — sinon en placer un
        // déplacerait les autres.
        let mut sorted = vars.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "{vars:?}");
        assert!(vars.iter().all(|v| v.starts_with("RAG3WEAVER_BURN_DEVICE_")));
    }
}
