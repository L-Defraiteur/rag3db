# Orchestration — ce qu'il faut savoir

Mis à jour le 3 octobre 2026 à 23 h 35.

## Conduire les sessions

- **Un cadrage dit** : le but, ce qui est déjà établi (à vérifier à la
  source), le périmètre, ce qu'on ne touche pas, la preuve attendue, où
  s'arrêter. Proposition courte avant le code quand la forme n'est pas
  évidente.
- **Relire par git**, pas par le message. Dire à Lucie ce qui est vérifié et
  ce qui est seulement annoncé.
- **Une session s'arrête sans le dire** après un compte rendu : vérifier
  l'activité (fichiers touchés, processus) et relancer par un message.
- **Ne jamais faire faire par une session ce qui a été refusé à une autre** ;
  une autorisation se donne par Lucie dans la fenêtre de la session.
- **Fusion** : un lot fini et vert va sur master sans attendre ; lib rejouée
  après tout rebase qui amène du code ; jamais de push en force.
- **Tests** : proportionnés au changement ; batterie complète une fois avant
  la fusion d'un lot qui touche le cœur ; « 0 passed » fait échouer
  (`run_e2e.sh`) ; un binaire plus vieux que ses sources est refusé.
- **Le poste** : `-j8` par session ; `LUCIVY_SCHEDULER_THREADS=8` ; les passes
  qui embarquent passent par le service distant (jamais la carte d'ici, qui
  porte l'écran) ; rien de lourd dans `/tmp` (mémoire vive).
- **Sous-module** : après un rebase, `git submodule update --init` ; `git
  status` ne montre pas un pointeur en retard.

## Ce que la soirée a appris sur la preuve

- Un banc peut accuser le moteur à tort **et** l'innocenter à tort : isoler
  dans les deux sens.
- Un rouge qui ne ressemble pas à la question posée est un rouge du harnais.
- Une mesure sur un jeu qu'on a écrit soi-même se rejoue sur un jeu qui n'est
  pas de sa main avant de conclure (modèles de décision : rien ne tenait hors
  du jeu d'origine).
- Instrumenter jusqu'à ce que la somme des postes fasse le total, avant de
  corriger (les « 140 s » étaient les blobs, pas les points de reprise).
- Un repli silencieux cache un défaut : le compter, le dire, et un test qui
  échoue s'il se produit.
- Une vraie passe d'agent trouve ce que les scripts ne voient pas (état
  d'index faussé par le journal de conversation, contournement par
  `run_command`).

## Les amonts

Kuzu (origine du fork), Vela, et **Ladybug**, qui a livré des correctifs qui
nous manquaient : recherche par clé ligne par ligne sous `UNWIND` (juillet
2026), gardes des index non chargés (septembre 2026). Regarder Ladybug avant
d'inventer un correctif du moteur.

## Les services sur l'autre poste

Par tunnel ssh, liés à 127.0.0.1 là-bas : granite-278m, bge-m3, granite-107m
(ports 7979 à 7981 ici), le modèle de décision JevK5-4B (7982), un petit
modèle de langage Qwen2.5-7B (7983, arrêté quand il ne sert pas). Procédure :
`3-octobre-2026-14h26/02-le-service-d-embarquement-sur-l-autre-poste.md`.
On n'y commite ni n'y bâtit rien.

## Où sont les choses

- Registre commun : `docs/journal-des-chantiers.md`.
- Visions : `extension/rag3weaver/visions/2026-10-03-20h16-explorer-les-relations.md` et `extension/rag3weaver/visions/`.
- Verrous : `docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`.
- Indexer un dépôt, mesures : `extension/rag3weaver/docs/3-octobre-2026-14h26/03-indexer-ce-depot.md`.
- Passes d'agent : `extension/rag3weaver/docs/3-octobre-2026-22h40/`.
- Modèles de décision : `extension/rag3weaver/docs/optimiseur/3-octobre-2026-20h55/`.
