# M4, l'archiviste — le design avant le code

Le lot M4 de la page 04 du 18 octobre (§2, §3, §8) : « `archive(document)`, le L0 gardé, le
découpage, la classification fermée (la tentative 4, plus « créer »), l'écriture du L1 avec sa
Ref `doc:` ». Écrit la nuit du 25 au 26 octobre 2026, après la migration de la fiche de Lucie
(page 05) : la mémoire réelle est sur le gabarit livré, l'archiviste peut y écrire.

## 1. Ce qui existe, et ce qui manque

Relevé sur master (b9cbfb774).

- **Toute la séquence d'écriture existe**, câblée deux fois en Rust avec une fiche choisie
  d'avance : le juge (`juge.rs`, `ecrire_le_bilan`) et l'équipier (`teammate.rs`,
  `file_closed_plan`) font `put_source` → `put_fiche` au besoin → `put_apport` →
  `relier_apport` → `create_ref_type doc` → `citer_source`. Les outils sont ceux du gabarit.
- **La décision fermée existe** : `Decide(via='llm', labels_from=…, labels=['aucun'])`, un
  schéma strict à enum, température 0, une réponse tronquée dite, une étiquette hors liste
  refusée. La tentative 4 l'a mesurée : 36 fiches justes sur 39, aucun faux rangement, par
  gemini-3.8-flash sur Vertex. L'étiquette « créer » est renvoyée à une tentative 5.
- **Ce qui manque** :
  1. **un modèle dans un backend** : `Backend` n'enregistre que le decider ; `LLM_SERVICE`
     n'existe que si `catalog.set_llm` est appelé, et personne ne l'appelle. Un `Decide
     via='llm'` dans un graphe d'outil n'a pas de modèle ; le banc enregistre le service
     lui-même. Aucun manifeste livré ne déclare `models.llm` ;
  2. **le découpage par sections** : `text-splitter` coupe sans titres ; aucun nœud ne rend les
     sections d'un Markdown avec leur titre, et rien ne résout une ancre `#titre` ;
  3. **le graphe de l'archiviste** et son outil ;
  4. **la relecture de l'ancien modèle** (Memory, Subject) de la mémoire de Lucie ;
  5. la reprise de `Decide` sur 429 et 503 (retour de la tentative 4).
- **La porte `ranger`, le repère `Rangement` et les crochets sont M9**, pas M4 :
  `CLAUDE_CODE_SESSION_ID` n'est pas encore passé à `POST /session`, et aucun crochet n'existe.
  L'entité `Rangement` est déclarée, personne ne l'écrit ; M4 ne la touche pas.

## 2. L'outil `archive`

`archive { chemin?, titre, contenu, origine = "envoi" }`, du gabarit memory, par MCP ou par
`/call`.

1. **Le L0** : `put_source` tel quel, `chemin` par défaut `sources/<jour>-<slug du titre>.md`,
   `via: archiviste`. La Source est immuable : le même envoi rejoué passe, un autre contenu au
   même chemin est refusé et dit.
2. **Le découpage**, sans modèle : les sections de titre du Markdown (`#`, `##`, `###`…).
   Chaque section rend `{title, level, text, anchor}` ; le texte avant le premier titre fait
   une section au titre du document ; deux titres identiques se numérotent (`titre`, `titre (2)`),
   pour que `doc:<chemin>#<titre>` reste unique. **Une section qui touche plusieurs sujets reçoit
   une décision**, celle de son sujet principal : le sous-découpage par le modèle attend d'être
   mesuré (§6).
3. **Les candidates** : `recall_fiches` sur le texte de la section, cinq fiches au plus (le
   banc rappelait 29 sur 31 avec trois ; la fiche manquée du rappel est la première erreur à
   régler). Chaque candidate se présente par son nom, son `about` et ses derniers apports,
   bornés : c'est « le contenu complet du sujet » que la mesure demandait ; quand M5 aura écrit
   le L2, c'est lui qui tiendra cette place.
4. **La décision fermée**, par section : `Decide(via='llm', labels_from=candidates,
   labels=['none', 'create'])`, l'instruction de la tentative 4 (une règle va sous le domaine où
   elle s'applique ; `none` si rien ne convient ; `create` si le sujet est net et n'a pas de
   fiche).
5. **L'écriture** :
   - une candidate : `put_apport {fiche, source, section: title, texte, niveau: L1, state:
     current, via: archiviste}`, `relier_apport`, `citer_source doc <chemin>#<title>` ;
   - `create` : une Fiche **`proposed`** (l'état existe, avec `accept` → `current` et `reject`
     → `retired`), nommée **par le titre de la section**, `about` = sa première phrase ; puis
     le même L1 dessus. Une proposition n'est pas une fiche tant que personne ne l'accepte ; la
     recherche la rend, marquée ;
   - `none` : rien d'écrit ; la section reste dans le L0, qui est cherchable (`bm25` sur Source),
     et le rapport la nomme.
6. **Le rapport**, rendu à l'appelant : `{source, sections: [{title, verdict, fiche?, apport?}],
   unranged, proposed, model}`. Un refus par section (modèle coupé, étiquette hors liste) se dit
   là, il n'arrête pas les autres.

**Sans modèle déclaré**, `archive` garde le L0, ne range rien et le dit : le filet n'invente
rien. C'est aussi le comportement d'un `Decide` qui ne répond pas après ses reprises.

## 3. Le graphe, déclaré, et le nœud qui manque : « pour chaque »

`graphs/archive.mmd` : `params (Rhai) → EntityRecordNode(Source, put) → SplitSectionsNode →
pour chaque section : [GraphNode(recall_fiches) → RhaiNode(candidates → labels, présentation) →
Decide(via llm) → Route(verdict) → EntityRecordNode(Apport, put) → LinkRecordNode(A_POUR_APPORT)
→ AddRefNode(doc), ou la même précédée d'un EntityRecordNode(Fiche, put, state proposed), ou un
compteur] → RenderResultsNode` du rapport.

**Ce que 73 a lu dans le code** (nuit du 25 au 26) : aucun nœud d'écriture ne lit par élément.
Une entrée de valeur (posture `Complete`) derrière un flux reçoit **le pli** du flux
(`concurrent.rs`, `merge_stream_elements`) ; deux JSON ne se plient pas, l'échec rend un
`Trigger`, et un `Trigger` s'efface devant la valeur suivante : pour N enregistrements, le
dernier si N est impair, rien si N est pair, **sans erreur**. C'est un défaut à part entière,
73 l'ouvre en ticket (un pli impossible doit refuser en le nommant). Un `GraphNode` ne
s'exécute pas non plus une fois par élément : le sous-graphe tourne une fois, les éléments
coulent à travers sa frontière. Seuls `Decide`, les nœuds de jetons et `spawn` lisent par
élément.

**La forme générale**, proposée par 73 à c7, plutôt qu'un nœud Rust propre à l'archive : un
nœud **« pour chaque »** (`ForEach`), qui lit son entrée en `PerElement` et appelle un
sous-graphe (un `GraphNode`, ou un graphe cité) **une fois par élément**, l'élément posé sur
une entrée nommée et les sorties remises en flux ; les écritures de chaque appel entrent dans
la transaction du run, et l'annulation d'un élément annule le run. Il règle d'un coup les
candidates par section et les écritures par section, pour tout graphe à venir. **M4 l'attend**
plutôt que d'écrire un nœud Rust qu'on remplacerait : les pas 1 et 2 (§7) ne dépendent pas de
lui, le pas 5 (la mesure sur Gemini, par le banc) non plus, et le pas 4 se joue par les outils.
Si c7 me le confie, je l'écris dans le dataflow avec son témoin (N éléments → N appels, N
écritures, l'annulation au k-ième).

## 4. Ce qui change dans le code, du générique au gabarit

| pièce | où | taille |
|---|---|---|
| `models.llm` d'un manifeste connecté à l'ouverture du backend (`connect_llm`, `catalog.set_llm`), `LLM_SERVICE` offert aux graphes ; sans déclaration, l'outil qui en a besoin le dit | `backend.rs`, à côté de `models.decide` | petit |
| la reprise de `Decide` sur 429 et 503 (trois essais, attente croissante, la raison dans l'erreur finale) | `decide_nodes.rs` ou le client `llm.rs` | petit |
| `SplitSectionsNode` : `{field, as}` → la liste des sections `{title, level, text, anchor}` | `src/dataflow/` | petit |
| `graphs/archive.mmd`, l'outil `archive`, le script Rhai des candidates, le rapport | `templates/backends/memory/` | moyen |
| la relecture de l'ancien modèle (§5) | un graphe du gabarit, joué une fois | petit |

Rien dans le schéma des entités : pas de version de format, pas de migration de fichiers.
Les identifiants sont en anglais (`archive`, `SplitSectionsNode`, `models.llm`, `none`,
`create`, `unranged`, `proposed`) ; les valeurs de données restent celles du gabarit (`via:
archiviste`, `niveau: L1`).

## 5. La relecture de l'ancien modèle de la mémoire de Lucie — jouée sur l'instantané

`scripts/relire_l_ancien_modele.py <backend.json> [ecrire]`, par les outils du gabarit sur une
base ouverte en appel unique par le binaire (le graphe attend « pour chaque », §3). Sans
`ecrire`, il lit et dit ce qu'il ferait. Ce qu'il fait :
- un L0 par jour de création, `sources/<jour>-ancien-modele.md`, une section par Memory
  (`said_as` pour titre, la `claim`, le `why`, la provenance avec l'uuid et les dates) ;
- une Fiche par Subject (nom, `about`, `current`) si elle manque ; une Memory sans Subject
  retrouvé donne une fiche `proposed` à son titre ;
- un L1 par Memory sur la fiche, texte = `claim` puis `why`, relié, cité par `doc:` (le genre
  créé au besoin) ; une Memory dont le texte est déjà celui d'un apport de la fiche est sautée ;
- rien n'est retiré, et le rapport dit ce qu'il a fait.

**Joué le 26 octobre à 0 h 15 sur une copie de l'instantané `migration-fiche-25-oct`** (sous
`~/.cache/rag3weaver-build/relecture-96/`, la vraie base intacte) : l'ancien modèle tient
exactement un Subject et une Memory, la paire « régime GPU » que la page 05 a portée à la main.
La relecture a écrit la Source `sources/2026-10-11-ancien-modele.md`, la Fiche « régime GPU »
(même uuid que sur la vraie base : il dérive du nom), son L1 et sa Ref `doc:` ; `cited_by` la
rend ; rejouée, elle saute tout. Les deux lignes Memory et Subject sont identiques avant et après.
Sur la vraie base, elle n'a donc rien à écrire aujourd'hui ; elle servira s'il revient des
Memory, et le jour où l'on retirera l'ancien modèle (`set_aside`, un geste à part, après le mot
de Lucie).

Limite : aucun outil ne rend `ANCHORED_TO`, le Subject d'une Memory se retrouve par `said_as`.

## 6. Les témoins et la mesure

- **`e2e_archiviste`, sur faux modèle** (comme `e2e_juge`) : un Markdown de trois sections
  entre ; le faux modèle répond le nom d'une fiche existante, `none`, puis `create`. On lit :
  un L1 sur la fiche avec sa Ref `doc:<chemin>#<titre>` ; une fiche `proposed` nommée par le
  titre, avec son L1 ; une section non rangée nommée par le rapport ; la Source immuable
  (rejouer passe, un autre contenu refuse) ; `cited_by <chemin>` rend les deux fiches ; une
  mémoire reconstruite de ses fichiers garde tout (`e2e_reconstruction`, générique) ; sans
  `models.llm`, le L0 est gardé et le rapport dit « aucun modèle ».
- **La tentative 5, sur Gemini** (le banc, `RAG3WEAVER_BANC_TENTATIVE=5`) : les 39 fiches de la
  tentative 4 avec `create` en plus de `none`, où les 8 sans sujet disent ce que le modèle fait
  d'un sujet net sans fiche. La mesure se lit avant d'allumer `create` pour Lucie ; si le
  modèle crée là où il devrait s'abstenir, `archive` démarre avec `create` éteint, et le dit.
- **Ce que la mesure tranchera ensuite**, pas cette nuit : le sous-découpage d'une section à
  plusieurs sujets ; le nom d'une fiche proposée par le modèle plutôt que par le titre ; le
  nombre de candidates.

## 7. L'ordre

1. Le générique : `models.llm` dans le backend et la reprise de `Decide`, avec leurs témoins.
2. `SplitSectionsNode`, avec son témoin (titres, texte avant le premier, doublons).
3. Le graphe `archive`, l'outil, le rapport ; `e2e_archiviste` sur faux modèle.
4. La relecture de l'ancien modèle, jouée sur une copie de la base de Lucie d'abord
   (l'instantané `migration-fiche-25-oct`), puis sur la sienne après son mot.
5. La tentative 5 sur Gemini, et sa note dans `templates/experiments/`.

Chaque pas se pousse sur sa branche ; master après le mot de c7 et le tour de luciepc, avec
`e2e_memoire_modele`, `e2e_memoire_fichiers`, `e2e_cited_by`, `e2e_reconstruction`, `e2e_juge`
et `e2e_equipier` au rejeu, puisque le gabarit memory bouge.
