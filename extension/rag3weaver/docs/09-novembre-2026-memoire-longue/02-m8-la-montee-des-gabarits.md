# M8 — la montée des gabarits d'un backend, sur le gabarit vivant

*Page de conception, 9 novembre 2026, session mémoire (96). Pour everything declarative (ED), qui
a posé le gabarit vivant, et pour l'orchestration. Rien ne se code avant la relecture d'ED. Ticket
d'origine : `docs/memoire/tickets/2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md`.*


**Révision du 12 novembre 2026, après relecture ED.** Voir la
[note sourcée](../../../../docs/memoire/12-11-2026/01-relecture-m8-adoption-fusion-et-reprise.md).
Le document décrit une conception, pas une commande livrée. Les garanties d'adoption, de fusion
avec conflits et de maintenance/reprise restent à spécifier et prouver. Le précontrôle 1b est
diagnostique et ne fournit pas un apply. Le ticket reste ouvert ; aucun lot de code M8 n'est ouvert.

## 1. Ce qui existe déjà (ED, pile de l'atelier, f16628f8b)

Lu dans `src/builtin_templates.rs` :

- **Les gabarits livrés par leur nom** : le binaire embarque `templates/`, une valeur
  `builtin:<chemin>` d'un manifeste désigne le gabarit livré et suit chaque bâti ; `templates
  link` remplace par `builtin:` les copies identiques au livré. Une copie qui diffère se dit à
  l'ouverture (`template_drift`, visible dans `/ide/etat`).
- **Le gabarit vivant** : à la pose, `record` écrit `livre.json` hors de l'arbre des déclarations
  (`~/.local/state/rag3weaver/manifestes/<empreinte du chemin>/`) : le nom du gabarit livré, **son
  contenu d'alors**, et l'empreinte du manifeste tel qu'écrit. À chaque ouverture, `refresh` :
  - le manifeste n'a pas bougé depuis la pose : il prend le gabarit du binaire, plus ce que la
    pose avait réglé — **une fusion à trois sur les feuilles JSON**, la base étant le contenu
    d'alors ; il rend les outils venus et partis (`TemplateRefreshed` sur le flux) ;
  - modifié à la main : gardé, `EditedByHand` ;
  - suivi par git (le dépôt des déclarations d'un arbre inscrit) : gardé, `TrackedByGit`.
- **Politique de revue** (rapport du 7 novembre ; identité tranchée par c7 le 13 novembre) : un arbre dont les déclarations sont
  suivies par git se remet au gabarit livré **par un commit à l'identité neutre `rag3weaver <rag3weaver@localhost>` (ou `workspace.git_identity` si le manifeste la déclare)**, fichiers propres
  seulement, sinon refus dit ; lot à part chez ED.
- **Deux défauts relevés ce matin** par ED : à l'installeur, « déjà posé » ne note ni ne
  rafraîchit, et une pose neuve n'écrit pas `livre.json`.

## 2. Ce qui manque, vu de la mémoire de Lucie

La mémoire de Lucie (`~/.config/rag3weaver/memoire-longue/`) est une copie de `templates/` du
25 octobre au soir, faite à la main : **pas de `livre.json`** (`NotRecorded`), pas de dépôt git, et
des copies locales des graphes, scripts et schémas, pas des `builtin:`. `refresh` n'y fait donc
rien, et ne ferait rien non plus si elle avait un `livre.json`, puisqu'il a été retouché depuis
(`EditedByHand`). `whois`, `select_sources`, `select_apports` et `archive` n'y sont pas.

Trois limites du gabarit vivant, pour ce cas :

1. **Il ne monte que le manifeste.** Les fichiers copiés à côté (un graphe, un script Rhai, un
   schéma d'entité) n'ont pas de base : `livre.json` ne garde que le JSON du manifeste.
2. **Il ne fusionne que l'intouché.** Une seule retouche à la main (`EditedByHand`) arrête tout,
   alors que la fusion des feuilles saurait garder la retouche et prendre le reste.
3. **Il ne sait rien des données.** Un schéma d'entité qui change (un champ requis, un type) ne
   se lit plus sur les lignes déjà en base ; rien ne le dit avant l'ouverture.

## 3. Ce que M8 ajoute

**Une commande, explicite, une mémoire à la fois** : `rag3weaver-backend templates upgrade
<manifeste> [--dry-run]`. La montée M8 envisagée est explicite. Le suivi vivant actuel reste distinct : `refresh` peut
réécrire un manifeste éligible à l'ouverture et `builtin:` suit le binaire. La coexistence avec une
épingle réellement stable reste à trancher ; l'ouverture n'est donc pas décrite comme purement
diagnostique. Cette page n'autorise pas à désactiver implicitement le suivi vivant.

1. **L'adoption d'une copie d'avant le gabarit vivant** : `templates adopt <manifeste> --from
   <commit>` écrit le `livre.json` qui manque, la base étant `templates/` au commit donné, lu dans
   l'historique git de rag3db (pour Lucie : le master du 25 octobre au soir). C'est ce qui rend la
   fusion possible sur une copie faite à la main.
2. **La base étendue aux fichiers copiés** : `livre.json` garde, pour chaque fichier local que le
   manifeste cite, son nom livré et l'empreinte de son contenu d'alors ; le contenu lui-même se
   relit dans l'historique git au commit de la base (pas de copie de plus sur le disque).
3. **La fusion à trois au-delà de l'intouché** :
   - le manifeste : la fusion des feuilles de `refresh`, mais une feuille changée des deux côtés,
     différemment, est un **conflit** ;
   - un fichier copié : une copie restée identique à sa base devient `builtin:` (elle suit les
     bâtis, comme `link`) ; une copie retouchée se fusionne par `git merge-file` (base, la
     copie, le livré neuf) ;
   - **un conflit refuse toute la montée**, avant d'écrire quoi que ce soit, et se nomme : le
     fichier, la feuille ou les lignes, les deux valeurs.
4. **Le diff des outils et leurs appelants**, avant la montée : les outils venus, partis,
   renommés (un outil parti dont le graphe revient sous un autre nom), et pour chaque outil qui
   part, ses appelants connus : les crochets de `hooks.json`, et les clients qui l'ont appelé dans
   les runs des trente derniers jours (le journal des runs nomme l'outil et la session). Un outil
   parti qui a des appelants est dit, et demande `--accept-removed <outil>`.
5. **La migration de schéma** (page 05 de la mémoire longue, la migration de la fiche) : un
   schéma d'entité qui change est comparé au schéma en base. Un ajout facultatif passe ; un champ
   requis nouveau, retiré ou retypé exige une migration déclarée dans le gabarit neuf, jouée sous
   instantané par le chemin de la page 05, sinon refus nommé.
6. **Le geste entier, sous l'hôte** : `--dry-run` dit tout cela sans rien écrire. Sinon :
   fermer le backend par l'hôte (`POST /backend/close`), instantané de la base et des
   déclarations, écrire la fusion, nouveau `livre.json`, migration s'il y en a, rouvrir
   (`/backend/open`), vérifier (`index_state`, `verify_files`, chaque outil neuf décrit), et une
   ligne dans la page du jour. Un échec après l'instantané le rend, et le dit.

## 4. Ce que M8 réutilise, sans second mécanisme

- `livre.json` et son dossier d'état, `record`, la fusion des feuilles de `refresh`, `link` et
  `builtin:` : M8 les étend, il ne les double pas.
- **Les déclarations suivies par git** : la montée est **un commit à l'identité neutre `rag3weaver <rag3weaver@localhost>` (ou `workspace.git_identity` si le manifeste la déclare)**, qu'ED a
  tranché, sur une branche `gabarits/<version>` du dépôt des déclarations, fusionnée par git
  (git fait la fusion à trois et nomme les conflits) ; refus si l'arbre n'est pas propre.
- **L'épingle envisagée** doit identifier le paquet de contenus, avec le commit Git en provenance.
  Le contenu de base de `livre.json` et les liens `builtin:` actuels ne suffisent pas à figer les
  dépendances. La forme de l'épingle et son diagnostic de retard restent à trancher ; un nombre
  de commits n'est pas une mesure universelle entre branches ou binaires distribués sans Git.
- La fermeture et la réouverture par l'hôte, l'instantané et la migration de la page 05.

## 5. Le témoin

Une mémoire posée depuis un gabarit d'une version A, avec une retouche à la main (une
instruction réécrite, un outil ajouté), puis montée en version B :

- la retouche est gardée, les outils de B sont servis, la base est intacte champ à champ ;
- la même retouche faite aussi dans B, autrement : refus nommé, rien d'écrit ;
- un outil retiré dans B et appelé par un crochet : refus sans `--accept-removed` ;
- un champ requis ajouté dans B sans migration : refus ; avec, les lignes migrées sous
  instantané ;
- une copie sans `livre.json`, adoptée au commit de A, puis montée : même résultat.

## 6. Pour Lucie, une fois sur master

`templates adopt` sur sa mémoire au commit du 25 octobre, puis `templates upgrade --dry-run`,
dont le rapport lui est montré ; la montée à son mot, sur une copie d'abord.

## 7. Questions pour ED

1. Le contenu d'alors des fichiers copiés : dans l'historique git de rag3db (léger, mais exige le
   dépôt sur le poste) ou dans `livre.json` (autonome, plus lourd) ?
2. Le commit des déclarations suivies, à l'identité neutre : la même branche pour le rafraîchissement de
   `refresh` et pour M8, ou M8 seul ?
3. Vu à côté : `~/.local/state/rag3weaver/manifestes/` porte 428 `livre.json`, dont 400 de
   `backends/code` écrits depuis le 8 novembre à 21 h 43. Ils viennent sans doute des tests, qui
   écriraient dans le dossier d'état réel faute de `RAG3WEAVER_STATE_DIR`. À vérifier avant
   qu'il grossisse. **Actualisation du 12 novembre** : le code possède désormais une isolation
   des témoins (`state_root`, fixtures e2e) ; aucun nouvel inventaire du dossier réel n'a été fait
   dans cette relecture.

## 8. Limites précisées par la relecture du 12 novembre

Les étapes ci-dessus sont des objectifs. `--from` doit identifier une base exacte et récupérable ;
une date supposée ou une copie ressemblante ne suffisent pas. La fusion actuelle de `refresh` ne
détecte pas les conflits M8. La proposition minimale est un plan à blanc avec fusion pure,
validation en recouvrement et refus structurés, sans écrire de livre.

La version du paquet de gabarits ne se confond pas avec le schéma, les formats ou la génération
de données. `builtin:` ne suffit pas à épingler le contenu. Le stockage des bases et la politique
vivant/épinglé restent des choix ouverts.

`POST /backend/close` n'empêche pas un autre appel de rouvrir le backend. Le geste entier attend
une exclusion, un instantané vérifié et une reprise durable relevant des contrats communs du
système, sans moteur de migration M8 parallèle. La recette historique de la page 05 ne prouve
pas ces garanties. Un ajout facultatif ou un undo ne prouvent pas à eux seuls la compatibilité
ou la restauration multi-ressources. Le détail des preuves et des témoins proposés figure dans
la note datée. L'identité Git d'un commit de gabarits est l'identité neutre `rag3weaver <rag3weaver@localhost>`,
passée par `-c user.name -c user.email`, ou `workspace.git_identity` si le manifeste la déclare
(`crate::backend_code::git_identity_or_default`) : jamais l'identité d'une personne, et jamais la
configuration globale du poste, qui porte l'adresse professionnelle de Lucie (décision du 9 novembre
de la session hooks-essai, 2d6e58d1a ; rappelée par everything declarative et tranchée par c7 le
13 novembre). L'identité Gmail de Lucie reste celle des commits que les sessions écrivent pour elle.
