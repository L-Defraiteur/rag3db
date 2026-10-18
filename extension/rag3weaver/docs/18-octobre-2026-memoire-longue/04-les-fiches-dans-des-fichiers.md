# Les fiches dans des fichiers, la base comme index reconstructible

*Étude légère du 18 octobre 2026 (mémoire longue), sur une idée de Lucie relayée par
l'orchestration. Son inquiétude : « il ne faut pas qu'on prenne l'habitude, et qu'au
dernier moment on dise : l'ancien index est corrompu, on doit reconstruire ». La réponse
qu'esquisse cette page : **la reconstruction ne doit jamais être un geste d'urgence, mais
un chemin qui tourne tout le temps**. Rien n'est codé ; la forme du Plan et `cited_by`
passent avant.*

## 0. Ce qui existe déjà

- **Un lecteur de dossier générique** (`FileSource`, `src/code_tools.rs:46`, arbre de
  travail ou instantané), mais **les fichiers ne deviennent des entités que par le schéma
  de code** : `workspace.index` n'accepte que `"code"` (`backend_code.rs:55-63` annonce
  « un dossier de documents nommera le sien », rien n'est écrit). Aucun gabarit hors code
  n'ingère un dossier de Markdown ; **aucun analyseur d'en-tête** (ni YAML ni TOML dans
  `Cargo.toml`).
- **Le rendu jinja** : `RenderResultsNode` (`render_nodes.rs:871`, modèles
  `templates/render/*.md.jinja`), les vues des routes, le `content` des entités dérivées.
- **Un seul nœud qui écrit un fichier**, `EditFileNode` (`code_nodes.rs:390`) : chemin
  relatif contrôlé (pas d'absolu, pas de `..`, pas de lien qui sort), écriture atomique,
  `policy.write_files` ; mais il réingère dans le schéma de code et ne prend son contenu
  que par paramètre (aucune entrée de port).
- **La montre** (`src/watch.rs`) : par sondage, sur un `workspace` en `working_tree`, avec
  une réconciliation au montage — **liée au schéma de code** (`MATCH (f:File)…`,
  `ReindexFileNode`).
- **La concurrence optimiste côté base** : `expected_revision` d'`EntityRecordNode`
  (`backend_nodes.rs:183-189`), et `Memory` déclare `writes.revision`. **`expected_hash`
  n'existe pas** : il est proposé pour `edit_file` (page 05 du proto, lot B1), et
  `EditResult` rend déjà le `content_hash` à comparer.
- **La preuve de parité, pour le code** : `tests/e2e_code_sync.rs` (`etat()`, `a_neuf()`,
  « une édition après un point de reprise vaut un index neuf »). Rien pour une entité
  déclarée.
- **Le classeur L1/L2/L3** : vision seulement
  (`visions/2026-10-04-00h09-memoires-par-gabarit-et-fiches-qui-pointent-vers-tout.md`
  §9.2-9.3) ; aucun code.
- **L'identité** : `origin.rs` sait le dépôt et la révision d'un fichier ; une session de
  l'hôte porte `client` et `keys` (`daemon_host.rs:62-78`, « gardées et dites… pas encore
  appliquées ») ; le jeton local est le même pour tous les clients ; **aucun champ auteur**
  sur `Memory` (`reach` dit la portée, pas qui).

## 1. La forme

```text
<dossier de la mémoire>/
  fiches/<uuid>.md            une fiche = un fichier ; le nom ne bouge jamais
  classeur/<sujet>/sujet.md   le sujet (name, about, state)
  classeur/<sujet>/L1/<date>-<uuid>.md, L2.md, L3.md   les résumés produits par un LLM
```

- **Les fiches à plat, par uuid**, pas sous leur sujet : une fiche a zéro, un ou plusieurs
  sujets, et son sujet change ; un fichier qui déménage à chaque rangement ferait du bruit
  dans git et des conflits de renommage. Le classeur la cite par son uuid.
- **L'en-tête** porte ce qui est indexé : `uuid`, `claim`, `kind`, `origin`, `author`
  (§5), `reach`, `state`, `revision`, `created_at`, `updated_at`, `reviewed_at`, `refs`
  (`[{genre, valeur, dit_comme}]`, en forme normale), `subjects` (les noms), `replaces`
  (l'uuid remplacé, s'il y en a). **Le corps** est le `why`, en Markdown libre. Les
  empreintes, états de Ref et vecteurs ne sont **pas** dans le fichier : ils se recalculent.
- **Le format de l'en-tête** est un choix de dépendance, donc à Lucie : YAML (le plus
  courant dans les éditeurs, mais une crate à prendre) ou TOML (`+++`, une crate petite et
  sûre). Je propose TOML : un format sans ambiguïté de types, ce qui compte pour une
  parité champ à champ.
- **Les L1/L2/L3 en fichiers**, comme le demande Lucie : ils coûtent un appel de modèle
  et ne se refont pas à l'identique ; un index reconstruit les **relit**, ne les
  **régénère** jamais.

## 2. La double sortie

**Le fichier d'abord, la base ensuite** : si la base est l'index, le fichier est la vérité.

- Une **déclaration générique sur l'entité**, pas un cas pour la mémoire (« une
  organisation nouvelle se décrit dans EntityConfig ») : `"files": {"dir": "fiches",
  "name": "{uuid}.md", "template": "fiche.md.jinja", "header": [champs…], "body": "why"}`.
- Chaque écriture par `EntityRecordNode` d'une entité qui déclare `files` : rendre le
  fichier (jinja), l'écrire atomiquement (le même garde de chemin qu'`EditFileNode`), puis
  ingérer la ligne. Une ingestion qui échoue laisse le fichier : l'index est en retard, et
  la réconciliation du §4 le rattrape. Une écriture de fichier qui échoue n'écrit rien en
  base.
- Les relations (`CITE`, `ANCHORED_TO`, `REPLACES`) s'écrivent dans l'en-tête de la fiche
  qui les porte ; `add_ref` et `anchor` réécrivent donc le fichier de la fiche.

## 3. La reconstruction, et sa preuve de parité

- **`rebuild`** : une base neuve, le dossier relu, chaque fichier posé par le même chemin
  que `seed` (`EntityRecordNode`), en mode **restauration** : les dates et la révision
  viennent du fichier (aujourd'hui `EntityRecordNode` refuse les champs que le serveur
  pose ; la restauration est la seule exception, et elle n'est pas un outil).
- **La preuve** : un témoin qui écrit par les outils (fiches, Refs, sujets, ancrages, un
  remplacement), puis compare la base vivante et la base reconstruite **champ à champ** —
  les lignes, leurs champs déclarés, les relations avec leur multiplicité (le patron
  d'`e2e_code_sync`, `etat()` contre `a_neuf()`). Les vecteurs se recalculent : leur
  comparaison est celle du service d'embarquement (écart absolu borné), pas l'égalité.
- **Contre l'inquiétude de Lucie** : la reconstruction tourne **tout le temps**, pas au
  dernier moment.
  - Le témoin de parité est dans la batterie.
  - L'hôte compare au démarrage le dossier et la base (empreinte de chaque fichier
    contre `_content_hash` de sa ligne), et `index_state` dit « dossier ≠ base : N
    écarts » au lieu de se taire.
  - Une commande `memoire verifier` fait la reconstruction à blanc dans une base
    temporaire et rend les écarts.

  Un index qu'on reconstruit chaque semaine sans le remarquer ne se découvre pas
  corrompu le jour où il compte.

## 4. L'édition à la main, et l'agent qui écrit en même temps

- **La montre sur le dossier** : la montre d'aujourd'hui est liée au schéma de code ;
  il faut la même sur un dossier d'entités déclarées (`files`), avec une réaction
  générique « un fichier a changé → relire son en-tête → poser la ligne ». La
  réconciliation au montage compare les empreintes, comme pour le code.
- **Un fichier invalide** (un en-tête qui ne se lit pas, un champ hors schéma, des
  marqueurs de conflit git) : la ligne indexée reste celle d'avant, le fichier n'est
  **jamais réécrit** par l'hôte, et `index_state` dit le fichier et la raison.
- **L'agent et l'humain sur la même fiche** : l'`expected_hash` du proto, côté fichier.
  `get_memory` rend l'empreinte du fichier lu ; `put_memory` la reprend ; si le fichier a
  changé depuis (l'humain l'a édité, la montre l'a déjà réindexé), l'écriture est refusée,
  « la fiche a changé depuis ta lecture », et rien n'est écrit — l'agent relit, puis
  réécrit. L'`expected_revision` de la base reste, en second filet.

## 5. Le partage par git, et l'auteur d'une fiche

**Le dossier est un dépôt git** (ou un dossier d'un dépôt). Deux postes, puis une équipe :
un `pull` change des fichiers, la montre réindexe.

- **Les conflits** : une fiche par fichier, donc un conflit seulement quand la même fiche
  a changé des deux côtés. Les marqueurs de conflit rendent le fichier invalide (§4) :
  l'index garde la dernière version bonne et dit le conflit. Les L1/L2/L3 en conflit se
  résolvent en prenant l'un ou l'autre, marqué « à refaire ». Deux postes qui créent la
  même fiche (même `claim`, même `reach`, donc même uuid, `hashsafe`) tombent sur le même
  nom de fichier : un conflit d'ajout que git dit.
- **L'auteur** (précision de Lucie : pas viser une personne, mais dire **de qui vient la
  fiche**, la personne pour qui l'agent travaillait) : un champ `author` sur `Memory`, de
  la provenance, à côté d'`origin`. **Posé par l'hôte, jamais par l'outil** : un champ que
  le serveur pose, comme `created_at`, qu'`EntityRecordNode` refuse s'il est donné par
  l'appel — l'agent ne peut pas se déclarer quelqu'un d'autre.
- **D'où vient l'identité**, sans que l'agent la dise :
  - **la clé de session MCP** (`--keys`, que l'hôte reçoit déjà dans `POST /session`) :
    écrite dans le `.mcp.json` du poste, donc falsifiable par qui édite ce fichier, pas par
    l'agent ; elle dit **quel client** (Claude Code, l'explorateur) ;
  - **l'identité git du dossier de la mémoire** (`git config user.email`, lue **dans ce
    dépôt**, pas en global : sur luciepc le global est une adresse pro, interdite ici) :
    elle dit **quelle personne**, et c'est aussi l'auteur du commit ;
  - **les deux** : `author` = l'identité git du dépôt de la mémoire ; `via` = le client de
    la session. Le commit git les recoupe : un `author` qui ne serait pas l'auteur du
    commit qui a posé la fiche se voit.
  
  Ni l'une ni l'autre n'est une preuve cryptographique : elles empêchent l'agent de
  mentir, pas la personne. Une signature de commit (`git commit -S`) serait l'étape
  suivante, si une équipe le demande.
- **Filtrer et trier par auteur** : `cited_by` gagne `authors?` (un `optional` `in` sur
  `Memory.author`, dans la forme fixée) et un tri possible par auteur puis date ; le
  classeur cite ses fiches avec leur auteur, et une vue du sujet se filtre par auteur.

## 6. Les lots, en ordre de grandeur relatif

| lot | contenu | taille relative |
|---|---|---|
| F1 | la déclaration `files` sur une entité, le rendu et l'écriture du fichier avant la ligne, l'en-tête (la crate à choisir) | moyen |
| F2 | `rebuild` en mode restauration, et **le témoin de parité champ à champ** | moyen |
| F3 | la montre sur un dossier d'entités déclarées, la réconciliation au montage, les fichiers invalides dits | moyen à gros (la montre est liée au code aujourd'hui) |
| F4 | `expected_hash` sur l'écriture d'une ligne miroir de fichier | petit |
| F5 | `author` posé par l'hôte (identité git du dépôt + client de la session), filtrable dans `cited_by` | petit à moyen |
| F6 | le partage : deux clones, un conflit, une fiche créée des deux côtés (témoins) | moyen |
| F7 | les L1/L2/L3 en fichiers | dépend du classeur, qui n'existe pas encore |

L'ordre : F1 puis F2 d'abord (sans la preuve de parité, le reste repose sur une promesse) ;
F4 et F5 ensuite ; F3 ; F6 ; F7 avec le classeur. Le format de l'en-tête (YAML ou TOML)
attend le mot de Lucie avant F1.
