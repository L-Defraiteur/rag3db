# Ce qu'on n'indexe pas : les règles d'exclusion, mesurées

14 novembre 2026, recherche, à la demande de c7. **Page de décision pour Lucie : aucun code avant son
mot.** Elle répond au ticket `docs/lucivy/tickets/2026-11-13-le-plein-texte-du-depot-pese-cent-fois-ses-sources.md`
et s'appuie sur la mesure de lucivy (`docs/lucivy/14-11-2026/01-ce-que-pese-lucivy-scope-document-par-document.md`).

## En bref

- **Le dépôt admis aujourd'hui pèse 67 Mo** (7 866 fichiers) sur l'arbre de Lucie, et **63 Mo** (7 278
  fichiers) sur code-lourd. Le plein texte en indexe **928 Mo**. Ce n'est pas l'arbre qui est gros.
- **Presque tout l'écart vient d'un seul fichier** : `extension/rag3weaver/templates/ide/app/app.js`, le
  bundle minifié de l'IDE (502 Ko, 85 lignes, dont une de 133 294 octets). Il est suivi par git, et il a
  été commité en **52 versions** depuis le 25 octobre.
  - Chaque fonction de la ligne devient un scope, et un scope reçoit les lignes qu'il couvre. Les
    centaines de fonctions de la ligne de 133 Ko reçoivent donc chacune la ligne entière.
  - C'est exactement le morceau de 133 Ko que lucivy compte jusqu'à 484 fois. Sa première ligne,
    `var _0=Object.create;var sd=…`, est aussi celle de l'un de ses groupes de doublons.
- **Deux remèdes, indépendants** :
  - borner le contenu d'un scope à ses octets, que **ED prend** (Rust) ;
  - ne pas indexer un minifié, la règle 1 ci-dessous.
  Chacun suffit à faire tomber l'essentiel des 856 Mo.
- **Les règles par fichier retirent peu de l'arbre actuel** : au plus un quart de ses 63 à 67 Mo, et
  surtout les dépendances importées. Leurs faux positifs sont réels : ils sont cités règle par règle.
- **Les copies de fichiers ne sont pas la cause des doublons** : 0,2 à 0,3 % des octets admis. Les 844 Mo
  de doublons exacts de lucivy sont des scopes d'une même ligne de bundle (voir le § « Les doublons »).

## Comment c'est mesuré

`mesure.py` (hors du dépôt, joué en lecture seule sur les deux arbres) reproduit ce que rag3weaver fait
aujourd'hui :

- **La marche** : `rg --files --hidden`, c'est-à-dire le crate `ignore` de rag3weaver, avec `.gitignore`,
  `.ignore` et l'exclusion globale, cachés compris, `.git` jamais. S'y ajoutent `SKIPPED_DIRS` (`target`,
  `node_modules`, `dist`, `build`, `.venv`, `venv`, `__pycache__`).
- **La porte d'admission** (`code::admission`) :
  - les motifs de nom (`.min.js`, `.min.css`, `-lock.json`, `.generated.`) ;
  - les extensions écartées ;
  - « sans extension » ;
  - le texte sans grammaire au-delà de 128 Kio ;
  - le binaire (un octet nul dans les 8 000 premiers) ;
  - le dossier `generated` et les marqueurs de généré dans les 2 048 premiers octets.
- **Puis chaque règle candidate, seule**, sur ce qui est admis : les fichiers, les octets, la part des
  octets admis, et les plus gros fichiers retirés, où se lisent les faux positifs.

Les deux arbres :

- **code-rag3db-depot** : l'arbre de Lucie, `~/git_workspaces/rag3db` (master du jour) ;
- **code-lourd** : à luciepc, `~/git_workspaces/rag3db-lourd`, figé au 11 octobre (a72acfeef).

**Ce que la mesure ne voit pas** : le texte par scope. Lucivy_Scope n'a pas de chemin, la jointure passe
par le graphe. La mesure est donc par fichier, et c'est le texte des fichiers qu'elle compte, pas celui
que les scopes répètent.

## Ce que la porte écarte déjà

| raison | arbre de Lucie | code-lourd |
|---|---|---|
| artefact ou données (extension) | 541 fichiers, 349 Mo | 498 fichiers, 384 Mo |
| texte sans grammaire > 128 Kio | 34 fichiers, 27,7 Mo | 31 fichiers, 124 Mo |
| sans extension | 91 fichiers, 12,1 Mo | 78 fichiers, 17,6 Mo |
| généré (marqueur ou dossier) | 31 fichiers, 4,1 Mo | 34 fichiers, 4,2 Mo |
| binaire | 108 fichiers, 0,4 Mo | 122 fichiers, 0,4 Mo |
| `.min.js`, `.min.css`, `-lock.json`, `.generated.` | 6 fichiers, 0,3 Mo | 5 fichiers, 0,1 Mo |

**Ce qu'elle laisse passer** : un fichier dont l'extension a une grammaire (`.c`, `.h`, `.cpp`, `.js`,
`.ts`, `.py`, `.rs`…) n'a **aucune taille maximale**, et un minifié qui ne s'appelle pas `.min.js` entre
comme du code.

## Les règles candidates

Chaque règle est mesurée seule, sur ce qui est admis aujourd'hui. Parts en octets admis.

| règle | arbre de Lucie | code-lourd | faux positif cité |
|---|---|---|---|
| **1. minifié par le contenu** : une ligne de plus de 5 000 octets | 9 fichiers, 0,8 Mo (1,2 %) | 7 fichiers, 0,3 Mo (0,6 %) | les fichiers `.test` de sqllogictest (`extension/vector/test/test_files/demo.test`, 88 Ko), des données de test, pas du code |
| 1 bis. moyenne de plus de 300 octets par ligne | 19 fichiers, 0,8 Mo | 16 fichiers, 0,3 Mo | les mêmes |
| **2. dossier d'import** : `vendor`, `third_party`, `external`, `extern`, `deps` dans le chemin | 886 fichiers, 15,3 Mo (22,9 %) | 886 fichiers, 15,3 Mo (24,4 %) | `extension/rag3weaver/codeparsers/vendor/tree-sitter-*` (grammaires générées : un vrai positif) ; mais `third_party/` contient du code que le moteur appelle (nlohmann_json, utf8proc, brotli), où « où est définie cette fonction » a une réponse |
| 3. dans un sous-module | 183 fichiers, 2,7 Mo | 1 298 fichiers, 14,6 Mo | **le code de codeparsers et de lucivy lui-même** (`codeparsers/src/scope_extraction/base_scope_extraction_parser.rs`, `ld-lucivy/lucivy_core/src/sharded_handle.rs`) : à ne pas retenir |
| 4. hors git (non suivi, non ignoré) | 2 fichiers, 0 Mo | 1 119 fichiers, 12,0 Mo | sur code-lourd, ce sont **les sources de lucivy** (le sous-module `ld-lucivy` que `git ls-files --recurse-submodules` ne voit pas là) : à ne pas retenir telle quelle |
| 5a. code > 128 Kio | 31 fichiers, 10,8 Mo | 26 fichiers, 9,2 Mo | `extension/rag3weaver/src/catalog.rs` (671 Ko, écrit à la main), `backend.rs` |
| 5b. code > 512 Kio | 4 fichiers, 4,3 Mo | 4 fichiers, 4,2 Mo | `catalog.rs` encore |
| **5c. code > 1 Mio** | 1 fichier, 1,9 Mo (2,9 %) | 1 fichier, 1,9 Mo (3,1 %) | aucun : `third_party/utf8proc/include/utf8proc_data.h`, une table Unicode générée |
| « `.min.` » dans le nom, toute extension | 0 | 0 | — |

### Ce qu'on lit dans ce tableau

- **La règle 1 est celle qui compte**, bien qu'elle ne retire que 0,8 Mo de l'arbre. Le fichier qu'elle
  retire est `app.js`, celui qui donne 856 Mo de texte à l'index. Ses faux positifs (les `.test`) sont
  des données de test, que l'on peut retirer aussi ou garder par une exception nommée.
- **La règle 2 est un choix, pas une évidence** : 15 Mo (un quart de l'arbre), mais du code que le moteur
  lie. La question pour Lucie : un agent doit-il trouver `nlohmann::json::parse` dans l'index ?
- **Les règles 3 et 4, telles quelles, retirent nos propres sources.** Le dépôt est fait de sous-modules à
  nous (codeparsers, lucivy). « Hors git » est déjà couvert : le `.gitignore` est respecté par la marche.
- **Une taille maximale de code tient à 1 Mio, pas au p99.** Le p99 des fichiers admis est de 80 à 84 Ko,
  mais des sources écrites à la main dépassent 512 Ko (`catalog.rs`, 671 Ko). À 1 Mio, il ne reste
  qu'une table générée.

## Les doublons

- **Les copies exactes de fichiers** : 83 fichiers, 0,1 Mo (0,2 %) sur l'arbre de Lucie ; 70 fichiers,
  0,2 Mo (0,3 %) sur code-lourd. Ce sont des docs recopiées entre deux dossiers de session et deux
  `lucivy.js` de `pkg/`. **Ils n'expliquent pas les 21 % de doublons de lucivy.**
- **Les 844 Mo de doublons exacts sont des scopes d'une même ligne de bundle** : les fonctions d'une ligne
  de 133 Ko reçoivent chacune la ligne entière. C'est le correctif d'ED qui les fait tomber, pas une
  règle d'exclusion.
- **Un fait de plus, à vérifier par jointure avec le graphe** : la base du backend `rag3db` déclare
  68 585 scopes et autant de documents plein texte (`daemon_status`, `fts_counts`). La copie de lucivy
  compte 89 894 documents vivants, et les `_node_id` de ses groupes de doublons de bundle sont tous
  au-dessus de 69 000 (69 693 à 94 834). Le bundle a 52 versions commitées. **Il se peut que les versions
  passées d'`app.js` restent dans le `.fts`** après le remplacement de leurs scopes. Si c'est le cas, une
  règle d'exclusion n'y suffit pas : il faut aussi une purge. C'est à lucivy ou à ED de le départager,
  par le `_node_id` contre les lignes `Scope` du graphe.

## Ce que je propose, à trancher par Lucie

1. **Ne pas indexer un minifié, reconnu par son contenu** : une ligne de plus de 5 000 octets dans un
   fichier de code. La règle retire `app.js`, les `lucivy.js` de `pkg/`, et des `.test` de données. Elle
   se dit dans `skipped`, comme les autres, avec la raison « minifié ».
2. **Une taille maximale pour le code : 1 Mio**, sous la même raison que le texte (« trop gros pour être
   écrit à la main »). Aujourd'hui, elle ne retire que `utf8proc_data.h`.
3. **Les dossiers d'import : à toi.** Soit les retirer (15 Mo, un quart de l'arbre), soit les garder. Une
   voie entre les deux : les garder sans vecteurs (le plein texte seul), puisque c'est là qu'on cherche
   un nom exact. Ce serait une option de manifeste.
4. **Ne pas retenir** « dans un sous-module » ni « hors git » tels quels : ils retirent nos propres sources.
5. Avec le correctif d'ED (le contenu d'un scope borné à ses octets), **reconstruire le plein texte de la
   base du dépôt, à ton mot**. La purge des documents d'anciennes versions, si elle se confirme, vient
   avec.

## Les témoins, quand le code viendra (rouges d'abord)

- Un `.js` d'une ligne de 10 000 octets n'est pas indexé, et l'estimation le dit « minifié ».
- Un `.c` de 1,5 Mio n'est pas indexé ; un `.rs` de 700 Kio l'est.
- Un fichier sous `third_party/` suit l'option du manifeste.
