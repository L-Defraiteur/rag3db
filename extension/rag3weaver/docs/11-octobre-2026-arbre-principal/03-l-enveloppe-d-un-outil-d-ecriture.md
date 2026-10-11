# L'enveloppe d'un outil d'écriture — atomique, ou dit pourquoi pas

*Arbre principal et mémoire longue, 11 octobre 2026. La page avant le code, demandée
par l'orchestration : la machinerie de la transaction par paquet, portée à un outil de
backend. Chaque section dit qui la tient. **Rien n'est codé.***

**D'où ça vient.** Le nœud de lien dit maintenant la vérité sur `atomic`
(`2d0687a71`). En une instruction, il est atomique par le moteur. En plusieurs
instructions de liens seuls, il est enveloppé de `begin` / `commit` / `rollback`, ce
qui est sûr puisqu'un lien n'a aucun effet hors de la base. Quand sa fermeture emporte
une création, il dit `false` avec sa raison. Ce dernier cas, et plus largement **tout
outil qui écrit des lignes**, est le sujet de cette page. Le contrat visé tient en une
phrase : *un outil qui écrit est atomique, ou il dit qu'il ne l'est pas, et ce qui est
resté.*

## 1. Les outils d'écriture des gabarits livrés, et ce que chacun écrit

*Mémoire longue. Recensé sur master le 11 octobre (gabarits `code`, `memory`,
`notebook`, `validated-result`, `proto/boutique`).*

| outil | nœud | ce qu'il écrit | atomique aujourd'hui | ce qu'il dit sur un échec |
|---|---|---|---|---|
| `put_*`, `save_*`, `submit_result` (tous les gabarits sauf `code`) | `EntityRecordNode` (`put`) | **une** ligne, par `ingest_entities` : la ligne, ses morceaux, son plein texte (fichiers), ses vecteurs si l'entité en déclare | **non** : plusieurs instructions, et des écritures hors de la base | « write/index failure (inspect state before retry) » et les avertissements — **sans dire ce qui est resté**, alors que le drain le sait (`rendu_pret`) |
| `create_ref_type` (`memory`) | `CreateRefTypeNode` | une ligne `RefType` : même chemin qu'un `put` | non, pour la même raison | *(à relire)* |
| `add_ref` (`memory`) | `AddRefNode` | une `Ref` (ligne, plein texte), **puis** un lien `CITE` | non : deux écritures séparées | fiche absente ou hors état : « écrite seule », dit. **Lien échoué après la `Ref` : une erreur qui ne dit pas que la `Ref` est restée** |
| `anchor`, `supersede_link` (`memory`) | `RelationBatchNode` | des liens | **oui**, ou dit pourquoi pas (`2d0687a71`) | « rolled back, nothing written », ou « writes may be partial » avec sa raison |
| `edit_file` (`code`) | `EditFileNode` | un fichier sur disque, puis sa réindexation | non : le fichier est hors de toute transaction | *(à relire)* |
| `index` (`code`) | `IndexNode` | un **travail**, avec ses propres transactions par paquet | hors du sujet : un travail long, pas un appel | son journal |
| `run_command` (`code`) | `RunCommandNode` | rien en base | hors du sujet | son code de sortie |

**Ce que le recensement montre.** Dans les gabarits livrés, l'outil qui écrit le plus
souvent écrit **une** ligne. `EntityBatchNode` et les outils d'instantané n'apparaissent
dans aucun gabarit livré : seule l'expérience MTG les déclare. Le cas courant à rendre
honnête est donc « une ligne avec ses dépendances hors base », plus que « plusieurs
lignes ».

## 2. Le contrat, vu de l'outil et de l'agent

*Mémoire longue.*

1. **Tout outil d'écriture rend `atomic`**, et `atomic_reason` quand c'est `false`,
   comme le nœud de lien depuis `2d0687a71`. Un agent n'a pas à deviner, d'après le nom
   d'un outil, si un échec le laisse dans un état mixte.
2. **Un échec dit ce qui est resté**, pas « inspect state ». Le drain sait déjà ce qui
   est prêt (`FlushResult.rendu_pret` : la donnée, le plein texte, le dense, le creux).
   Un échec qui le rend dit à l'agent si la ligne existe, sans sa recherche. Et un échec
   qui n'a rien laissé le dit aussi, puisque c'est la bonne nouvelle.
3. **`add_ref` dit que sa `Ref` est restée** quand le lien échoue après elle, comme il
   le dit déjà pour une fiche absente.
4. **Vu d'un client MCP**, rien ne change dans le protocole : le refus part en
   `isError` avec le texte du nœud, et le succès porte `atomic` dans son JSON. Ce qui
   change, c'est ce que dit le texte.

Ce contrat ne demande **aucune transaction** : il demande de dire la vérité. C'est la
première marche, et elle tient seule.

## 3. Ce que la transaction par paquet garantit, ce qui vit hors de la base, et le prix de l'empoisonnement

*Arbre principal, à écrire.* `commencer` / `terminer` de `code_sync`,
`set_in_transaction`, les points de reprise coupés, l'empoisonnement après `ROLLBACK`,
`close_without_checkpoint` ; le plein texte en fichiers, les blobs, les vecteurs, et
comment le paquet les rattrape à la reprise.

**La question dure, et la première réponse de l'arbre principal** (marche 1 de C) : sous
l'hôte, un backend dont la base doit être rouverte est lâché seul. Son appel en cours
rend une erreur transitoire nommée, jamais rejouée, et l'appel suivant le rouvre. Les
autres backends ne voient rien, mais **les autres sessions du même backend perdent leurs
appels en vol, et leurs caches avec**. Un outil qui empoisonne son catalogue coûte donc
une réouverture à tous les clients de ce backend.

## 4. Les formes possibles, et leur coût

*À deux, à écrire.* De la moins chère à la plus chère :

- **dire** (§2) : `atomic: false` partout où ce n'est pas sûr, et ce qui est resté après
  un échec ;
- **envelopper ce qui est sûr** : comme pour les liens, une transaction seulement quand
  tout ce que l'outil écrit vit dans la base ;
- **différer ce qui vit hors de la base au `commit`** : le plein texte et les vecteurs
  écrits après la validation de la transaction de la base, rattrapés par la dette si le
  processus meurt entre les deux ;
- **la machinerie complète du paquet**, empoisonnement compris, avec son prix au §3.

## 5. Les témoins

*Mémoire longue pour la part outil ; arbre principal pour la part transaction.*

- **un `put` qui échoue après la ligne et avant ses vecteurs** dit que la ligne existe et
  que sa recherche dense manque (faute injectée, sur le modèle de celle du drain des
  liens : deux variables, jamais l'une sans l'autre) ;
- **un `add_ref` dont le lien échoue** dit que la `Ref` est restée ;
- **chaque outil d'écriture des gabarits livrés rend `atomic`** : un filet sur le modèle de
  `src/gabarits.rs`, qui rougit quand un gabarit ajoute un outil qui ne le dit pas.

## 6. Ce qui reste à Lucie

*À écrire en dernier.*
