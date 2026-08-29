# 11 — Rapport de session : le 27 après-midi et soir

Écrit à contexte plein, pour reprendre froid. Le [doc 10](10-journal-du-27-apres-midi.md)
garde les commandes et les retours bruts ; celui-ci garde **ce qu'on a appris**,
y compris ce qu'on a cru et qui était faux.

---

## 1. Ce qui a été livré et commité

```
81ca594e1  feat(trace): un participant est une identité, pas une session
3fdee9d3f  docs(27 août): le compteur, et le terminal à plusieurs
5adac3d33  perf(embed): borner les lots par le texte, pas par le nombre
cef6d6b14  fix(burn): le choix de carte existait, personne ne l'appelait
20e04670d  feat(meter): compter ce qui est consommé…
ea166079c  build(e2e): le journal survit, les échecs sont nommés, le ménage passe avant
c0077b504  fix(config): alimenter une KB compte comme du contenu
f2bdbdc7d  feat(config): le langage de déclaration apprend le coût et l'état
```

Dernière passe verte : **33 suites, 277 E2E, 819 unitaires.**

---

## 2. Ce qui est écrit mais **pas commité**

Compilé, unitaires verts, **aucune passe complète depuis** :

| Fichier | Ce que c'est |
|---|---|
| `src/events.rs` | `conversation` dans l'enveloppe, `send_in`, `broadcast` |
| `src/agent.rs` | `with_domain`, et les outils de dialogue **annoncés au modèle** |
| `src/dataflow/trace_nodes.rs` | `Run.domain`, le fil **dit** l'emporte sur le fil dérivé |
| `src/dataflow/runtime.rs` | `domain` vide pour un run de graphe |
| `tests/e2e_conversation_a_plusieurs.rs` | **nouveau** — l'expérience à plusieurs |
| `docs/…/09` §5 bis, ter, quater | tour de parole, enveloppe de fichiers, résultats mesurés |
| `docs/…/10` | le journal des mesures |

**À faire en reprenant** : une passe complète, puis commit.

---

## 3. Les expériences, et ce qu'elles ont mesuré

Le tournant de la soirée : *« on construit une expérience et même si foireuse
on a des traces et on itère empiriquement, plutôt que juste valider "tiens le
test compile" »*. Quatre manches, artefacts dans `target/artefacts/`.

| Manche | Montage | Résultat |
|---|---|---|
| 1 · `fil-a-plusieurs.md` | `src/dataflow` seul, 6 tours, `HashEmbedder` | aucun n'a conclu ; `RepeatedError` sur `read` |
| 2 · `fil-essai2.md` | tout `src/`, 14 tours | 3 × `Finished(Eos)`, 13 appels chacun, réponses **contradictoires** |
| 3 · `fil-temoin.md` | idem, **sans phrase de domaine** | 13 appels chacun, mêmes outils, deux phrases d'ouverture **identiques** |
| 4 · `fil-vrai-moteur.md` | **BGE-M3 réel** | écrit, **pas encore lu** |

### Ce qui est solide

- **Le rôle en prose ne porte rien.** Témoin à l'appui : sans la phrase de
  domaine, mêmes comptes d'appels, même répartition d'outils, deux réponses
  qui commencent par la même phrase mot pour mot. Le critère falsifiable du
  [doc 09 §3](09-le-terminal-a-plusieurs.md) a été **exécuté**, et il rend non.
- **Zéro `pause_dialogue`** sur les quatre manches, alors que l'outil est
  déclaré et l'invite le demande.
- **Les agents se contredisent sans que rien ne le voie.** Zed : *« on peut se
  retrouver avec un index à moitié à jour »*. Maurice : *« l'index ne sera pas
  à moitié à jour »*. Même fil, même question.
- **Les outils ne mentent jamais.** Neuf fois ils ont calculé la bonne réponse
  et l'ont rendue en message : *« did you mean: catalog.rs »*, *« Paths are
  relative to the root of this source, not to the repository »*. L'agent a
  renvoyé le même appel.

### Les conclusions que la mesure a tuées

C'est la partie utile.

1. ~~« Les rôles les font diverger »~~ — le témoin dit que c'est du tirage.
2. ~~« Ils boudent le RAG »~~ — 0/39 puis 2/39, la différence est du bruit.
3. ~~« Ils sous-utilisent `search` »~~ — **on leur avait donné un `search`
   amputé** : `HashEmbedder` factice, 34 résultats tous `bm25`, et
   `search(Scope, "node failure")` → *No results*. De là où ils se tenaient,
   `search` **était** un grep avec des étapes en plus. Ils avaient raison.
4. ~~« Le montage est bon, le moteur est en cause »~~ — manche 1 : question sur
   le catalogue, corpus limité à `src/dataflow`. Le montage contredisait sa
   propre question.

> Quatre fois, je regardais le comportement de l'agent sans vérifier ce que le
> système lui présentait.

---

## 4. Cinq mécanismes construits et jamais branchés

Le motif de la journée. Aucun n'aurait été trouvé par une relecture.

| Mécanisme | Découvert par |
|---|---|
| `BurnDevice::for_role` + 4 variables | quatre mesures de cartes rendant le même temps |
| `Postures::describe_for` | en cherchant où l'injecter — personne ne l'appelait |
| borne de lot du dense et du sparse | le **dual du même nœud** la respectait déjà |
| `pause_dialogue` / `confirm_pause` | jamais **annoncés** au modèle ; les tests scriptaient l'appel |
| `FlushConfig::embed_batch_size` | déclaré, testé en sérialisation, appliqué nulle part |

Et une variante, découverte ce soir : **l'outil calcule la bonne réponse et la
met dans un message d'erreur.** Le remède n'est pas un meilleur texte, c'est
d'appliquer ce qu'on a trouvé — *« interprété comme `catalog.rs` »* plutôt que
*« did you mean: catalog.rs »*.

---

## 5. La maquette d'origine — ce qu'on a perdu

Lucie a pointé `LR_CodeRag`, le projet précédent. **Trois documents à relire**,
et ils ne sont pas anecdotiques.

### 5.1 `…/kuzu-wasm-exp/l5/code-rag/` — la maquette du schéma

`schema.js`, `hooks.js`, `presets.js`, `index.js` (20 Ko en tout).

Le schéma est **déjà le nôtre** : `File` / `Scope` / `Library`, et les mêmes
relations à un nom près — `DEFINED_IN`, `CONSUMES`, `CONSUMED_BY`, `PARENT_OF`,
`HAS_PARENT`, `USES_LIBRARY`, `IMPORTS`, plus `INHERITS_FROM`, `IMPLEMENTS`,
`DECORATES` qu'on n'a pas.

**Trois idées perdues**, chacune répondant à un défaut mesuré ce soir :

| Idée | Ce qu'elle corrige |
|---|---|
| `boostIf` par type de scope (`presets.js`) — *« les conteneurs sont souvent de meilleurs points d'entrée »* | notre premier résultat pour `drain` était `file_scope_06`, **un bloc de commentaires**, au-dessus de `Catalog::drain` |
| `enrichClassContent` (`hooks.js`) — une section « Members: » ajoutée au contenu d'une classe **avant l'ingestion** | une classe devient trouvable par ce qu'elle contient |
| `enrichCodeResult` (`hooks.js`) — pour un conteneur, aller chercher les enfants **pertinents pour la requête** via `PARENT_OF`, automatiquement | chez nous `search_expand` est un outil séparé que l'agent doit penser à appeler — et n'appelle jamais |

Le `boost` existe encore dans `search.rs` comme fusion de signaux, **plus comme
réglage par champ**. Les deux enrichissements n'existent pas.

### 5.2 `ragforge/docs/BRAIN_SEARCH_OUTPUT_PROPOSAL.md` — le format de sortie

202 lignes. Sa section « Problème Actuel » est **notre défaut d'aujourd'hui,
mot pour mot** :

- *« données redondantes : `absolutePath`, `file`, `filePath` — 3× le même path »*
  → chez nous le chemin absolu deux fois par ligne, plus `cursor=worktree:/home/…`
- *« champs internes exposés »* → `bm25`, les scores bruts, `scope_type=`
  dupliqué avec `signature=`
- *« pas de format lisible pour humains »*

Gain chiffré : **112 K → 8 K**, lisibilité d'une étoile à cinq.

**Les quatre règles**, dont on n'applique aucune :

1. **La signature est le titre** — `const FIELD_MAPPING: Record<…> (variable) ★ 1.26`.
   Nous affichons `file_scope_06`.
2. **Le chemin est relatif**, une fois, sur sa ligne avec les numéros.
3. **On montre la docstring**, pas le contenu brut tronqué — nous répétons le
   même texte deux fois, une fois coupé dans `content=`, une fois en citation.
4. **Un arbre ASCII des dépendances** en fin de sortie, groupé par relation,
   profondeur bornée. Le code de `buildAsciiTree` est dans le document.

La quatrième est la plus intéressante : c'est `search_expand` rendu
**automatiquement pour tous les résultats**. Ça rejoint `enrichCodeResult`, et
ça explique peut-être pourquoi nos agents ne l'appellent jamais — **chez eux, il
n'y avait rien à appeler.**

### 5.3 `ragforge/docs/brain_search_example_output.md` — l'artefact réel

81 lignes, une sortie complète pour `"FIELD_MAPPING unified field extraction"`.
C'est le modèle à copier, pas à paraphraser.

---

## 6. Le reste à faire, par valeur

1. **Une passe complète**, puis commiter le §2.
2. **Porter les quatre règles de rendu** dans `render_nodes.rs`. C'est le plus
   gros gain immédiat : ça touche chaque résultat de chaque agent.
3. **Lire `fil-vrai-moteur.md`** — la manche avec BGE-M3, écrite et pas lue.
4. **L'A/B de la description de `search`.** Elle parle notre plomberie
   (*« entité »*, *« base de connaissances »*, *« chunks résolus vers leur
   entité parente »*) là où `grep` dit ce qu'on obtient. Elle ne dit jamais ce
   que `search` fait que `grep` ne fait pas : **chercher par le sens**. Ne
   changer que ça, et remesurer.
5. **L'enveloppe de fichiers** (`WorkDomain` appliqué au `FileSource`), le `cd`,
   et la ligne dérivée *« vous travaillez dans X, racine autorisée Y »* —
   [doc 09 §5 ter](09-le-terminal-a-plusieurs.md).
6. `boostIf` par type de scope, et les deux enrichissements de la maquette.
7. Le relevé du compteur — `Meter::describe()` existe, personne ne l'affiche.
8. `Trace` sans `hashsafe`.

---

## 7. Ce que je retiens de la méthode

**L'expérience trouve ce que la validation manque.** Les tests scriptés savent
déjà ce qui va arriver ; c'est pour ça qu'aucun n'avait vu que `pause_dialogue`
n'était pas annoncé au modèle.

**Ne changer qu'une chose à la fois.** Le témoin n'a servi que parce que rien
d'autre n'avait bougé. J'ai failli modifier la description de `search` pendant
qu'une manche tournait.

**Vérifier ce que le système présente avant de juger ce que l'agent fait.**
Quatre fois de suite, et à chaque fois c'est une question de Lucie qui l'a
rattrapé.
