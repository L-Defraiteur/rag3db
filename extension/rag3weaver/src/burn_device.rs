//! Choix du périphérique burn (wgpu), partagé par les embedders, les rerankers
//! et l'OCR. Compilé dès qu'une feature burn est active (`burn-embedder` ou
//! `burn-ocr`) ; les modules historiques le ré-exportent sous leur ancien chemin
//! (`crate::burn_bge_m3_embedder::BurnDevice`).

use burn::prelude::*;

/// Which GPU burn should run on.
///
/// **Deux piles, pas une.** `DiscreteGpu`/`IntegratedGpu` passent par wgpu
/// (compilateur SPIR-V, feature `vulkan`) : portable, c'est ce qui tourne sur
/// n'importe quelle carte. `Rocm` passe par HIP, sans wgpu du tout — le chemin
/// natif d'une carte AMD, disponible seulement si la feature `burn-rocm` est
/// active et si ROCm est installé.
///
/// `Device` de burn 0.22 dispatche à l'exécution, donc les deux piles
/// cohabitent dans le même binaire : le choix reste une variable
/// d'environnement, pas une recompilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BurnDevice {
    /// Best available device (discrete GPU if present).
    #[default]
    Default,
    /// Nth discrete GPU — useful for sharding across several cards.
    DiscreteGpu(usize),
    /// Integrated GPU.
    IntegratedGpu(usize),
    /// Nième carte AMD par HIP/ROCm. L'index est celui de ROCm (`rocminfo`),
    /// qui n'a aucune raison de coïncider avec celui de wgpu — à vérifier
    /// comme le reste, en regardant la VRAM bouger.
    Rocm(usize),
    /// CPU fallback. Correct but slow; handy for reproducible reference output.
    Cpu,
}

/// À quoi sert un modèle. Le rôle décide de la carte, parce que trois modèles
/// n'ont pas la même empreinte ni la même durée de vie : BGE-M3 prend 2,2 Go
/// le temps d'un passage, l'OCR quelques centaines de Mo, le reranker de même.
/// Les mettre tous sur celle qui porte l'affichage marche jusqu'au jour où ça
/// ne marche plus.
///
/// **Pas de rôle `Llm` :** notre moteur ne fait pas d'inférence de LLM
/// (décision du 28 août 2026). Un LLM vient de `llama.cpp` ou d'un
/// fournisseur distant, et c'est ce serveur-là qui choisit sa carte — pas
/// nous. `RAG3WEAVER_BURN_DEVICE_LLM` n'existe donc plus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BurnRole {
    Embedder,
    Reranker,
    Ocr,
}

impl BurnRole {
    /// La variable d'environnement qui décide pour ce rôle.
    pub fn env_var(self) -> &'static str {
        match self {
            Self::Embedder => "RAG3WEAVER_BURN_DEVICE_EMBEDDER",
            Self::Reranker => "RAG3WEAVER_BURN_DEVICE_RERANKER",
            Self::Ocr => "RAG3WEAVER_BURN_DEVICE_OCR",
        }
    }
}

/// **SPIR-V épinglé, pas choisi à l'exécution.**
///
/// `Device::wgpu()` passe par l'`AutoCompiler` de burn, qui choisit WGSL,
/// SPIR-V ou MSL *au lancement, selon les features activées*.
/// `Device::vulkan()` épingle SPIR-V à la compilation et court-circuite ce
/// choix.
///
/// On active la feature `vulkan` depuis le début — pour SPIR-V — et on
/// appelait `Device::wgpu()`, donc l'`AutoCompiler`. Ça se voyait le jour où
/// l'ajout d'un autre backend (`burn-rocm`) a changé le jeu de features et
/// donc son choix : le même `drain` de 12 documents passait de **3,1 s à
/// 71 s** sans qu'une ligne de notre code ne bouge (28 août 2026). Un choix
/// fait à l'exécution d'après les features est un choix qu'on ne contrôle
/// pas.
///
/// Sans la feature `vulkan`, on retombe sur l'`AutoCompiler` — c'est alors le
/// bon comportement, il n'y a rien à épingler.
fn wgpu_pinned(kind: DeviceKind) -> Device {
    #[cfg(feature = "vulkan")]
    {
        Device::vulkan(kind)
    }
    #[cfg(not(feature = "vulkan"))]
    {
        Device::wgpu(kind)
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
        if let Some(n) = s.strip_prefix("rocm:") {
            return n.parse().map(Self::Rocm).map_err(|_| format!("device '{s}' : après 'rocm:' il faut un entier"));
        }
        match s {
            "default" | "" => Ok(Self::Default),
            "cpu" => Ok(Self::Cpu),
            other => Err(format!("device '{other}' inconnu (default | cpu | gpu:N | igpu:N | rocm:N)")),
        }
    }

    /// La carte d'un rôle, depuis l'environnement.
    ///
    /// La variable du rôle l'emporte sur `RAG3WEAVER_BURN_DEVICE`, qui vaut
    /// pour tous. Une valeur illisible n'est pas une raison de s'arrêter :
    /// on le dit et on prend le défaut — un modèle qui tourne sur la mauvaise
    /// carte reste préférable à un modèle qui ne tourne pas.
    pub fn for_role(role: BurnRole) -> Self {
        // **Dire d'où vient le choix, pas seulement lequel.** Le message existe
        // pour qu'un humain vérifie ; nommer la mauvaise source le rend pire
        // qu'absent — on chercherait une variable que personne n'a posée.
        let depuis = |v: String, source: &'static str| (v, source);
        let choix = std::env::var(role.env_var())
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(|v| depuis(v, role.env_var()))
            .or_else(|| {
                std::env::var("RAG3WEAVER_BURN_DEVICE")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .map(|v| depuis(v, "RAG3WEAVER_BURN_DEVICE"))
            })
            // **Le régime, en dernier recours — pour les trois rôles.**
            //
            // Il ne valait que pour l'embarqueur, avec une raison
            // d'efficacité : c'est lui qui tient le modèle des heures durant,
            // un reranker ou un OCR prennent la carte le temps d'un appel.
            // La raison est bonne et ne décide plus ici : `confort` ne parle
            // pas d'efficacité mais de **ne pas être dérangée**, et un OCR qui
            // prend la carte du compositeur le temps d'un appel fait
            // précisément le tort qu'on cherche à éviter. Sous `plein`,
            // `carte_locale()` rend `None` et rien ne bouge.
            .or_else(|| {
                crate::regime::Regime::courant()
                    .carte_locale()
                    .map(|v| depuis(v, "la carte la moins chargée, régime confort"))
            });

        let Some((raw, source)) = choix else {
            return Self::Default;
        };
        match Self::parse(&raw) {
            Ok(d) => {
                // Dire ce qu'on a choisi : sans ça, placer trois modèles sur
                // deux cartes se fait à l'aveugle.
                eprintln!("[rag3weaver] {role:?} sur {raw} ({source})");
                d
            }
            Err(e) => {
                eprintln!("[rag3weaver] {source} : {e} — carte par défaut");
                Self::Default
            }
        }
    }

    /// **Le défaut consulte l'environnement ; un choix explicite, jamais.**
    ///
    /// C'est la précédence qu'on attend partout : le code l'emporte sur la
    /// variable, qui l'emporte sur le défaut. Sans ce maillon, [`Self::for_role`]
    /// n'était appelé par personne et les variables ne servaient à rien —
    /// mécanisme construit, documenté, et jamais branché (27 août 2026, en
    /// cherchant pourquoi l'embedding ralentissait le poste : les quatre
    /// mesures comparant les cartes donnaient le même temps, et pour cause).
    ///
    /// Le rôle vient de l'appelant parce que **lui seul le connaît** : un
    /// embedder sait qu'il est un embedder, `resolve()` ne le saura jamais.
    pub fn or_role(self, role: BurnRole) -> Self {
        match self {
            Self::Default => Self::for_role(role),
            explicite => explicite,
        }
    }

    /// Shared with the other burn embedders in this crate.
    pub(crate) fn resolve(self) -> Device {
        match self {
            BurnDevice::Default => Device::default(),
            BurnDevice::DiscreteGpu(i) => wgpu_pinned(DeviceKind::DiscreteGpu(i)),
            BurnDevice::IntegratedGpu(i) => wgpu_pinned(DeviceKind::IntegratedGpu(i)),
            BurnDevice::Cpu => wgpu_pinned(DeviceKind::Cpu),
            #[cfg(feature = "burn-rocm")]
            BurnDevice::Rocm(i) => Device::rocm(i),
            // **Demander ROCm sans l'avoir compilé n'est pas une panne.** On le
            // dit et on prend le défaut : la même règle que pour une variable
            // illisible. Un modèle sur la mauvaise pile reste préférable à un
            // modèle qui ne tourne pas.
            #[cfg(not(feature = "burn-rocm"))]
            BurnDevice::Rocm(i) => {
                eprintln!(
                    "[rag3weaver] rocm:{i} demandé, mais la feature `burn-rocm` n'est pas active — carte wgpu par défaut"
                );
                Device::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le chiffre qui décide de ne pas activer ROCm.**
    ///
    /// Mesuré le 28 août 2026, même test, mêmes 12 documents, `drain` seul :
    ///
    /// | | gpu:0 | gpu:1 | rocm:0 | rocm:1 |
    /// |---|---|---|---|---|
    /// | `AutoCompiler`, rocm compilé | 60,4 s | 71,5 s | 7,5 s | 35,3 s |
    /// | SPIR-V épinglé, rocm compilé | 40,7 s | 40,7 s | 7,9 s | 7,5 s |
    /// | **SPIR-V épinglé, sans rocm** | **2,94 s** | **2,99 s** | — | — |
    ///
    /// Trois choses, dans l'ordre de ce qu'elles coûtent :
    ///
    /// 1. **ROCm marche** — 4 tests verts sur les deux cartes — et il est
    ///    **2,6× plus lent** que wgpu/SPIR-V ici (7,5 s contre 2,94 s). Le
    ///    chemin natif d'une carte AMD n'est pas le plus rapide sur ce travail.
    /// 2. **Compiler `burn-rocm` dégrade le chemin wgpu d'un facteur 14**
    ///    (2,94 s → 40,7 s) sans qu'une ligne de notre code ne change. Ce n'est
    ///    donc pas une feature qu'on peut laisser dormir « au cas où » : elle
    ///    est exclusive en pratique.
    /// 3. **Épingler SPIR-V** vaut de toute façon : ça retire la variance
    ///    entre cartes (60/71 s devenus 40,7/40,7) et gagne 6 % sur le chemin
    ///    normal.
    ///
    /// Conclusion : le code ROCm reste — vingt lignes, il fonctionne, et la
    /// question se reposera à la prochaine version de burn. La feature reste
    /// **éteinte**.
    #[test]
    fn rocm_is_a_measured_choice_not_a_default() {
        // `rocm:N` se lit toujours, feature ou non : c'est `resolve()` qui
        // décide quoi en faire, et qui le dit quand il ne peut rien.
        assert_eq!(BurnDevice::parse("rocm:1"), Ok(BurnDevice::Rocm(1)));
        assert!(BurnDevice::parse("rocm:x").is_err());
        assert!(BurnDevice::parse("rocm").unwrap_err().contains("rocm:N"));
    }

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
        let vars: Vec<&str> = [BurnRole::Embedder, BurnRole::Reranker, BurnRole::Ocr]
            .iter()
            .map(|r| r.env_var())
            .collect();
        // Trois rôles, trois variables distinctes — sinon en placer un
        // déplacerait les autres. (Il y en avait quatre : `Llm` est parti avec
        // l'inférence locale, le 28 août 2026.)
        let mut sorted = vars.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 3, "{vars:?}");
        assert!(vars.iter().all(|v| v.starts_with("RAG3WEAVER_BURN_DEVICE_")));
    }
}
