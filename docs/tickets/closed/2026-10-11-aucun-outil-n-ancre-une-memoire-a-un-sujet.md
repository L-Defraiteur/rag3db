# Aucun outil du gabarit `memory` n'ancre une mémoire à un sujet, donc sa réaction ne peut pas partir en service

- **état** : corrigé par la marche 3 du lien générique (le commit qui déplace ce fichier dans `closed/`, 11 octobre 2026) — le gabarit `memory` déclare `anchor` (`ANCHORED_TO`) et `supersede_link` (`REPLACES`) sur `templates/tools/link.mmd`, avec leurs gardes d'états ; `e2e_reaction_montee` pose son ancre par l'outil
- **gravité** : déclaration inatteignable — la réaction `review_anchored` est montée et ne peut rien transitionner
- **atteignable en service** : oui, c'est précisément le cas en service
- **touche rag3weaver** : oui, et seulement rag3weaver (un gabarit, pas le moteur)

## Ce que c'est

Le gabarit `templates/backends/memory` déclare la relation `ANCHORED_TO`
(`Memory → Subject`) et, depuis `0387fe401`, la réaction `review_anchored` qui
remonte cette relation pour faire repasser « à revoir » les mémoires dont le
sujet a changé.

Ses sept outils exposés sont `put_memory`, `get_memory`, `put_subject`,
`recall`, `recall_subjects`, `add_ref`, `create_ref_type`. **Aucun ne pose
`ANCHORED_TO`** — `add_ref` pose `CITE` (`Memory → Ref`), et `REPLACES` n'a pas
d'outil non plus.

Donc un agent branché par MCP peut écrire des mémoires et des sujets, mais pas
les relier : la réaction tourne à chaque écriture de sujet et trouve toujours
zéro ligne concernée. Le montage est vrai, la machine est vraie, et le chemin
qui les relie est fermé par en haut.

## La recette minimale

Jouée, journal à `docs/10-octobre-2026-18h00/07-l-ordre-a-la-memoire-branchee.md` :

1. `rag3weaver-backend mcp --manifest <…>/backends/memory/backend.json` ;
2. `tools/list` → les sept noms ci-dessus ;
3. chercher un verbe qui prenne `ANCHORED_TO` ou deux identités : il n'y en a pas.

## Le témoin

Aucun, et c'est la raison pour laquelle le défaut a vécu : le témoin du montage
(`tests/e2e_reaction_montee.rs`) pose l'ancre **par le catalogue**
(`Catalog::link`), pas par un outil — il éprouve le réacteur, pas la surface
exposée. Un témoin de ce défaut-ci se lit en deux lignes : la liste des outils
d'un gabarit doit couvrir chaque relation que ses réactions remontent.

## La cause

Le gabarit a reçu ses entités et ses relations avant d'avoir une réaction, et
`ANCHORED_TO` n'avait alors aucun lecteur : personne ne s'est aperçu qu'aucun
verbe ne l'écrivait. La réaction, déclarée le 11 octobre, lui a donné son
premier lecteur — et a rendu le trou visible.

## Ce qu'il faut pour le fermer

**Et d'abord : ce qui ne le ferme pas.** J'avais écrit ici que la pièce existait
déjà — `templates/tools/link_snapshot.mmd` (`RelationBatchNode`) —, qu'il
suffisait de la déclarer, et je l'ai dit à l'orchestration qui a répondu « oui,
six lignes ». **C'est faux, et c'est mesuré** (11 octobre 2026, essai joué par
MCP sur une copie hors dépôt, outil `anchor` déclaré puis retiré) :

```
anchor(links: [{from: {claim, reach}, to: {name}}])
→ "snapshot links cannot modify relations of managed entities"
```

`RelationBatchNode` refuse si **l'un des deux bouts est géré** — un cycle de
vie, une révision, `created_at`, `updated_at` ou une date de transition
(`backend_nodes.rs:757-766`). `Memory` et `Subject` ont les trois dernières et
un cycle de vie chacun. Le nœud est, par sa propre documentation, pour les liens
d'un **instantané** : « Snapshot links use stable endpoint identities ». Aucun
témoin n'épingle cette garde, donc elle ressemble à une portée (« ce nœud sert
aux instantanés ») plus qu'à un invariant démontré — mais une garde ne se
relâche pas sans prouver ce qu'elle mesurait.

Il n'y a donc, à cette date, **aucun nœud qui relie deux entités gérées par
leur identité**. Le seul qui relie une entité gérée est `AddRefNode`, et il
fabrique une `Ref` en chemin : il ne sert pas ici.

Trois chemins, et aucun ne tient en six lignes :

1. **Un nœud de lien générique** (`LinkRecordsNode(relation, links)`) : deux
   identités déclarées, une relation sans propriété, la garde de cycle de vie
   des deux bouts inchangée — c'est le plus propre, et c'est une signature
   nouvelle.
2. **Un verbe de plus sur `EntityRecordNode`** (`action='link'`) : moins de
   surface, mais le nœud mélangerait écrire un record et poser une relation.
3. **Élargir la portée de `RelationBatchNode`** : le moins de code, et le plus
   de risque — il faudrait d'abord établir ce que la garde protégeait, puisque
   rien ne l'épingle. À ne pas faire sans cette preuve.

**La garde de `RelationBatchNode` n'est éprouvée par aucun témoin : ce qu'elle
protège n'est pas établi.** Qu'on la relâche ou non, quelqu'un devrait
l'épingler — un test qui dit *pourquoi* un bout géré est écarté, et non
seulement qu'il l'est. Tant que ce test n'existe pas, le chemin 3 est à écarter
par principe et non par mesure, ce qui est une faiblesse de ce ticket.

La suite est découpée en deux tickets, dans cet ordre :

1. [Aucun nœud ne relie deux entités gérées par leur identité](2026-10-11-aucun-noeud-ne-relie-deux-entites-gerees.md)
   — la cause générale, le `LinkRecordsNode` et les deux déclarations du gabarit
   (`ANCHORED_TO` et `REPLACES`). Une à deux passes, et c'est une signature
   nouvelle : décision de Lucie ;
2. [Une réaction peut remonter une relation qu'aucun outil n'écrit](2026-10-11-une-reaction-peut-remonter-une-relation-qu-aucun-outil-n-ecrit.md)
   — le huitième refus de la section `"reactions"`, qui aurait attrapé ce
   défaut-ci au chargement. Une demi-passe, **après** le premier : écrit avant,
   il rendrait le gabarit `memory` livré inchargeable.

Ce ticket-ci se ferme avec le premier.

**Et une règle à écrire si elle est retenue** : le chargement pourrait refuser
un manifeste dont une réaction remonte une relation qu'aucun outil n'écrit. Ce
serait le huitième refus de la section `"reactions"`, et celui-ci se lit sur le
manifeste seul. Il aurait attrapé ce défaut-ci au chargement du gabarit.
