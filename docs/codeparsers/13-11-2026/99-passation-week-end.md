# Passation codeparsers — week-end du 13 novembre 2026

## Instruction de reprise prioritaire

Lucie a demandé l'arrêt des nouveaux GO/calculs pour limite d'usage et une
passation immédiate. **Aucun ancien délai, créneau, budget, test ou campagne ne
se réactive à la lecture de ce fichier.** Ce rapport ne donne aucun tour master.
Aucun calcul lancé pour cette passation ; seulement inventaire Git/ressources,
écriture et publication sur branche non master. Les preuves et restes sont conservés.

Session codeparsers : 01a1cb5d-1698-7ca3-b94b-30fed1adefea.
Orchestration : 01a1cb5c-62b9-7ea0-8b7e-cf38398ece09.
Archi actuel : 01a1cb70-7ec9-7d53-a7d1-2a76313d457d.

## Livré, sans travail runtime en cours de ce fil

1. **Require/default exports** : master
   2e910930dbb9684fdfeb1f387fc2a430c9604880, producteur
   a730a9ba89ef233423bb7063f85181de26d98ce0. Dépendance/navigation vers symbole
   ou valeur exportée d'un module relatif malgré l'alias local ; instance vers
   zone d'export, jamais constructeur inventé ; ambigu sans guess. Contrat
   documenté et accepté ; cinq désaccords LSP stricts restent faux. Six nouveaux
   liens strictement corrects attribués, aucun score global mélangé.
2. **Prévention du nettoyage administratif** : master
   65aa01328456439ed0389e9c2a6a60e966c0ae23. Garde Python commune, archives
   fidèles préalables, rapport exclusif hors cible, partage toujours refusé.
   Lien source index120000 inchangé traité comme feuille sans visiter sa cible ;
   administrations séparées sous noms quelconques détectées. 19 témoins verts.
   Ne confondre livraison de la garde et intégrité historique rétablie.
3. **Qualification des membres et coordonnées** : master
   a4bf86c865eebd79e2f05425e1a4e5353c645a52. Sept fixtures/six langages,
   50 scopes, offsets UTF-8 validés. Sources/recettes/rapport dans
   `docs/codeparsers/12-11-2026/navigation-ir/`. Aucune modification de parser,
   schéma, runtime partagé ou gitlink dans ce soutien.
4. **Navigation Archi, reçue et distincte de notre preuve IR** : première tranche
   3edbb8475ace557b87316da6a269938055d23f9d ; clôture de tranche
   8f604328a432388a575592b58bf91908a03bd450. Archi a signalé lib2252/65ignorés,
   navigateurs40, lectures27/hôte13/PG36 et nettoyage deux postes. Ces validations
   sont celles d'Archi, pas des suites rejouées ici. Le ticket courant master
   annonce aussi grep enrichi d3f5a264d, qualifié sur6786c03b5 ; ne pas reprendre
   l'ancien état « grep pas livré » sans relire ce ticket.

## Branches, remotes et worktrees aux deux postes

Base master observée pour ce rapport :
`3a5080d93bdaeba05fd333b25bdd0565b49ad18b`.
Seul worktree créé pour cette passation, sur le portable :
`/home/lucied/git_workspaces/cx-codeparsers-passation-weekend`, branche
`cx-codeparsers-passation-weekend`, publiée sur origin avec ce rapport.
Le SHA de publication est transmis à l'orchestration ; lire la pointe de cette
branche pour l'obtenir, aucun push master n'est effectué par cette passation.

Branches distantes rag3db conservées et relues :

| Branche origin | SHA |
|---|---|
| cx-codeparsers-require-4 | 2e910930dbb9684fdfeb1f387fc2a430c9604880 |
| cx-codeparsers-garde-nettoyage | 65aa01328456439ed0389e9c2a6a60e966c0ae23 |
| cx-codeparsers-symboles-ir | a4bf86c865eebd79e2f05425e1a4e5353c645a52 |

Le sous-module a730a9b a été publié et vérifié accessible sur son propre origin,
branche cx-codeparsers-default-metadata, lors de la livraison require. Les deux
origins (rag3db/codeparsers) ne sont pas interchangeables.

Les worktrees de lot require, garde, qualification et leurs branches locales
ont déjà été retirés après contrôles aux deux postes ; **ne pas les recréer
pour terminer un nettoyage déjà fait**. Le target privé unique
`~/.cache/rag3weaver-build/target-codeparsers-clic` est confirmé absent à luciepc.
La base PostgreSQL privée cx_codeparsers_20261111_1698_test avait été supprimée
après la passe require. Aucun nouveau service ou base à arrêter pour ce fil.

Vestiges anciens à préserver, sans leur attribuer du WIP neuf :

- portable `/home/lucied/git_workspaces/rag3db-garde`, branche
  codeparsers/tickets-deplaces, pointe historique b81fed2d6, status propre relu ;
- luciepc `/home/lucied/git_workspaces/codeparsers-garde` : ancien garde devenu
  clone autonome après incident, état historique non certifié. **Aucune nouvelle
  intervention dans ce dépôt**, pas de reset/GC/prune/nettoyage ;
- autres branches historiques du dépôt : non auditées exhaustivement, ne pas
  les effacer ni pousser aveuglément. Aucun WIP possédé et non publié de ce fil
  n'a été identifié dans l'inventaire présent. La passation reste sur son W propre.

## Tickets et suites encore ouvertes

- Corrigé : `docs/tickets/closed/2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md`.
- **Incident historique ouvert** :
  `docs/codeparsers/tickets/2026-11-11-le-nettoyage-du-parent-invalide-un-worktree-de-sous-module.md`.
  Prévention livrée ; ancien index, références locales et reflogs exhaustifs
  non certifiés avant/après. Pas de preuve initiale de tous fichiers/writers.
  Un clone propre sur672a0d5 ne prouve pas la conservation de l'ancien état.
- **Oracle ouvert** :
  `docs/codeparsers/tickets/2026-11-11-l-oracle-cherche-la-cible-au-lieu-du-nom-mentionne.md`.
  La question doit porter sur la mention source, distincte de la cible ;
  UsageSite perd nom/colonne et la relation persistée garde min(lignes).
- **Homonyme externe ouvert** :
  `docs/codeparsers/tickets/2026-11-11-un-membre-externe-platform-se-lie-a-un-homonyme.md`.
  Faux connu ; politique externe non corrigée par le lot default exports.
- **Navigation/couverture ouverte**, porteur Archi avec appui IR :
  `docs/rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md`.
  Façade initiale intégrée ; suite relue dans son état courant, pas d'ancien lot
  réactivé. Positions de membres Python et omissions de couverture documentées
  dans la qualification, pas corrigées par cette branche documentaire.
- Chantiers antérieurs distincts, non lancés : import_binding dans
  `docs/codeparsers/tickets/2026-11-11-une-mention-d-un-import-es-externe-n-a-pas-de-cible-a-elle.md`,
  receveurs sans type dans le ticket2026-10-04, marques de résolution dans son
  ticket2026-10-04. Relire leur état, aucune nouvelle campagne implicite.

## Preuves, rouges et NON JOUÉ

Racine des preuves : `~/.cache/codeparsers-notes/`, surtout luciepc.

- Require : `require-default-v2-d96ed3264/` (deux bases A/B, binaires figés,
  TSV historiques, source/hash oracle) et `complement-23/` (RPC/positions complets,
  provenance, A/B, questions). Première A/B également conservée dans
  require-default-exports-e1015a379/. Ne pas mélanger les producteurs/versions.
- Passe intégrée : `cx-passe-require-3/` : cinq jeux features verts ; lib2114/0/64
  ignorés (39 repris par navigateurs), navigateurs39/0/0, code36/usages19/PG34
  sans ignoré. 25 autres ignorés lib **non validés**, noms conservés dans lib.log.
  Résidus e2e_code déclarés par le dépôt. 8 témoins ciblés et286 tests producteur
  avaient passé avant la passe. Rouges antérieurs et versions de correction conservés.
- Complément23 : six nouveaux liens strict-justes attribués ; cinq strict-faux
  de navigation alias/export conservés ; huit « pertes » expliquées par lignes
  agrégées. Reader/CSS non attribués exhaustivement. Déclaration LSP échoue
  -32601 sur23questions, non assimilée à une absence de réponse definition.
- Garde : `cx-garde-nettoyage/` : rouges et versions/test logs ; derniers
  tests-liens-administrations-separees.log (19/19), hashes correspondants.
  Faux VERT separate-git-dir observé en fixture puis corrigé ; aucun retrait réel
  n'a servi à découvrir ce défaut. Premier refus natif conservé.
- Portable `cx-garde-nettoyage-livraison-v2/` : archives fidèles, deux rapports
  selfguard VERT, contrôles push et nettoyage. Rapports SHA-256 e85242767f1d9f8e8de0d19f0bc4e2bb591f7bedc94d14c23cfb301089152f54
  et365988edcdd60c5214be48f92558fb8b73771f735a958f5d9638bd0c6c2f9981.
  Première livraison/refus et preuves incident préservés séparément ; ne pas les remplacer.
- Qualification IR luciepc : `navigation-ir-7a0e98b49/` (7JSON, build6,57s,
  fixtures.sha256) et `navigation-ir-3a10a493b/` (seul Rust changé, qualification).
  qualification.json SHA-256 db7e572c8e35c65b45062406ce89e14a97b608846da0aeadff131e478182b37b.
  Recettes publiées avant exécution. Projection code.rs **lue**, pas de nouvelle
  matérialisation de base ou test e2e navigation dans ce soutien.
- Livraison IR : `navigation-ir-livraison/` aux deux postes, archives/admins et
  rapports de garde neufs VERT ; W/branches/target retirés, remotes gardés.

**NON JOUÉ et non acquis** : nouvelle campagne LSP globale, capture de provenance
avant perte sur les23 (proposée/autorisée antérieurement mais non exécutée),
correction homonyme, comptage/activation import_binding, réparation exhaustive
historique, extension de couverture des six langages ou des treize au total.
Aucune ancienne borne600s ou fenêtre mesure ne repart automatiquement.

## Contrats, ressources et contraintes

- Contenu, identité, occurrence et contexte de résolution restent distincts.
  Plages scopes dérivées des lignes UTF-8, fin LF exclue, pas bornes AST exactes.
  compact/Item Rust ont même [429,463) ; égalité ambiguë. HAS_PARENT n'est pas
  une preuve lexicale ; Go hors classe et trois Box Rust ne se fusionnent pas.
  Lambda brute peut être repliée dans l'index ; ast_valid ne prouve pas exhaustivité.
- Python membres typés : lignes de classe dans la fixture ; propriétés sans
  type absentes soit du producteur soit de field_types. Pas d'UUID/type/position
  inventé pour les membres incomplets. Détails dans qualification.md.
- Inventaire luciepc à la passation : poste libre, zéro lourd/mesure déclaré,
  target session absent. Aucun PID/processus propre connu encore lancé ; pas
  d'inventaire exhaustif des processus d'autrui, ne rien tuer par nom.
- Privé7882 rendu puis arrêté par audit ; aucune réservation/client prévu
  codeparsers. 107m7880 froid, modèles partagés278m7878 etBGE7879 selon audit ;
  ne pas réactiver ni substituer modèle/dimension à un index sans besoin qualifié.
  Pas de nouveau modèle chargé pour cette passation.
- Card2 ressource normale, coordination capacité/VRAM ; card0 nocturne est une
  exception datée, **pas un droit reconduit au week-end**. Toute mesure requiert
  sa propre fenêtre. Pas de test/calcul à lancer maintenant.
- Tous nouveaux W/branches cx-, jamais écrire/commiter dans l'arbre de Lucie.
  Un seul target privé32Go/session, j8 max, poste lourd ; lib convenue épinglée
  explicitement (ancienne passe29a7 complète dans ses logs), jamais courant.
  Tout nouveau travail a son ticket et ses preuves ; master uniquement au tour
  donné par l'orchestration. Après rebase nom neuf, aucun force push.
- Git identité Lucie Defraiteur <luciedefraiteur@gmail.com>. Ne pas lire .env,
  afficher/copier .vault, ni retoucher les anciens restes forensiques.
- Toute suppression future : copie administrative fidèle neuve hors cible,
  garde livrée, rapport exclusif neuf, retrait seulement VERT et sans changement
  depuis copie. Aucun partage administratif autorisé même sauvegardé.

## Trois prochains pas, après consigne de reprise explicite

1. **Relire les états entrants et coordonner le périmètre**, sans lancer de suite :
   `git fetch origin master` puis `git show origin/master:docs/orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md`
   et le ticket navigation actuel. Contacter l'orchestration avant de choisir
   une suite ; la nuit passée n'accorde pas un nouveau lot au week-end.
2. **Choisir une seule suite de ticket autorisée**, par exemple provenance des
   occurrences ou couverture des membres, et publier plan/source/recette avant
   le moindre essai. Lecture des preuves existantes possible sans reparser :
   `git show origin/master:docs/codeparsers/12-11-2026/navigation-ir/qualification.md`.
   Le script qualifier.py exige un fichier sortie neuf ; ne pas le lancer
   automatiquement ni écraser les preuves. Ne pas commencer par reconstituer
   les refs/index historiques supposés.
3. **Si et seulement si un essai est demandé**, créer W neuf sur base convenue,
   vérifier ressources, lib/target, borne après acquisition et critères de fin.
   Commande de préparation documentaire :
   `git worktree add -b cx-codeparsers-suite-weekend <nouveau-chemin> origin/master`.
   Pas de cargo, LSP, GPU, réindexation ou restauration depuis ce rapport seul.
   Publier WIP/rouges, faire relire, attendre « à toi » avant master et utiliser
   la garde pour le nettoyage. Le W de cette passation reste conservé.
