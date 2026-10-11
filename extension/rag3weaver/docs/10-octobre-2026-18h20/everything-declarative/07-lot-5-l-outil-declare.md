# Lot 5 — l'agent écrit des déclarations : l'outil `declare`

11 octobre 2026, session « everything declarative » (chantier I). L'agent
modifie le backend qu'il sert en écrivant ses déclarations ; chaque écriture
est vérifiée, mise en service et commitée avec sa raison.

## 1. Un geste d'hôte

`declare` n'est pas un nœud de graphe : un graphe déclaré qui réécrirait ses
propres déclarations rendrait la frontière (l'exposition par clés, vision §7)
rechargeable de l'intérieur — le même argument que pour la montre sur les
fichiers (session mémoire). L'hôte donne à l'agent trois outils à côté de
ceux du manifeste (`DeclarationTools`, qui enveloppe n'importe quel
`ToolBox`) :

| outil | ce qu'il fait |
|---|---|
| `list_declarations` | les fichiers déclarés du backend, chemins relatifs |
| `read_declaration(path)` | le contenu d'un fichier déclaré |
| `declare(path, content, reason)` | écrit un fichier entier, vérifié, en service, commité |

`Backend::declare`, `read_declaration`, `list_declarations` (`src/declare.rs`)
sont les mêmes gestes pour un autre hôte (la page vivante du lot 6).

## 2. Ce que `declare` accepte

- Un chemin **relatif** au dossier du backend, d'une sorte connue : `json`,
  `mmd`, `ts`, `js`, `rhai`, `html`, `htm`, `jinja`, `txt`, `xml`, `md`.
- Refusés, en nommant le chemin : absolu, hors du dossier (`..`), caché
  (`.git` compris), le dossier de la base, toute autre sorte (jamais du code
  compilé), une raison vide.

## 3. Vérifier avant d'écrire, et l'histoire

1. Une déclaration à la fois dans le processus.
2. Le nouveau contenu est écrit (renommage atomique), puis **revérifié comme
   au chargement** (`check_reload`). Refusé : l'ancien contenu est remis (ou
   le fichier neuf retiré), et le refus revient à l'agent avec sa raison et
   sa ligne quand le chargement la connaît (vues, TypeScript, mermaid).
3. Accepté : mis en service (`apply_reload`), puis **commité** dans le dépôt
   git qui **englobe** le dossier (créé seulement s'il n'y en a aucun
   au-dessus) — **ce chemin seul** (`git add -- <chemin>`, `git commit --
   <chemin>` : jamais `-A`, jamais un fichier hors du dossier, les autres
   changements de la personne restent où ils sont), sous l'**identité git du
   dépôt** (c'est la personne qui a demandé), avec pour message la raison de
   l'agent, puis `declare: <chemin>`. Pas de ligne d'attribution.
4. Si git refuse le commit (identité absente…), la déclaration est retirée
   et le service revient à la version d'avant : pas de changement sans sa
   raison. git absent du poste : refus nommé.
5. Le même contenu une seconde fois : rien à commiter (`commit: None`).

Décidé par l'orchestration le 11 octobre (renversable par Lucie) : le dépôt
englobant, le commit par chemin, l'identité de la personne. Le retour en
arrière (un commit qui défait) viendra plus tard.

## 4. Les témoins (`src/backend_reload_tests.rs`)

Rouges à la compilation, puis verts (6 témoins neufs, 31 au total) :

- ce qui n'est pas une déclaration de ce backend est refusé en le nommant ;
  une raison vide aussi ;
- une vue invalide est refusée avec sa ligne ; le fichier, la page, le dépôt
  et la version ne changent pas ;
- une vue valide est en service et commitée : le message commence par la
  raison et porte `declare: views/hello.html`, l'auteur est l'identité du
  dépôt, le commit ne contient que ce chemin ; le même contenu ne recommite
  pas ;
- **les autres fichiers modifiés ou non suivis du dépôt ne sont pas
  commités** ;
- une déclaration neuve dans un dossier neuf s'écrit, se liste, se lit ;
- **un modèle enregistré** pose une vue invalide, reçoit le refus (qui
  nomme le fichier), corrige au tour suivant : la page change, le commit
  porte sa raison.

## 5. Ce qui reste

- Le retour en arrière d'une déclaration (un commit qui défait).
- La page vivante (lot 6) : le chat, le graphe du backend, la page produite
  et le journal des rechargements, poussés au navigateur.
