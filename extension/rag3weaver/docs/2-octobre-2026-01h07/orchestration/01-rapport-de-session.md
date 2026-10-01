# Rapport de session — orchestration, nuit du 1er au 2 octobre 2026

Écrit à 01 h 25, après que les cinq sessions actives ont rendu leurs documents
et cessé de travailler. Lucie a arrêté la séance pour la nuit.

## En dix lignes

- **Master est à `ffbde599c`**, vert, poussé. Rien ne tourne : aucune
  compilation, aucune suite, aucune session au travail.
- **Livré cette nuit sur master** : les deux correctifs du journal d'écriture,
  le correctif de reprise après panne de l'index de clé primaire,
  `sparse-vector` 4.3.0, les trois suppressions côté recherche, l'écriture en
  lot sous `Lifecycle`, les poids granite régénérés, la référence du banc de
  recherche.
- **Cinq branches en cours**, toutes poussées, aucune fusionnée (§2).
- **L'objectif décidé par Lucie** : les écritures parallèles, jusqu'à
  plusieurs processus, avant le produit. Le plan existe, ses deux premières
  marches sont entamées, et le banc a prouvé que le mode multi-écrivains
  corrompt en silence (cinq cas rouges, dont deux que le plan n'avait pas).
- **Rien n'est parti dehors** : ni PR vers tracel-ai, ni demande au support
  GitHub.
- **Pour reprendre** : §6. Le registre des chantiers est
  `docs/journal-des-chantiers.md` ; les documents de chaque session sont dans
  les dossiers voisins de celui-ci.

## 1. Ce qui est sur master

| Quoi | Commit | Preuve |
|---|---|---|
| Journal d'écriture : les enregistrements de plus de 4096 octets gardent leur début | `955b1b136` | test rouge avant ; 1103 tests Rust, neuf e2e |
| Journal d'écriture : une fin coupée rouvre au dernier COMMIT, ce qui est retiré est copié à côté | `a66bb0b9d` | `WalTornEndTest`, transaction_test 55, 1216 tests Rust |
| Reprise après un point de reprise interrompu (index de clé primaire, défaut hérité de Kuzu) | `6bf46150b` | test rouge avant ; transaction_test 56, **api_test 102/102**, lib 1101, huit e2e, trois scripts Python |
| `sparse-vector` 4.3.0 (sources identiques à 4.0.1) | `7c653f66c` | suites de recherche |
| `fuse_results`, la grappe d'exploration, `search_with_strategy` retirés ; leurs tests portés | `01791e347` | même nombre de cas avant et après |
| Un lot peut écrire une entité à `Lifecycle` (tout ou rien, par les gardes de transition) | `213eeb214` | `test_backend_lifecycle_batch.py`, rouge avant |
| Poids granite régénérés, prouvés contre l'ONNX d'IBM (3e-7 en f32), références en fixtures | `88495d5ba` | suite granite 11/11 |
| Référence du banc de recherche sur granite-278m | `504b86168` | tel quel 0,333 ; fonctions et méthodes 0,412 |
| Épinglage des forks sur les commits recréés | `04b5052ec` | burn recompilé, `e2e_burn_embedder` 4/4 |

**Attention à `api_test` 102/102** : le rouge connu
(`LecteursConcurrents.CeQueLeLecteurVoitEstCoherent`) est passé vert à cette
passe. La course du lecteur n'est pas corrigée pour autant — le correctif de
`DiskArray` en a retiré une cause, la revérification à l'ouverture (marche 1)
n'est pas fusionnée. **Le tenir pour intermittent**, pas pour vert, jusqu'à la
marche 1.

## 2. Ce qui est en cours, par branche

Toutes poussées sur origin. Aucune n'est à fusionner sans contrôle.

| Branche | Commit | Arbre | Session (sujet) | État | Premier geste à la reprise |
|---|---|---|---|---|---|
| `lecteur-reverifie-a-l-ouverture` | `6bc8ce632` | `../rag3db-moteur` | cœur C++ | Marche 1, « en cours » : crochet de test, quatre tests déterministes, revérification ; tests ciblés 17/17, suites complètes non rejouées | Rebaser sur master, rejouer les suites, mesurer le coût sur un gros journal, me rendre le tableau |
| `reprise-apres-panne-index-cle-primaire` | `a486fde9c` | `../rag3db-moteur` | cœur C++ | Le correctif est sur master ; il reste un commit de test seul (la panne pendant la phase de stockage, 180 interruptions, 38 s) | Relire ce test, le faire fusionner avec la marche 1 |
| `banc-de-concurrence` | `00b2a0263` | `../rag3db-banc` | banc | Étape 1 faite (C0 à C3), étape 2 en cours, compile | La session cœur C++ relit l'étape 1 ; puis fusion ; puis fin de l'étape 2 |
| `pas-c-ponderations` | `130984f61` | `../rag3db-pas-c` | recherche | Quatre tests de l'ordre de priorité écrits, logique à coder ; son rapport porte le dessin complet | Lire le rouge, coder l'étape 1, puis l'étape 2 ; arrêt avant fusion |
| `synchronisation-par-perimetre` | `325134ff0` | arbre principal | produit | `SnapshotConfig` posé dans `EntityConfig` ; le reste à écrire, dans l'ordre de son rapport | Rebaser, tests rouges sur l'entité synthétique, puis la marque, l'appel de fin, les garde-fous |

À supprimer par Lucie (je n'ai pas le droit de supprimer une branche
distante) : `origin/fin-de-journal-dechiree`, périmée, et les huit branches
du 18 septembre déjà dans master. La commande est au §5.

Dossiers de travail laissés sur le disque : `../rag3db-moteur`,
`../rag3db-banc`, `../rag3db-pas-c` (utiles), et deux restes à vérifier avant
de les retirer : `../rag3db-docs-banc` et `../rag3db-pasC`.

## 3. Ce que Lucie a décidé cette nuit

1. **Les écritures parallèles avant le produit**, « peu importe ce que ça
   coûte », **jusqu'à plusieurs processus**, la version dans un seul processus
   d'abord si elle est sur le même chemin (elle l'est, pour la détection des
   conflits).
2. **Le second écrivain attend, sur un verrou par clé**, au lieu d'échouer.
3. **Pour un choix du moteur : faire comme PostgreSQL et Neo4j**, sauf quand
   on sait faire mieux ; ne lui rendre que les écarts.
4. **Ne pas tout garder dans un seul fichier** : la découpe par genre d'index
   est une forme à mettre au plan.
5. **Le pas C** : oui ; les pondérations se règlent dans les graphes ; ordre de
   priorité C (appelant > choix du graphe > entité > défaut du gabarit >
   moteur). Poids de fusion : mesurer au banc, sparse compris. Le nom embarqué
   avec le corps : plus tard. Les trois suppressions : faites.
6. **La synchronisation par périmètre** : oui aux six recommandations ; et une
   idée d'elle, la mise de côté avant purge.
7. **« Ça ne doit pas faire que du code »** : rien d'orienté dans le moteur,
   pas d'entité « source » intégrée.
8. **Son adresse professionnelle ne doit apparaître dans aucun de ses
   dépôts.** Les forks ont été réécrits, supprimés par elle, recréés.
9. **Les PR vers tracel-ai** : d'accord pour les proposer, après vérification
   que l'amont n'a pas déjà corrigé (fait : aucune ne l'est).
10. **Les poids granite** : régénérer, publier sur Hugging Face (en privé pour
    commencer).
11. **La base MTG** : on la laisse tranquille.

## 4. Ce qui attend Lucie

| Question | Ma recommandation |
|---|---|
| Le délai par défaut de la mise de côté avant purge (jours, ou « jusqu'à la synchronisation suivante ») | 7 jours, réglable par entité |
| L'annulation en bloc d'une fin de synchronisation : rien dans le catalogue n'annule un lot aujourd'hui ; la mise de côté la fournirait presque entièrement | La faire avec la mise de côté, pas avant |
| La forme des envois à tracel-ai : sept PR, ou trois envois avec une issue d'abord | Trois envois, issue d'abord : ce sont leurs règles |
| Signaler à l'amont (Kuzu, Vela, Ladybug) le défaut de reprise après panne | Oui, le message est prêt ; c'est une prise de parole publique, donc son mot |
| Passer en public les deux dépôts Hugging Face de granite | Oui, comme ses sept autres |
| La note sur les verrous, quand elle arrivera | Ne trancher que les écarts aux moteurs établis |
| Garder entier le test de reprise de 38 secondes | Oui |
| Le budget de reprise du lecteur (250 ms contre un pic à 567 ms) | Attendre la marche 1 : le pic était peut-être cette course |
| Le ménage : branches distantes, deux dossiers de travail en trop, cinq fichiers non suivis à la racine | Les branches : la commande du §5 |

Après la marche 6 du plan, lui reposer la question de la cible à plusieurs
processus : d'après la session cœur C++, les marches 1 à 6 couvrent le besoin
du produit, et la suite coûte deux à trois mois.

## 5. Ce qui attend dehors, et que seule Lucie peut faire

- **La demande au support GitHub**, pour purger les neuf anciens commits des
  forks qui portent son adresse professionnelle. Ils ne sont plus sur aucune
  branche, mais répondent encore à qui connaît leur identifiant, y compris par
  les dépôts de tracel-ai. Texte prêt : `.vault/demande-support-github-forks.md` ;
  contrôle après coup : `.vault/controle-purge-forks.sh` (dix-huit
  « introuvable » attendus).
- **Supprimer les branches distantes périmées** :
  ```
  git push origin --delete fin-de-journal-dechiree banc-ponderation banc-ponderation-suite doc-dernier-chemin-parallele embarquements heuristique-taille mtg-experiments nettoyage-apres-monolithe retrait-monolithe-recherche
  ```
  Toutes sont entièrement dans master, sauf `fin-de-journal-dechiree`, dont le
  contenu y est sous d'autres identifiants (rebasé).
- **Rapporter les poids d'origine de l'ancien PC** : la note
  `RECUPERER-POIDS-RAG3WEAVER.md` est à la racine de sa clé USB (montée sur
  `/run/media/lucied/lucied`). Les poids de l'OCR, eux, ne sont nulle part
  ailleurs.

## 6. Comment reprendre

### Pour Lucie

Rouvrir cette session et dire « on reprend ». Si les autres terminaux ont été
fermés, les rouvrir ; leurs noms auront peut-être changé, ce n'est pas grave.
Répondre, quand elle veut, aux questions du §4 : aucune ne bloque le
redémarrage.

### Pour la session qui orchestre

1. **Lire**, dans cet ordre : la mémoire (`orchestrer-les-sessions-rag3db`),
   `docs/journal-des-chantiers.md`, ce rapport, puis le
   [02](02-knowledge-dump.md).
2. **`ListAgents`**, et redemander son sujet à chaque session `rag3db-xx`. Une
   session revenue sans historique reçoit son dossier (`<sujet>/01` et `02`)
   à lire avant toute consigne.
3. **Vérifier l'état par git** avant de relancer quoi que ce soit :
   `git fetch`, la tête de master, les cinq branches du §2, `git status` de
   l'arbre principal (il doit être sur master, propre).
4. **Relancer dans cet ordre.** Les dépendances sont entre parenthèses.
   1. **Cœur C++** : la marche 1 — rebaser, suites complètes, tableau. Puis
      relire le banc. Puis le refus après un point de reprise échoué (vérifier
      d'abord dans la documentation de PostgreSQL). Puis la note sur les
      verrous. Lui transmettre dès le début les deux rouges nouveaux du banc :
      C4 (des virements créent de l'argent, mécanisme non identifié) et C6
      (suppression et mise à jour de la même ligne validées toutes deux).
   2. **Produit** : quand je lui rends la marche 1 relue, elle reconstruit
      `build/lecteurs-csv` et rejoue les suites Rust (c'est la seule session
      qui livre le C++ sur master). Entre-temps : la synchronisation par
      périmètre, tests d'abord, arrêt avant fusion.
   3. **Banc** : finir l'étape 2 ; fusionner l'étape 1 dès que la session
      cœur C++ l'a relue (dépend de 1) ; préparer les trois cas d'attente sur
      verrou (dépend de la note sur les verrous).
   4. **Recherche** : coder les étapes 1 et 2 du pas C, arrêt branche poussée ;
      je relis ; puis le banc de recherche contre la référence du 2 octobre
      pour poser les poids de `Scope`.
   5. **Optimiseur** : finir les vérifications des PR 7, 3, 2, 5, puis les
      vérifications complètes de burn, selon la forme que Lucie aura choisie.
      Rien ne part sans son mot. Régénérer les poids de l'OCR si Lucie ne les
      rapporte pas.
5. **Budget** : `-j8` par session qui compile, `-j6` pour une reconstruction
   de burn ; un test sensible au temps qui rougit se rejoue seul.
6. **Ensuite, dans l'ordre du plan** (`docs/2-octobre-2026-00h17/01-…`, §12) :
   les corrections des écritures parallèles contre le banc, le point de
   reprise repris de Vela, l'époque pour les lecteurs qui restent ouverts, les
   index dans leurs propres fichiers. Le produit — un backend parmi d'autres
   pour l'agent de code, par une session de synchronisation générique — vient
   après.

### Si une session ne revient pas

Tout ce qu'elle savait est dans son dossier : `produit/`,
`moteur-concurrence/`, `banc-de-concurrence/`, `optimiseur/`, `recherche/`.
Les clones de l'optimiseur étaient dans /tmp : ses sept branches se
retrouvent par `.vault/forks/pr-*.bundle` ou par les patches de
`extension/rag3weaver/docs/optimiseur/2-octobre-2026-00h15/patches/`.

## 7. Points de vigilance

- **La base MTG** (`experiments/mtga/data/…recovered.rag3db`) : ne pas
  l'ouvrir. Depuis la tolérance aux fins coupées, l'ouvrir avec les réglages
  par défaut pourrait écarter une partie de son journal (copiée à côté, donc
  rattrapable, mais on n'y touche pas sans Lucie).
- **Ne pas allumer le mode multi-écrivains** avant les corrections : cinq
  corruptions silencieuses prouvées.
- **rag3weaver continue après un point de reprise échoué** ; le moteur, dans
  ce cas, perd des clés puis plante. Tant que ce n'est pas corrigé : un commit
  qui échoue sur une erreur de point de reprise doit être traité comme une
  raison de rouvrir la base.
- **Un lecteur d'un autre processus peut lire faux pendant un point de
  reprise** jusqu'à la marche 1 ; le chemin sûr reste : un processus tient la
  base, les autres passent par lui.
- **Le banc de recherche** n'est dans aucune liste de livraison.
- **Les poids de l'OCR** (ppocrv6-tiny) manquent sur ce poste.

## 8. Mes erreurs de la nuit

- **`api_test` manquait à ma liste de livraison** : un rouge est resté sur
  master sans être vu, depuis une date que je n'ai pas établie. La liste est
  corrigée.
- **J'ai recommandé de supprimer et recréer les forks comme une purge
  certaine** ; c'était faux (les commits restent atteignables par le dépôt
  d'origine). Corrigé avant que Lucie ne supprime quoi que ce soit sur cette
  foi ; elle a ensuite choisi de les recréer en connaissance de cause.
- **J'ai présenté le périmètre de synchronisation par un exemple de code**, et
  proposé une entité « source » intégrée : c'était orienter un moteur qui doit
  rester générique. Retiré.
- **J'ai dit que les poids 0,6 / 0,4 n'avaient jamais été mesurés** ; ils
  avaient été choisis sur un cas réel le 27 août. Ce qui manque est la mesure
  au banc.
- **J'ai décrit PostgreSQL et Neo4j de mémoire** : c'est dit comme tel, et la
  session qui codera dessus doit le vérifier.
