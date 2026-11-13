# Passation week-end — embarquement, 13 novembre 2026

## Arrêt demandé et situation

Passation demandée par Lucie via l'orchestration : limite d'usage, aucun nouveau
GO ni calcul. Ce lot de passation est uniquement documentaire, poussé sur une
branche NON master. Les preuves, rouges, archives et références sont conservés.
Ne relancer aucun ancien script ou délai à partir de cette passation.

Le lot produit de cette session est **livré** : première activation dense locale
par `ModelSource` → `Serveur`/`Attache`, acquisition concurrente partagée,
annulation propre à chaque demandeur, identité de recette contrôlée et chauffe
par capacité. Appelant existant : `Backend::connect_embedder`. Politique initiale
`maintenir`, aucun TTL ou arrêt implicite. Activation distante, éviction, baux
distribués et reprise après crash restent **non livrés**, ticket ouvert ci-dessous.
Aucun redéploiement de service installé n'a accompagné cette livraison.

- Produit master : `23ceed87b80c58fb0070bf69cab8fa610d133ab3`.
- Bilan de livraison : `7308544ce12e008138b7002ff43ff013394f1cfe`.
- Correctif de garde intégré : `e167681867390914610cf3fe6e9560c6f3ca1f30`.
- Clôture effective du défaut de garde : `a1b5b73a1748482a5c2a1598e1b78cd48beda222`.
- Source d'union qualifiée avant rebase documentaire :
  `138268398ee27855260854e2d3a6328e3cf266f7`.

Le README et les rapports `01-rapport-de-session.md`, `02-knowledge-dump.md`,
ainsi que `../12-11-2026/03-activation-des-modeles.md`, détaillent le contrat.
Cette passation prime sur leurs anciens états de worktree « en cours ».

## Branches, WIP et arbres des deux postes

Worktree documentaire conservé sur le portable :
`/home/lucied/.cache/rag3weaver-build/cx-wt-embarquement-passation-week-end`.
Branche et destination : `cx-embarquement-passation-week-end-20261113`,
`origin/cx-embarquement-passation-week-end-20261113`.
Base observée avant rédaction : `3a5080d93bdaeba05fd333b25bdd0565b49ad18b`.
Le SHA final du rapport se lit sur cette branche distante ; il est également
transmis directement à l'orchestration après le push. Aucun target créé pour
cette branche. Aucun push de master par cette passation.

Ancien W produit `/home/lucied/git_workspaces/cx-activation-modeles` : **absent
sur portable et luciepc**, absences vérifiées. Target unique historique
`/home/lucied/.cache/rag3weaver-build/target-spans` : **absent sur les deux**.
Aucun nouveau W de passation n'a été créé à luciepc. Les autres worktrees,
anciens arbres historiques ou lots voisins ne sont pas à retirer par cette
session ; l'arbre de Lucie n'a reçu aucune édition ou commit de cette session.

Références distantes historiques conservées (SHA observés localement) :

| Référence sous origin/ | SHA complet |
|---|---|
| cx-activation-modeles | 45785b7ae387921eed76dc251301e70179b02a4a |
| cx-activation-modeles-2 | a1e22151cfe9e60fb36e6be7e5c6425921c39b69 |
| cx-activation-modeles-3 | 32471ec414b06997c6f6b47f9737bdf1a933ddc2 |
| cx-activation-modeles-4 | a5977c930ef28b87eafb32ad25f348b17d399ae1 |
| cx-activation-modeles-6 | 7308544ce12e008138b7002ff43ff013394f1cfe |
| cx-activation-retrait-garde-13nov | d823ec0918088b8f7040d622c89b2330897925c2 |
| cx-activation-cloture-garde-13nov | a1b5b73a1748482a5c2a1598e1b78cd48beda222 |

Restent des branches locales historiques : `cx-activation-modeles-3`, `-4` et
`-5`. Leurs commits ont tous une référence distante : contrôle
`git rev-list ... --not --remotes=origin` = **0** avant cette passation.
La branche locale5 était une tentative abandonnée, sans commit propre ni remote
à créer. Les branches locales de livraison/retrait ont été supprimées par `-d`,
jamais `-D`. **Aucun WIP de code non poussé détenu par cette session**.
Seuls le rapport et sa liaison au ticket constituent le WIP documentaire de fin.
Ne pas confondre les modifications de l'arbre de Lucie avec du WIP de cette session.

## Tickets

- **Ouvert**, porteur embarquement/ED/optimiseur selon les capacités :
  `docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md`.
  La tranche dense locale est livrée ; les politiques distante/éviction/baux
  distribués/crash restent à concevoir et qualifier, aucune reprise automatique.
- **Ouvert**, incident historique :
  `docs/embarquement/tickets/2026-11-08-granite-278m-a-luciepc-s-est-arrete-huit-fois-cause-illisible.md`.
  Cause non établie, attendre une récidive avec journal exploitable. La consigne
  historique prévoit une décision le 8 décembre si aucune récidive ; ne pas
  transformer l'absence de journal en diagnostic ou fermeture automatique.
- **Clos** : `docs/tickets/closed/2026-11-12-revoir-les-services-d-embarquement-residents.md`.
- **Clos** : `docs/tickets/closed/2026-11-13-la-garde-attribue-un-worktree-au-cwd-apres-deinit.md`.
- Le défaut de compilation daemon seul signalé à ED est désormais **clos** dans
  `docs/tickets/closed/2026-11-12-la-boite-ne-compile-pas-sous-daemon-seul.md`.
  Lire ce ticket actuel, ne pas reprendre son ancien statut ouvert.

Les relectures physiques externes sont rattachées au ticket d'Arbre
`docs/arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md`.
Cette session n'en porte ni WIP, ni recette active, ni autorisation de calcul.

## Preuves réellement obtenues

Preuves PC sous `~/.cache/embarquement-notes/activation/`, hors ancien target :

| Preuve commune | Résultat acquis |
|---|---|
| validation-jqwupheq | standalone17, connecteur5, chauffe1 ; tous passés sans ignoré |
| validation-bvdguacm | banc réel107m : demandes partagées, annulation indépendante, réattachement et reprise ; deux activations, chauffe16/16, vrais vecteurs384 ; résidence/calcul PCI04 observés |
| validation-tbwulyl4 | fournisseurs privés BGE/278, tests2/2 ; observateur préattaché continu,80 captures complètes ; deux processus avec résidence/calcul PCI04, aucun client PCI07 observé |
| validation-qdm6acnb | union navigation : F5/5, lib2258/0/65 structurels, NAV40/0/0 |
| validation-uu18g72q | union navigation : fournisseurs2/2 et physique2/2 |
| validation-twt84wn7 | sparse réel1/1, santé7879 avant/après inchangée |
| validation-5wmntc4y | union Pub/ED/activation : F5/5, lib2273/0/66 structurels, NAV40/0/0, transactions3/1/18/3 ;4 suites sur4, aucun ignoré dans ces suites ; code0/restes[]/bus nettoyé |

Les65/66 ignorés structurels de lib ne sont pas qualifiés. Inconnus `/proc`
nommés dans les observations, jamais convertis en zéro. Les latences du banc
107 ne sont pas une preuve de caches disque/GPU froids. Les profils et devices
déclarés dans la recette ne constituent pas une attestation physique.

**Rouges conservés** : concurrence initiale (deux départs au lieu d'un), sonde
lente et budget extrême, erreurs initiales de fixture HTTP, plafond target
32,75Gio avant nettoyage sélectif, observateur initial incomplet, et union
`validation-hex7oup7` : F5 verts puis lib2272/1/66, carte show_progress absente.
Ce dernier défaut était une course de fixture ; correctif ED08c/c44 avec vraie
barrière d'index, assertions de cartes conservées et cas terminé ajouté, validé
ensuite dans l'union complète. Aucun NAV/TX n'a été prétendu joué après le rouge.
Le ticket conserve les détails et les limites de chaque précontrôle.

**Non joués/non livrés** : éviction, TTL, activation distante, baux globaux,
récupération après crash de l'activation. Aucun nouveau modèle, GPU, API ou
compilation n'a été lancé par cette session pour les relectures ultérieures.
Les verts des autres sessions ne sont pas de nouvelles preuves d'embarquement.
Aucun verdict CI actuel n'est établi par cette passation.

## Archives et retrait achevé

Sources138 sur les deux postes :
`~/.cache/embarquement-notes/activation/sources-138268398.tar.gz`, SHA256
`937ed60424230e5fd8e1d5b6a9e4809e42c1ead70b8f07be383ab1f8cf00e5e2`.
Binaires mesurés PC dans `bin-345a402d0/`, `bin-356d7d927/` et
`bin-avant-rebase3/` ; empreintes et provenance conservées. Ne pas supprimer
ces artefacts, ni les anciens rouges avec les caches.

Portable : `archives-fin-portable/{garde.json,retrait.json}` et
`archives-cloture-docs/{garde.json,retrait.json}`.
PC définitif : `archives-fin-luciepc-e167-final/{administration-fidele,garde.json,
quiescence-avant.json,quiescence-apres.json,retrait.json}`. Verdict VERT/refus[].
Ancien rouge dans `archives-fin-luciepc/garde.json`, également gardé ; les
précontrôles e167/r2/r3 ne sont pas les reçus définitifs de retrait.

La garde corrigée intégrée e167, SHA256
`6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c`,
a été extraite hors cible et vérifiée. Après archive fidèle et nouvelle garde
verte, Git refusait encore le retrait normal à cause des métadonnées de
sous-module. L'orchestration a explicitement borné UN `worktree remove --force`
pour ce seul W, selon la procédure, sans contourner un refus de garde. Bornage
consigné AVANT action dans `bornage-retrait-e167.txt`. Aucun double force,
`branch -D`, changement core.worktree/cwd pour passer, ou rm manuel d'admin.
Aucune permission générale de retrait forcé n'en découle pour le futur.

## Ressources et état relu à la passation

- Aucun processus de validation ou target détenu par embarquement.
- Ancienne unité `cx-activation-union-13nov-2.service` : failed historique,
  MainPID0 ; unité `cx-activation-union-reprise-13nov.service` : inactive,
  MainPID0. Ne pas nettoyer ces états pour masquer les rouges.
- `poste --qui` sur luciepc : libre, zéro lourd/mesure au contrôle. Ce constat
  ponctuel ne réserve aucune ressource aux sessions voisines.
- Granite278 installé : active, PID966256, NRestarts4 ; BGE installé : active,
  PID3415586, NRestarts0. Aucun arrêt/redémarrage par ce lot. Les lectures
  coordonnées voisines ont rendu7879 ; aucun créneau en cours ou réservé ici.
- Les deux fixtures PG de l'union ont été supprimées ; contrôle après retrait :
  bases0/sessions0 (`cx_embar_union_20261113` et suffixe `_r2`).
- Fenêtre card0 nocturne expirée le13nov à07:54 CET, aucune reconduction
  implicite. Reste la coordination réelle card2/PCI04 pour tout futur usage.

## Relectures externes faites depuis la livraison

Lecture seule sur les branches Arbre, aucun changement ou import dans ce lot :
réserve7d62 sur sparse déclaré mais neutralisé sans dense ; levée à la lecture
be9699 par profil lexical strict qui refuse dense/sparse avant effets. Avis
progress3827 et10e6 : None n'invente ni nom, débit ou sonde ; Option du modèle
et JSONnull explicites, Some/Absent préservés. Avis40d36 sur tampon : validation
permis avant ouverture, politique commune et readonly/provenance conservées.
512Mio était un candidat de recette, pas un minimum prouvé ou un OOM machine.
Les dernières recettes voisines ont ensuite évolué ; les bilans actuels sont
ceux d'Arbre/C7, pas ces avis historiques limités. Leur état global n'a pas été
requalifié par cette session. Le créneau7879 voisin explicitement rendu à
08:00:56 locale n'est pas une permission d'usage toujours valable.

## Contraintes à reprendre avant tout travail

Français, tutoiement, faits prouvés. Jamais éditer/committer l'arbre de Lucie,
lire/copier `.env` ou `.vault`, toucher le travail ou les processus d'autrui.
Ticket avant nouvelle action transmise. Après rebase : branche neuve, pas de
force-push/amend publié ; chaque commit poussé sur sa branche dans la minute.
Identité git `Lucie Defraiteur <luciedefraiteur@gmail.com>`.

Reprise déclarative : contenu/identité/occurrence et politique/mécanisme
séparés ; composer les primitives existantes. Ne pas étendre la tranche locale
en affirmant que l'activation distante ou une migration physique est livrée.

**TOUS LES ANCIENS DÉLAIS SONT ÉCHUS ET NE SE RÉACTIVENT PAS**, notamment les
preuves4q, hex/5wm, scripts reprendre-union et deadlines voisines901710.
Aucun ancien jeton d'autorité, marker VERT, marker de créneau ou script de push
ne vaut autorisation de reprise. Une nouvelle tâche doit recevoir une recette,
source, budget et coordination réels ; Max32G/high0/j8/target unique/lib épinglée,
bus privé, nettoyage des descendants. Aucun nouveau calcul avant nouveau GO.
Master demande le tour explicite de l'orchestration ; cette passation n'en donne
aucun. Les preuves physiques antérieures ne se transportent qu'à composants
inchangés et après relecture des équivalences, jamais par simple ressemblance.

## Trois prochains pas et commandes de lecture

1. Lire cette passation, docs/README.md, les tickets et le bilan week-end C7.
   Vérifier la branche de passation et son état sans changer l'arbre de Lucie :

   ```bash
   git -C /home/lucied/.cache/rag3weaver-build/cx-wt-embarquement-passation-week-end status --short --branch
   git -C /home/lucied/.cache/rag3weaver-build/cx-wt-embarquement-passation-week-end log -1 --format='%H %s'
   git -C /home/lucied/.cache/rag3weaver-build/cx-wt-embarquement-passation-week-end show origin/cx-activation-modeles-6:docs/embarquement/12-11-2026/03-activation-des-modeles.md
   ```

2. Choisir avec C7 une suite explicitement autorisée du ticket activation ou
   l'observation de l'incident278. Relire l'état réel, sans lancer/arrêter de service :

   ```bash
   ssh luciepc 'systemctl --user show rag3weaver-embeddings@granite-278m.service rag3weaver-embeddings@bge-m3.service -p Id -p ActiveState -p MainPID -p NRestarts'
   ssh luciepc '/home/lucied/.cache/rag3weaver-build/poste --qui'
   ```

3. Seulement après ce choix et un nouveau GO, préparer une branche/worktree
   isolés depuis master courant, relire les contrats et produire une recette
   fraîche avant toute compilation ou mesure. Retrouver d'abord les preuves :

   ```bash
   ssh luciepc 'cat /home/lucied/.cache/embarquement-notes/activation/archives-fin-luciepc-e167-final/retrait.json'
   ssh luciepc 'sha256sum /home/lucied/.cache/embarquement-notes/activation/sources-138268398.tar.gz'
   ```

Aucun effacement supplémentaire demandé pour la passation : conserver ce W,
les branches, archives et preuves en l'état jusqu'à la reprise coordonnée.
