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

## llvmpipe installé : le repli `cpu` s'ouvre, mais refuse sa mémoire (15 novembre, 11 h 4x → 12 h 0x)

Lucie a installé vulkan-swrast aux deux postes. À luciepc, `vulkaninfo` voit llvmpipe (LLVM 23.1.4,
256 bits, `PHYSICAL_DEVICE_TYPE_CPU`). Le témoin A3 (`RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE=cpu`, branche
`repli-cpu-a3-2`, 3a51c7fa7) **n'entre pas** :
- **Ce qui marche** : le fils de la sonde ouvre le périphérique, `burn : Cpu → Device<Vulkan(Cpu)>`.
- **Ce qui casse** : le fil de calcul de cubecl panique, « failed to reserve 384003072 bytes of device
  memory: can't allocate buffer of size: 384003072 » (`cubecl-wgpu/src/compute/server.rs:318`). Puis
  la sonde meurt : « Memory location was never initialized » (`generated/granite_107m_onnx.rs:332`).
- **La fréquence** : rouge trois tours sur trois sur la tête reposée. Le « vert » de 11 h 43 portait la
  même réservation refusée : la sonde avait survécu par course. Ce n'était pas un vert ; il a été dit
  comme tel par erreur, puis retiré.

**La cause plausible, pas prouvée** : llvmpipe borne la taille d'une allocation, et cubecl réserve des
pages de 384 Mo. **Ce qu'il faudrait** : une page mémoire plus petite pour le périphérique CPU (la
configuration mémoire de cubecl, au périphérique `Cpu`), ou le backend ndarray. Témoin : A3 vert vingt
tours sur vingt, pas une fois, et aucune ligne « failed to reserve » dans le journal.
