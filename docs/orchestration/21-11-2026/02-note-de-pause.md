# Note de pause de c7 — 21 novembre, 15 h 3x, avant le redémarrage de la tablette

Lucie redémarre la tablette pour sortir `/tmp` de la RAM (ligne tmpfs de `/etc/fstab` commentée par elle). Les treize
sessions ont écrit leur note de pause sur leur branche ; les passes de luciepc continuent ; l'hôte 7970 est fermé
proprement par c7 avant l'arrêt. Le rapport du matin est la page 01.

## L'état

- **master** : `c23436281a` (PR 429), l'arbre de Lucie y est avancé. Atterris cet après-midi : dossier sans index et
  index à la demande (405, 406), le nom de fichier d'une erreur de script (407), l'offre d'index (415), le registre des
  racines accordées (418), l'accueil en arbre des agents (419), `/ide/run` (423), `poste cpu` / `poste gpu` (426), les
  procédures en `poste cpu` (427), le correctif du confineur (428), le dossier non-git (429), les graines (414, 425),
  le rapport du matin (421), la page des juges (424).
- **Le pousseur** (luciepc) tourne ; en file : `ticket-fsm-finalize-race`, `emb-reuse-by-hash`, `poste-gpu-english-names`,
  `keyed-links-1`, `index-master-ff-2`, `poste-sccache-server-outside`, `arbre-sandbox-roots`, `fermer-sans-detruire-3`,
  `ed-scripted-nodes-hot-reload`, `memoire-attribution` (à renommer `appeler_pour` → `call_for` avant redépôt).
- **À annoncer aux sessions à la reprise** : `confiner-poste-classes` a atterri (586528701d, PR 428) ; validations et
  e2e de nouveau permis.
- **Disque de la tablette** : 374 Go libres (4,6 Go à 13 h 1x) ; nos targets et copies effacés sur l'autorisation de
  Lucie ; bâtis et passes à luciepc désormais. Faute de c7 : un build/ de série de cœur effacé en vol (exe non regardé).
- **Le modèle de Lucie** : `~/models/ornith-1.5-122b-uncensored/Ornith-1.5-122B-Q4_K_M-UNCENSORED.gguf`
  (dealignai, 79 371 735 200 octets, sha256 `10770e75…a0b8266f` vérifiée). **À lancer après le redémarrage**, en
  contexte 65 536 avec garde mémoire (`lancer-ornith-64k.sh` du scratchpad de c7, mêmes options que `llm-serve ornith`).

## Décisions de l'après-midi

- Occurrences : voie (a), liens multiples à clé ; le contournement d'octobre retiré (`keyed-links-1`), le moteur
  prouvé sain sur les liens parallèles ; pas 3 en cours (Postgres migré à l'enregistrement, refus nommés).
- Mémoire : `author` posé par l'hôte (M7) ; fiches de memoire-longue ouvertes aux sessions (miroir md vérifié) ;
  garde-fou de doublons en code à venir.
- Mode auto : accord de racine en lecture par défaut ; dossiers protégés en dur (jeton, cookie, magasins
  d'identifiants, `~/.config/rag3weaver`) ; `by.kind = human` posé par l'hôte (jeton d'utilisateur) ; contenu du dehors
  marqué non sûr (tty) ; Qwen3.5-9B oui/non recommandé comme premier juge (banc).
- MCP de code : consigne « MCP d'abord » et relevé des ratés (`~/.local/state/rag3weaver/mcp-misses.tsv`, tenu par
  recherche) ; hôte de luciepc éveillé, index de master, tunnel 7971 ; un client à plusieurs hôtes (page d'archi).

## Ce qui attend Lucie

1. Qwen3.5-9B comme premier juge du mode auto ; les trois juges intermédiaires à luciepc (27,5 Go) ; Inkling (ggml-org
   17,6 Go ou abandon).
2. La publication de lucivy 4.4.1 si la mesure en release de (A) dépasse 5 %.
3. `CLAUDE_CODE_TMPDIR` pour les sessions (devenu sans objet si `/tmp` est sur le SSD).
