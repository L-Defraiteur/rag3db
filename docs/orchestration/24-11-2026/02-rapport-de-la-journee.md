# Rapport de la journée du 24 novembre 2026 (11 h – 17 h 30)

Suite du rapport de la nuit (`01-rapport-de-la-nuit.md`, sur master). Les autres sessions sont arrêtées : Lucie ménage
son usage ; seule l'orchestration a tourné, avec quelques sous-agents.

## L'hôte stable de Lucie (posé à la tablette)

Branche `c7/24-11-2026-sans-copie`, bâtie à luciepc, liée à la lib épinglée `fc64aca6f3`, installée dans `~/.local/bin`
(anciens binaires en `.avant-c7-sans-copie`, `.avant-c54c9f4aa0`, `.avant-38d253fc8a`) :

- **plus de copie de travail** : un dossier sans git est travaillé en place (la veille, un terminal lancé depuis
  `~/git_workspaces` avait copié 106 Go, Sairen compris ; copie effacée au mot de Lucie) ;
- **l'agent est dit l'état des index** à l'entrée dans le dossier et à chaque demande, et n'indexe qu'à la demande ;
- **le terminal relance l'hôte** s'il est absent ou ne répond pas ;
- **les consignes tapées après le premier message** (`instruction`) comptent comme demandes : le rappel des tours d'avant
  et la demande du tour les voient (l'agent ne voyait plus son tour précédent) ; dernières lignes gardées à 6 000
  caractères ;
- **invite** : « search_code EN PREMIER sur un sujet neuf » ; « un rapport écrit : sa conclusion courte et son chemin ».
  Posées aussi à chaud en surcharge (`~/.config/rag3weaver/templates/teammate/prompts/system.md`) : **à retirer** dès que
  la branche est posée normalement.

## Bâti, pas encore posé

- `c7/24-11-2026-pose` : la fusion des correctifs ci-dessus et du premier lot de contexte.
- `c7/24-11-2026-contexte` (premier lot) : règle d'invite « Appris : … Ensuite : … » ; réécriture par paquets des vieilles
  sorties commentées en talons (seuil 20 000, 8 000 gardés), faits de l'index gardés par du code.
- `c7/24-11-2026-contexte-2` (second lot, calqué sur gemini-cli) : grosse sortie mise de côté sur disque avec coupe et
  résumé ; résumé des sorties non commentées ; budget du pas (65 000 / 150 000) et instantané d'état. **Avant la pose** :
  déclarer le modèle léger (`gemini-3.1-flash-lite`) dans `models.llm` et son prix dans `prices.json`, sinon le repli sur
  le modèle de l'équipier coûte plus qu'il n'économise.
- `c7/24-11-2026-eclaireur` : l'éclaireur de l'index au début du tour (score `knows`, mode bloquant, `chemin:ligne`
  vérifiés par code).

## Mesures

- La passe difficile (plafond 64 000 forcé, trois tirages) : 16 noms inventés sur 26 en médiane avant les correctifs de
  contexte, 5 après ; réglages par défaut : 0.
- Coût : 8,28 $ le 24 sur l'hôte de la tablette (la nuit ~4 $, les essais de Lucie ~4 $ en 2 h 30 : 574 appels, 37 000
  jetons d'entrée en moyenne, 86 % servis par le cache). La facture Vertex est plus haute que le registre (tests, bancs,
  passes hors registre) ; pics à 80 € les 15 et 20 novembre.
- Qwen3-Coder-Next Q4_K_M (80 B, 3 B actifs) téléchargé (`~/models/qwen3-coder-next-q4_k_m/`), testé (50 jetons/s en
  écriture, 520/s en lecture), puis arrêté : il fait ramer la tablette. Ornith 122B déplacé à luciepc
  (`~/models/ornith-1.5-122b-uncensored/`, sha256 vérifiée), 272 Go libérés à la tablette.

## Décisions de Lucie

- Plus aucune copie de travail ; un agent travaille dans le dossier.
- Appels au vrai modèle payant seulement pour une vraie étape à tester ; sinon faux modèle, modèle local ou modèle léger.
- S'inspirer de gemini-cli « sur tous les points » pour le contexte.
- L'asynchrone dans les graphes en priorité : un nœud d'attente par événement, aux ports déduits du câblage.
- Plugin Claude Code : Claude par défaut pour ses traitements (`claude -p`), nos modèles en option.
- Produit viable et financé : démo de la quête d'abord, puis Bourse French Tech, crédits cloud, prévente « licence
  fondateur », investisseurs. Pas de Kickstarter.
- La quête des véhicules d'abord ; le monde de voxels original ensuite.
- Le design de la maquette lui plaît ; style tiré du logo R3W.

## Tickets écrits (branche `c7/24-11-2026-sans-copie`, `docs/orchestration/tickets/`)

L'hôte sourd pendant un travail lourd (et le nœud d'attente) ; les outils d'index sur un dépôt TypeScript étranger (le
faux homonyme de `describe()`, le filtre `src/`) ; l'éclaireur de l'index ; prévenir l'utilisatrice dès qu'une action
est faite (`notify_user`, adresse servie repérée) ; l'agent voit les images (deux chemins selon la capacité déclarée) ;
changer de modèle dans une conversation ouverte ; le terminal aveugle après une relance de l'hôte ; ingérer un catalogue
xlsx en composant un DAG (aussi la consigne de la quête).

## Références et maquette

- Références sur le coût : `~/git_workspaces/G/24-11-2026-references/` (Goose, Hermes Agent, Pi, GenericAgent, Aider,
  huit articles) ; fiche déposée au pousseur pour `docs/optimiseur/24-11-2026/`.
- Étude de style : `~/Documents/rag3weaver-presentation/etude-de-marche/` (12 produits, 36 captures, trois directions).
- Maquette « La ruche rag3weaver » (artefact privé) : l'accueil (animé), le matin, le flux d'une revue (animé, boucle
  par ré-entrée), le gestionnaire de merge (chat).

## À la reprise

1. Poser `contexte-2` (modèle léger déclaré d'abord), retirer la surcharge de `system.md`, mesurer au registre.
2. Déposer les lots au pousseur avec leurs témoins quand les sessions reprennent.
3. Les tickets et l'objet swarm, puis la quête des véhicules (le catalogue xlsx d'abord).
4. Choisir une des trois directions de style.
