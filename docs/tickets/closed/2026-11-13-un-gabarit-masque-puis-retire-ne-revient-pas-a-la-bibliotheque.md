# Un gabarit fourni masqué par un projet, puis démasqué, ne revient pas dans la bibliothèque

- **État** : corrigé le 13 novembre 2026 (session hotfixs) : le défaut était dans le test, pas dans
  le catalogue. Trouvé par la session tty, et reproduit sur master seul.
- **Famille** : produit
- **Gravité** : réponse fausse (une fiche de gabarit fournie manque au catalogue après une synchronisation).
- **Atteignable en service** : oui, par `sync_templates` d'un projet qui supprime un gabarit qu'il masquait.
- **Touche rag3weaver** : oui (le catalogue des gabarits).

## Ce que c'est

Un projet masque le gabarit fourni `entity/user` (un fichier `user.json` dans la couche du projet), puis supprime ce fichier. Après la synchronisation suivante, `sync_templates` rend bien `updated: ["entity/user"]`, mais la fiche `user` d'origine « builtin » n'est plus visible dans le catalogue. Elle devrait reprendre sa place, sous le même uuid.

## La recette

```bash
cd extension/rag3weaver
RAG3DB_LIB=rag3db-build-835d5298f80b832377a7caa80ee58d242e3ece23-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF \
CARGO_TARGET_DIR=<un target neuf> ./run_e2e.sh --features openai-llm,anthropic-llm --test e2e_catalogue_gabarits e2e_catalogue_gabarits
```

Rouge : 11 tests passent, 1 échoue, `la_synchronisation_alimente_la_couche_du_projet`, qui panique à `tests/e2e_catalogue_gabarits.rs:683` sur « user revenu à la bibliothèque » (étape 5 du test). La garde de `run_e2e.sh` relève aussi un résidu dans le dossier temporaire de la suite (`etat-temoins.*`, 4 Ko).

## La preuve qu'il ne vient pas d'un lot en cours

Le rouge est identique sur master seul, au commit `503a5ba2d`, avec la même lib et un target neuf (le worktree `/home/lucied/git_workspaces/cx-tty-temoin-master`, journal `~/.cache/rag3weaver-build/cx-tty-temoin-master.log`). On l'a d'abord vu sur la branche `cx-tty-foret`, qui n'ajoute que `templates/themes/foret.json` et ne touche aucun gabarit d'entité.

## Le témoin

Le test lui-même : `tests/e2e_catalogue_gabarits.rs`, `la_synchronisation_alimente_la_couche_du_projet`.

## La cause (trouvée le 13 novembre 2026, hotfixs)

Le défaut était dans le test, pas dans `sync_templates`. La fiche `user` revenait bien à la
bibliothèque. Mais `fiches_par_origine` cherchait « gabarit » avec une limite de 50 résultats, et
le test concluait à l'absence de tout ce qui tombait au-delà. Les outils de l'atelier, ajoutés à
partir du 9 novembre (76647d66d et suivants), ont fait passer la bibliothèque fournie au-delà de 50
fiches. Mesuré sans plafond sur la recherche du test :

| état | fiches fournies rendues | rang de `user` |
|---|---:|---:|
| c22975067~1 (9 nov., 13 h 25) | 49 | 47 |
| c22975067 (9 nov., 13 h 29) | 50 | 48 |
| master (13 nov.) | 65 | 63 (la 50e est `atelier_ask_permission`) |

Avec la limite portée à 200, la suite passe à 12 sur 12. Le résidu `etat-temoins.*` du dossier
temporaire n'est pas un second défaut : il disparaît avec le vert. C'est la trace laissée par le
panic.

**Le correctif** (`tests/e2e_catalogue_gabarits.rs`) ne relâche pas le plafond. Une présence se lit
par le nom de la fiche : la nouvelle aide `fiche_nommee` passe par `Catalog::find_by_field`, sans
recherche. Les étapes 2 et 5 l'utilisent pour `user` et `product`. `fiches_par_origine` ne sert plus
qu'aux quelques fiches du projet, et elle échoue si elle atteint son plafond, au lieu de rendre une
liste tronquée.

**Les preuves**, sur e93710f36 avec la lib bâtie sur ce commit :
- l'ancien test est rouge, 11 sur 12 ;
- le test lu par nom est vert, 12 sur 12, sans résidu ;
- avec l'étape 5 sabotée (`user.json` laissé dans le projet, `r7` non vérifié), la nouvelle
  assertion rougit sur `left: [("project", …)]` contre `right: [("builtin", …)]`.

Le reste de cette section est ce que le ticket disait à son ouverture.

Pistes notées à l'ouverture : Pistes à lire :
- la reprise de la fiche fournie quand le fichier du projet disparaît (`sync_templates`) ;
- l'origine écrite sur la fiche ;
- `fiches_par_origine` côté test, peut-être après un changement récent du catalogue.

L'historique récent du dépôt montre plusieurs lots du 13 nov. sur les stockages et le journal (`d55b5264c`, `fd32269d8`). Rien n'est encore départagé.

## Ce qu'il faut pour le fermer

Le test vert sur master, puis la suite sans résidu dans son dossier temporaire, ou bien inscrite à `tests/e2e/suites_qui_laissent_des_residus.txt` si ce résidu est voulu.
