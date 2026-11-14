# L'aide des tests panique quand aucun service listé ne sert le modèle demandé

- **État** : fermé (15 novembre 2026, embarquement) : l'aide se replie sur un périphérique nommé (`RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE`), et rougit en le nommant sans lui ; ouvert le 14 novembre, arbre principal.
- **Famille** : produit
- **Gravité** : un rouge d'environnement à chaque passe à luciepc qui joue `e2e_estimate` sous les
  services ; il se rejoue vert à part, mais il coûte la passe à qui ne le sait pas.
- **Atteignable en service** : non (les tests).
- **Touche rag3weaver** : oui (`tests/common/mod.rs`, `burn::par_le_service`, l. 241-249).
- **Pour** : embarquement.
- **Signalé par** : arbre principal, passe des suivies à luciepc, 14 novembre 2026, 23 h 12 ; sur la
  décision de c7 pour la forme attendue.

## Ce qu'on voit

`e2e_estimate::ce_depot_s_estime_avant_de_s_indexer` rougit à luciepc sous
`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879` :

> RAG3WEAVER_EMBED_SERVICE : aucun service ne sert granite-107m — 127.0.0.1:7878 sert granite-278m ;
> 127.0.0.1:7879 sert bge-m3

`par_le_service` appelle `DaemonEmbedder::from_service(modele)` ; une variable posée qui ne sert pas le
modèle rend `Some(Err)`, et l'aide **panique** (`unwrap_or_else(|raison| panic!(…))`). Sans la variable,
elle rend `None` et le modèle se charge sur place. Le 7880 (granite-107m) est arrêté et désactivé à
luciepc depuis le 12 novembre (mot de Lucie, ticket fermé
`2026-11-12-revoir-les-services-d-embarquement-residents.md`) : aucune passe à luciepc qui embarque par
service ne peut donc jouer cette suite verte.

Rejoué sans la variable (le modèle sur place), le test est vert : ce n'est pas le code testé.

## Ce qu'on voudrait (décision de c7)

**Le repli, pas « SAUTÉ »** (un test sauté n'est pas vert) : pour un modèle qu'aucun service listé ne
sert, l'aide **charge le modèle sur place en le disant**, comme sans la variable (« ▸ granite-107m sur
place : aucun service listé ne le sert »). La panique reste pour un service listé qui sert le modèle mais
refuse ou se trompe de dimension : là, c'est une vraie panne.

## Le témoin à écrire

Sous une variable qui liste des services dont aucun ne sert le modèle, `par_le_service` rend `None` et le
dit ; sous une variable dont un service sert le modèle, il rend ce service ; un service qui sert le modèle
avec une autre dimension reste une panique nommée.

## Le correctif (15 novembre 2026, embarquement, branche `repli-de-l-aide`)

- **La forme retenue** (c7, en deux temps). D'abord « le repli, pas SAUTÉ, jamais sur une carte
  épargnée, le CPU si rien n'est dit ». Puis, au vu du poste : **pas de défaut CPU qui n'existe pas**.
  Le `Cpu` de burn passe par Vulkan (`wgpu_pinned(DeviceKind::Cpu)`), et aucun des deux postes n'a
  d'adaptateur Vulkan CPU. À luciepc, `vulkaninfo` ne voit que les deux R9700 et l'Intel ARL ; au
  portable, que le Radeon 8060S intégré. La sonde sur `cpu` paniquait : « No CPU device found ».
- **Le repli, seulement nommé** : `RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE` vaut `gpu:N`, `igpu:N`, ou
  `cpu` là où un adaptateur CPU existe. Absente ou `non` : le test est rouge, « pas de service pour
  <modèle> : … ; aucun périphérique de repli nommé ». L'aide dit le repli
  (« ▸ granite-107m sur place (IntegratedGpu(0)) : … »). Le service qui sert le modèle mais se trompe
  reste une panique nommée (l'assertion de bge-m3).
- **À luciepc, `igpu:0`** est l'Intel ARL (card1, i915). Relevé du 15 novembre, 00 h 2x : ses quatre
  sorties sont « disconnected », il ne porte aucun écran. Les écrans sont sur card0 (HDMI-A-3) et sur
  card2 (DP-6, HDMI-A-2). Les passes de luciepc le poseront dans leur environnement (lot de G,
  `passe_epinglee.sh`).
- **Les témoins** (`tests/e2e_aide_des_tests_repli.rs`, la sonde relancée dans son processus par
  `test_de_cette_suite!`), rouges d'abord puis verts :
  - A1 : `igpu:0`, le vrai modèle chargé ;
  - A2 : sans variable, rouge nommé ;
  - B : `non`, rouge nommé.
- **Restent** : lavapipe aux deux postes (sudo, question posée à Lucie par c7), et le ticket souhait
  `docs/embarquement/tickets/2026-11-15-un-repli-cpu-des-tests-sans-vulkan.md`.
