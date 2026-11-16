# Qualifier la couverture sur des corpus publics par langage

- **État** : fait — e5e2d9cf5a ; première référence exploratoire32382/32382, oracles indépendants et preuves aux deux postes ; suites explicitement ticketées
- **Famille** : produit
- **Gravité** : preuve de couverture ; un parsing sans erreur ne démontre pas des captures ou relations complètes
- **Atteignable en service** : non, banc isolé ; les défauts de parseur révélés seront qualifiés séparément
- **Touche rag3weaver** : oui, faits d'index de code ; première entrée limitée au producteur et à ses fixtures de corpus
- **Pour** : codeparsers pour CLI/protocole/qualification ; Banc pour acquisition, orchestration pour sélection et coordination
- **Signalé par** : Lucie via orchestration le 16 novembre, corpus publics pour chaque langage supporté, environ 2–3 mille fichiers source par langage

## Périmètre autorisé

Téléchargements de dépôts publics autorisés, sans exécuter leurs scripts de
build, tests, installations ou programmes. Ni secrets, services, base de Lucie,
modèles ou reprise d'ancien GO. Commits/licences/hash et sélection déterministe
à publier ; langages et feature flags réels à relire avant de choisir les corpus.
C7 prend sélection et documentation ; Banc prend acquisition en parallèle.
Aucun défaut masqué par le statut parseOK, aucun claim d'exhaustivité.

## Travail attendu et preuve de fin

1. Inventaire des langages réellement reconnus, versions parser/grammaires,
   extensions, options et limites ; distinguer reconnaissance et extraction.
2. Protocole/CLI pour chaque fichier : extension non prise en charge, zéro scope,
   erreurs AST/parseur, captures absentes/non jugées, scopes/symboles/relations,
   temps et mémoire/pic avec provenance de mesure. Définir ce qui n'est pas mesuré.
3. Corpus figés et hashés, sélection triée déterministe d'environ 2000–3000
   fichiers pertinents/langage, manifeste source/licence/exclusions, pas de code
   téléchargé exécuté. Sous-effectifs explicites, pas duplication pour gonfler.
4. Source et recette publiées, créneau et budget frais coordonnés avant charge.
   Rouges conservés, premier échec inattendu diagnostiqué, suites non jouées
   explicites. Rapport séparé par langage et cas moins usuels, défauts révélés
   rattachés à leurs tickets. Parsing réussi n'est pas un oracle de couverture.

La qualification du correctif Rust7601 et ce banc sont distincts ; aucune
nouvelle campagne ne remplace ses validations restantes. Le présent ticket
n'autorise pas de nouveau Cargo ou mesure sans son profil et son créneau.

## Inventaire statique initial

Voir `docs/codeparsers/16-11-2026/language-corpus/01-inventaire-et-protocole.md` :
huit langages dans le routeur, API spécialisées distinctes. HTML est un stub
explicite ; Svelte initialise un parser sans langage puis appelle `parse().expect()`.
Ces limites restent ouvertes dans ce ticket, sans exécution ni correction annoncée.
Le protocole d'entrée est proposé à Banc avant implémentation de la CLI réservée.

## Source initiale publiée et acquisition disponible

Exemple codeparsers `be2ebe3` sur `cx-language-corpus-probe`, base bibliothèque
bd13385 inchangée ; aucun Cargo joué. Contrôleur et recette dans
`docs/codeparsers/16-11-2026/language-corpus/02-recette-premiere-campagne.md`.
Sept tests légers du contrôleur verts ; chemin canonique réel vérifié, archive
sans .git vue rouge puis acceptée avec vérification du gel, archive altérée refusée.
Première sélection Banc disponible : Go et C# 2500 contenus uniques chacun.
La compilation puis le parsing sous allocation fraîche restent à faire.
Enfant Svelte : `2026-11-16-le-parseur-svelte-n-initialise-pas-sa-grammaire.md`.
Coordination C7 : `2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md`
(commit e843a7a9ba3e0654510f9222773ffdea433bcff4).

## Rouge du pilote avant Cargo : contexte du lanceur

GO `C7-CORPUS-384-16nov`, source parent384165760b / parser1b35cfb,
FULL02 manifest96c2e771. Arrêt réel avant test/compilation/adoption :
`cache_quiet` trouve deux cwd source, PIDs2717971/2717972, starttime118716552.
Le shell SSH avait fait cd worktree et le pilote extérieur y restait ; contrairement
au pilote Rust précédent, le pilote corpus n'avait pas déplacé son cwd en preuves.
Aucun processus étranger ni utilisateur tué ; aucun retry.

Scope `validation-2h66zbqx` ROUGE/code1/erreur null/restes[]/nettoyé true,
privé `/tmp/r3v-djcyhfvm` supprimé. Target marker toujours Rust fix crate.
Preuves PC `~/.cache/codeparsers-notes/language-corpus-02/`, copie des artefacts
sans reçu/token sur portable au même chemin avant correction :
PQS SHA5367e169523bcd4fa948054aad23314d5e8ab097d4601565d20eef455541ee63,
stderr SHA8076f5015797022b61a6e3caf2a4147f4b22dbb7d30bebe32f3458f425955d96,
classification SHAce2ca87cd7c85f0e48aad93fe4fdf7b273f550291ebf9d8cf2bbf19c623d2bbe.
Correctif attendu : cwd du pilote hors source, commande SSH lancée hors source
également ; conserver le contrôle PQS strict, puis nouvelle source/profil/borne.

## Interprétation des compteurs et reclassement

Les premières lignes oracle C#/Go rapportent total_scopes=0 malgré5/75 scopes
récursifs effectivement rendus. Le banc conserve les deux, ne propage pas le zéro
comme absence de captures. Le contrat du compteur de fichier reste à qualifier ;
aucun correctif de compteurs pendant la campagne.
Banc fournit une annexe de reclassement indépendante (b33c7d5d2c),584 fichiers
upb générés explicitement,329C/255Cpp. Le résumé les sépare sans modifier le gel,
et ne prétend pas que tous les autres fichiers sont non générés.
`summarize_probe.py` vérifie ordre/identités et longueur contre sélection puis
calcule les statistiques à partir des seules sorties ; aucun parsing supplémentaire.


## Clôture de la première référence

Rapport et résultats : docs/codeparsers/16-11-2026/language-corpus/03-rapport-premiere-campagne.md,
e5e2d9cf5a. Scopecanonique02b VERT(code0/erreur null/restes[]/nettoyé), autorité
terminée, mais coverageunjudged/globalpassfalse. RésultatsSHA0667eab18d7892b442b8b912d98528d1253041620f43c60a286b47cd5fa81678
et ELF exactbff1cafb conservés aux deux postes, auditsC7/Banc également.
32382entrées :28115completed,1workerexit,5timeouts,4261refusHTML/Svelte.
105ASTinvalides sont des données ; les enfants conservent défauts de faits,
stack overflow et délais sans relance. Oracles avant résultats C7/Banc, jugement
109faits=80observés/2manquants/1nonexposé/26indéterminés, sans rappel global.

Suites ouvertes :

- qualifier-les-relations-sur-les-projets-du-corpus-public ;
- etendre-l-oracle-aux-facades-du-corpus-public ;
- le-parseur-svelte-n-initialise-pas-sa-grammaire ;
- un-worker-deborde-sa-pile-sur-un-fichier-openssl ;
- les-champs-des-methodes-csharp-confondent-noms-et-types ;
- cinq-fichiers-atteignent-la-borne-du-banc-public ;
- un-template-forward-cpp-contamine-le-namespace ;
- un-parametre-generique-rust-garde-sa-contrainte-dans-le-nom ;
- les-parametres-proprietes-typescript-manquent-aux-membres.

Les noms sont les tickets2026-11-16 de codeparsers ; le classement584générés a
son ticket Banc. Le correctif Rust7601 reste un lot séparé et non déclaré fini.
Cette clôture signifie livraison de la référence autorisée par C7, jamais une
qualification globale du résolveur ou des façades HTML/Svelte.
