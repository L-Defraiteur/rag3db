# Reprise du relais contextuel — 11 novembre 2026

## Sources et périmètre

Branche `cx-ed-relais`, worktree `/home/lucied/git_workspaces/cx-ed-relais` aux deux postes.
Source ciblée `14585ca4a`, puis documentation seule `ee0222542`. Contrat : page 04 voisine.
L'atelier déclare huit lectures contextuelles et wait_output. Les graphes gardent l'opération
fixe et les arguments typés ; InvokeToolNode et InvocationHost fournissent le mécanisme commun,
sans dispatcher par nom d'outil. Le premier résolveur accepte une référence backend hébergée
exacte ; espaces et vues indisponibles refusent. L'inventaire des worktrees donne la référence.

L'autorité appelante vient des services du runtime et de la politique de l'outil, pas des
arguments du modèle. Les domaines doivent correspondre. La cible doit être une opération
prouvée en lecture seule (primitive snapshot_safe, politique, harnais et crochet examinés).
Le premier lot refuse donc les relais imbriqués et les effets non établis ; il ne prétend pas
autoriser tous les outils de tous les domaines. L'annulation parent s'attache au runtime enfant.
La provenance du reçu reste dans Turn.metadata et ToolCallFinished.metadata, hors du texte
tronquable. Pas de conversion en appel humain ou discret.

Les fichiers code.rs et le gabarit code restent inchangés. Aucun orchestrateur multiagents,
autorisation par le juge ou archiviste supplémentaire. Archi tient l'UI et son témoin contextuel.

## Preuves et résultats distincts

- Rouge navigateur antérieur d'archi sur `dbf3f7419` : search_code absent des outils reçus par
  le modèle (3,78 s). Pas de reçu ni lien inventé.
- Premier ciblé ED sur `2d81847ff` : compilation rouge, deux fixtures Backend sans nouveau
  champ invocation_host. Aucun test joué ; fixtures complétées dans `79a13dc6b`.
- Ciblé suivant : compilation passe, 6 verts, un rouge, un navigateur ignoré par le filtre trop
  large. Le rouge de la boucle modifiait le manifeste cible sans le rouvrir ; l'opération ajoutée
  restait non exposée. La fixture utilise maintenant l'ouverture normale de l'hôte avec session.
- Ciblé 3 sur `14585ca4a` : modules précis, 5/5 primitive, 2/2 hôte (1,64 s), 1/1 transport,
  zéro échec et zéro ignoré. Outline atteint un vrai dépôt et rend sa cible ; les gardes vérifient
  notamment droits, domaine, auto-appel, référence inconnue, vue indisponible, boucle et arguments
  sans injection. Le contexte survit à troncature et sérialisation de l'historique.
- Gabarits embarqués comparés octet pour octet aux fichiers du worktree courant, verts.
  OUT_DIR relevé : `target-I-apres/debug/build/rag3weaver-dce91fc913aa759c/out`.
- Suite features et lib entière en cours, unité `cx-ed-relais-socle-1` : pas de verdict global.
  Archi a reçu la source pour l'intégration diagnostique du parcours à deux dépôts et du clic.
  Le parcours UI complet n'est pas encore déclaré vert.

Preuves et scripts sur luciepc : `~/.cache/rag3weaver-build/I-luciepc/relais-20261111/`,
`cible-{1,2,3}.log`, `commit-cible-3`, `socle-1.sh`, `check-1.log`, `lib-1.log`.
Target unique de session : `~/.cache/rag3weaver-build/target-I-apres`, repris explicitement
après la fin de la passe remaining, sans cargo concurrent. Ne pas l'effacer comme artefact de
remaining pendant qu'il appartient au relais. Les preuves remaining restent dans leur dossier.

## Pour continuer

Lire le résultat du socle, puis intégrer le complément UI d'archi dans une branche cx neuve
et rejouer les contrôles nécessaires sur la source commune. L'ancien inventaire d'absences
atelier est un instantané, à adapter ; droits du dépôt hors tâche et de la tâche restent des
invariants. La preuve navigateur doit exécuter la recherche de B malgré des chemins/symboles
identiques dans A, cliquer son vrai lien et conserver la cible après rechargement, y compris
si le texte du reçu est tronqué. Aucun repli vers l'identifiant de conversation.

La fenêtre de push reste tenue par l'orchestration ; aucun push master de ce lot. Les suites
de lecture et navigateur du changement restent à juger ; un vert ciblé ne les remplace pas.
