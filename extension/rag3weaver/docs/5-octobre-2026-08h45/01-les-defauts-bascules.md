# Les défauts basculés : plein texte en fichiers, transaction par paquet, paquets de 2 048

*Préparée le 5 octobre 2026 sur la branche `defauts-bascules`. **Ce sont les défauts
depuis le 10 octobre 2026** (master `21a1d67c5`), après le oui de Lucie.*

## Les trois défauts

| | Avant | Après | Pour revenir en arrière |
|---|---|---|---|
| Où vit le plein texte d'une base **neuve** sur disque | dans la base (`_index_blobs`) | dans des fichiers à côté d'elle, `<base>.fts/` | `RAG3WEAVER_FTS=blobs` (et `RAG3WEAVER_FTS=fichiers` force les fichiers) |
| Transaction par paquet (synchronisation d'une source, mode en masse) | coupée, `RAG3WEAVER_TX_PAR_PAQUET=1` pour l'activer | **active** | `RAG3WEAVER_TX_PAR_PAQUET=0` |
| Fichiers par paquet | 64 | **2 048** | `RAG3WEAVER_BATCH_FILES=64`, ou l'option `batchFiles` d'une synchronisation, qui l'emporte |
| Paquets par validation (K) | 1 | 1 (inchangé) | `RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION` |

Le reste ne change pas. Les autres réglages de la transaction par paquet restent ce qu'ils
étaient : `RAG3WEAVER_TX_SANS_NAISSANCES`, `RAG3WEAVER_TX_POUSSEE_A_LA_FIN`,
`RAG3WEAVER_TX_SANS_PREUVE` et `RAG3WEAVER_TX_AVEC_POINTS_DE_REPRISE`.

Quand le plein texte vit dans la base (base existante, ou `RAG3WEAVER_FTS=blobs`), chaque
paquet demande le point de reprise forcé de ses COPY (`force_checkpoint_on_copy=true`).
Ses blobs ne passent donc pas par le journal, où ils seraient écrits deux fois (+30 à
40 s). En fichiers, c'est le défaut du moteur qui s'applique.

## Ce qui change pour un utilisateur

**Où vit le plein texte sur le disque.** Pour une base neuve sur disque, un dossier
`<base>.fts/` apparaît à côté de `<base>`. Il fait partie de la base : on le copie, on le
sauvegarde et on le supprime avec elle. Une base en mémoire garde son plein texte en elle.

**Une base existante dont le plein texte est en blobs le garde.** Il n'y a ni migration
silencieuse ni refus : à l'ouverture, une base qui a des blobs dans `_index_blobs` et
pas de `<base>.fts/` reste en blobs. L'écrivain et ses lecteurs font le même choix. Pour
passer une base existante en fichiers, il faut la réindexer, ou poser
`RAG3WEAVER_FTS=fichiers` : son plein texte est alors rebâti depuis les lignes à la
première synchronisation (la garde des comptes le voit vide).

**Le pic de mémoire baisse.** Mesures de la session embarquements sur le dépôt entier
(environ 7 000 fichiers, disque, tampon de 8 Gio, médiane de trois passes) :

| | Durée | Pic de mémoire | Réouverture |
|---|---|---|---|
| blobs, paquets de 512 | 107 s | 15,4 Go | 2,2 s |
| **fichiers, paquets de 2 048 (la bascule)** | **78–79 s** | **9,1–9,4 Go** | **0,3 s** |
| blobs, paquets de 2 048 | 78 s | 14,2 Go | 2,0 s |

**La première chose cherchable arrive plus tard.** Avec la transaction par paquet, rien
n'est visible avant la validation du premier paquet. Plus le paquet est grand, plus
l'attente est longue (mesuré par `e2e_estimate`, « première chose cherchable ») :

| Paquets | Première chose cherchable |
|---|---|
| 2 048 × K = 1 (la bascule) | 8 à 10 s |
| 512 × K = 1 | 2 s |
| 64 (avant) | moins de 2 s (estimé, non mesuré) |
| 2 048 × K = 4 | 57 s (une seule validation, à la fin) |

**Ce qu'un arrêt brutal laisse.** Un paquet en cours est défait en entier, et une reprise
le refait. Les paquets validés restent. Depuis le 5 octobre (`d74c0877e`, moteur
`37608cf4b`), une mort pendant une validation laisse le paquet entier ou absent, jamais à
moitié.

**Un COPY refusé par le moteur arrête la synchronisation.** C'est un filet qui ne dépend
pas de la bascule (`f0f7f0951`). L'erreur est nommée et la base est à rouvrir, au lieu
d'un repli silencieux. Sur le dépôt entier, avec un tampon d'au moins 8 Gio, le compte est
0 dans toutes les passes mesurées.

## Les tests ajustés, et pourquoi

- `e2e_tx_ligne_a_ligne`, `e2e_tx_par_paquet_arret`, `e2e_reprise_durcie` : leurs fils
  retiraient `RAG3WEAVER_TX_PAR_PAQUET` pour dire « sans transaction ». Avec le nouveau
  défaut, l'absence de variable veut dire « avec ». Ils posent désormais `=0`. Aucun seuil
  n'est changé.
- `e2e_tx_par_paquet_arret` : ses cas « en blobs » posent explicitement
  `FtsStorage::BlobBacked`. Une base neuve sur disque prenant les fichiers par défaut, ils
  auraient sinon éprouvé les fichiers sans le dire. Ses cas « en fichiers » les posaient
  déjà.
- Nouveau témoin : `e2e_stockage_du_plein_texte` (base neuve en fichiers, base existante
  en blobs gardée sans dossier `.fts`, variable, base en mémoire, lecteur au même choix).
- Les autres suites jouées pour ce changement passent sans retouche : la lib, `e2e_code`,
  `e2e_code_sync`, `e2e_synchronisation`, `e2e_search`, `e2e_scope`, `e2e_plans_par_lot`,
  `e2e_graphe_et_paquets`, `e2e_estimate`, `e2e_prise_atomique`, `e2e_working_tree`,
  `e2e_impact_fichier`, `e2e_rouvrir`, `e2e_reprise_durcie`, `e2e_preuve_d_existence`,
  `e2e_copy_refuse_rouvre`, `e2e_tx_vecteurs_apres_rollback`,
  `e2e_lecture_seule_apres_fermeture_sans_point_de_reprise`.
- Deux suites ajustées à la fusion (10 octobre) :
  - `e2e_points_de_reprise_nettoyes` : une source neuve passe en transaction par paquet,
    qui coupe les points de reprise du dataflow. Rien n'était écrit, et la garde « le test
    ne prouverait rien » tombait. Son fils pose `RAG3WEAVER_TX_PAR_PAQUET=0`.
  - `e2e_agent_loop` : son rouge ne venait pas de la bascule (l'assertion d'un rendu changé
    le 4 octobre, rouge aussi sur master).
- **La batterie complète** a été jouée une fois avant la fusion, le 10 octobre (hors carte
  locale, régime doux) : 72 suites vertes sur 74, et les deux rouges sont ceux ci-dessus.
  Après le rebase sur master, avec la lib commune rebâtie sur la version de stockage 40,
  la lib et sept suites ont été rejouées, toutes vertes.

## Ce que Lucie a décidé (10 octobre)

1. La bascule : oui.
2. Les bases existantes : la coexistence. Pas de migration à l'ouverture, pas de refus.
3. Le délai avant la première chose cherchable, environ 9 s au lieu de moins de 2 s :
   accepté.

La transaction par paquet suit aussi ce que déclare le dialecte (`DialectCapabilities`,
commit `a67c083a3`). Sur un dialecte sans `transactions`, elle est coupée, et le rapport
de synchronisation le dit dans `warnings`. Demandée explicitement
(`RAG3WEAVER_TX_PAR_PAQUET=1`), elle donne un refus nommé.
