# Aucun outil du gabarit `memory` n'ancre une mémoire à un sujet, donc sa réaction ne peut pas partir en service

- **état** : ouvert (mesuré le 11 octobre 2026, passe MCP de l'ordre A)
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

La pièce existe déjà : `templates/tools/link_snapshot.mmd`
(`RelationBatchNode(relation=$relation, links=$links)`) relie des identités
déclarées, sans propriétés, avec validation du lot avant écriture. Il suffit de
la déclarer dans le gabarit, liée sur `ANCHORED_TO`, avec une `description` qui
parle de mémoire et non d'instantané :

```json
"anchor": {
  "graph": "../../tools/link_snapshot.mmd",
  "bindings": { "relation": "ANCHORED_TO" },
  "description": "Ancre une mémoire à un sujet : la mémoire repassera « à revoir » si le sujet change."
}
```

Non fait ici parce que l'ordre était de brancher le gabarit **tel quel** en une
passe. À décider avec le même geste : exposer aussi `REPLACES` (une mémoire qui
remplace une autre), que la vision de la mémoire longue emploie et qu'aucun
outil n'écrit non plus.

**Et une règle à écrire si elle est retenue** : le chargement pourrait refuser
un manifeste dont une réaction remonte une relation qu'aucun outil n'écrit. Ce
serait le huitième refus de la section `"reactions"`, et celui-ci se lit sur le
manifeste seul.
