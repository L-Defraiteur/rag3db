# Un repli CPU des tests sans Vulkan

- **État** : ouvert (15 novembre 2026, embarquement, souhait ; décision de c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : souhait. Le repli des tests sur le CPU n'existe pas tant qu'aucun adaptateur Vulkan CPU
  (lavapipe) n'est installé : le `Cpu` de burn passe par wgpu.
- **Atteignable en service** : non (les tests).
- **Touche rag3weaver** : oui (`src/burn_device.rs`, les embarqueurs burn)

## Ce que c'est

`BurnDevice::Cpu` se résout en `wgpu_pinned(DeviceKind::Cpu)`. Sans lavapipe, cubecl-wgpu panique :
« No CPU device found, adapters [] ». C'est le cas aux deux postes au 15 novembre 2026. Le repli de
l'aide des tests (`RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE`) ne peut donc nommer `cpu` nulle part.

## Ce qu'il faudrait

Un vrai repli CPU sans Vulkan, par le backend ndarray de burn, pour les embarqueurs des tests au moins.
Sinon, lavapipe aux deux postes, ce qui est à Lucie (sudo). Témoin : `RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE=cpu`
charge granite-107m et calcule, sans GPU.
