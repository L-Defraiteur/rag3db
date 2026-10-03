# Le crochet après outil — proposition

**Session recherche, 3 octobre 2026.** La pièce générique que deux visions
attendent : l'idée 2 du produit code (« le même motif existe ailleurs »,
le crochet nommé par Lucie) et la mémoire longue (les mémoires accrochées
arrivent avec `read_file`, §2-7 du document 02). Proposition courte, avant
tout code.

## La forme

Un outil du manifeste peut déclarer un crochet :

```json
"edit_file": {
  "graph": "../../tools/edit.mmd",
  "policy": { "write_files": true },
  "after": {
    "graph": "../../tools/apres/motif-ailleurs.mmd",
    "title": "À voir aussi",
    "max_lines": 12,
    "threshold": 0.75
  }
}
```

- **Un crochet = une section.** Après l'exécution de l'outil, le petit
  graphe du crochet tourne ; s'il rend du texte, le rendu de l'outil
  gagne une section `### <title>` tronquée à `max_lines`. S'il rend
  vide — rien ne dépasse le seuil, l'index n'est pas prêt, le service
  manque — **la section n'existe pas**. Le droit de se taire est la
  conduite par défaut, pas un cas d'erreur.
- **Il ne peut rien écrire ni bloquer l'outil.** Le graphe du crochet est
  monté sous une liste de nœuds plus étroite que la base : la base **sans
  les nœuds d'écriture** (pas d'EntityRecord/Batch, pas de Snapshot*, pas
  d'Index). `read_files`/`write_files`/`run_commands` valent faux sauf
  déclaration explicite dans `after.policy` — même échelle que les
  outils, même refus actionnable au chargement. Une erreur du crochet se
  journalise et se tait : le résultat de l'outil part toujours, entier,
  inchangé.
- **Ce qu'il reçoit** : un port d'entrée `context` (map) — les arguments
  de l'appel, plus les sorties que le graphe de l'outil expose par ses
  ports libres. Pour `edit_file` : le chemin, l'ancien texte, le nouveau.
  C'est le mécanisme existant de composition (les ports libres d'un
  graphe-outil restent branchables), pas un canal neuf.

## Premier client : « le même motif existe ailleurs »

Après `edit_file`, chercher par similarité le code qui ressemble à
l'**ancien** texte — « tu as corrigé ici, trois autres endroits ont la
même forme ». Le graphe du crochet : construire la requête depuis
l'ancien texte (tronqué au budget d'embarquement), recherche vectorielle
sur `Scope` avec `mode=indexed` et un filtre `file_path != <chemin
édité>`, garder ce qui passe `threshold`, rendre « chemin:ligne — la
signature » borné à `max_lines`. Le seuil s'applique au score du signal
vectoriel (une similarité), pas au score de fusion (un rang) — c'est en
l'implémentant qu'on calibrera sa valeur, au banc, comme les poids.

**Latence sur une édition** : un embarquement de requête (l'ancien texte,
borné à ~4 096 caractères — c'est lui qui domine : ~50-150 ms par le
service distant au régime doux) + une recherche HNSW (quelques ms) + le
rendu. De l'ordre de **100-300 ms**, après que l'édition elle-même est
faite et réindexée — le crochet n'allonge pas le chemin d'écriture, il
s'ajoute au rendu. À mesurer et à écrire dans le doc du lot, pas à
promettre.

**Pendant une indexation, le crochet se tait** par la même porte que la
recherche adaptative : `try_lock` sur le catalogue puis `index_state()`.
La similarité exige des vecteurs — si `vectors != Ready` (ou verrou
occupé), la section n'existe pas, sans attente ni erreur. Un agent qui
vient d'éditer pendant un `index` en fond a déjà sa ligne d'état par la
recherche ; le crochet n'a rien à y redire.

## Second client, prévu sans être codé

`read_file` + un crochet dont le graphe interroge les mémoires accrochées
au fichier lu (les relations de la mémoire longue) : affirmations seules,
avec budget — exactement ce que la session mémoire demande. Même
mécanique, seule la requête du graphe change ; la proposition ne lui
réserve rien d'autre que l'existence du port `context` et du droit de se
taire. S'il faut un jour plusieurs sections (motifs + mémoires sur le
même outil), `after` devient une liste — la forme JSON le permet sans
casser l'existant (un objet seul = une liste à un).

## Ce que ça touche

`ToolAttachment.after` (manifeste + validation au chargement),
`call_tool` (monter le second graphe, concaténer la section), une liste
`HOOK_NODES` dans `backend_code.rs` (la base moins l'écriture), le gabarit
`motif-ailleurs.mmd`, la tuyauterie (une édition rend la section quand un
motif existe, se tait sinon et pendant l'index). Rien dans le moteur de
recherche : le crochet est un client du chemin composable existant.
