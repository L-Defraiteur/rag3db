# Recette globale de l'union physique — publiée avant exécution

**NON JOUÉE.** Le ciblé privé P14 sura43c475f9 est vert ; l'union de ce code
avec masterf8cbd6a48 n'en hérite pas d'un verdict global. C7 autorise la préparation
et la sonde RO du service7879, pas encore cette exécution lourde. Le SHA40 final
de publication de cette recette doit recevoir le GO explicite avant entrée.

## Source et absence de perte au rebase

Branche NEUVE cx-arbre-raccord-physique-union-r1, worktree isolé
~/.cache/rag3weaver-build/cx-wt-raccord-physique-union. Premier SHA d'union publié
6b6f12d33283dfc02f746c53ce3b8a514645a736, base master
f8cbd6a48abac5ad563883689132d0f45621916f. Branche ciblée P14 et son WT conservés.
[Audit](raccord-physique/union-audit.json) et [range-diff](raccord-physique/union-range-diff.txt)
:22patches sous extension/rag3weaver TOUS égaux ; aucun conflit Rust. Fichiers du
lot hors chevauchement exactement conservés, fichiers master non chevauchants
exactement conservés (ED/Archi/garde inclus). Seul chevauchement : ticket parent,
repris canonique privé qui contient les ajouts master sans retrait. Les deltas
ultérieurs de cette préparation concernent docs et pilotes, pas ce code produit.

Le contrôle CPU des tickets a repéré puis corrigé le champ Atteignable du ticket
lecteur RO. Deux tickets restent ouverts, désormais marqués ciblé P14 qualifié
mais union globale/livraison attendues. Pas de test Cargo local.

## Commande, budget et ressources

À luciepc SEULEMENT, après GO source40exacte, depuis /var/tmp :

    bash /home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique-union/docs/arbre-principal/13-11-2026/raccord-physique/lancer-globale.sh SHA40

Le lanceur appelle le confineur commun suivi sur source, acquiert le poste réel,
pose UNE échéance6000s et UN bus privé pour toutes phases, Max32GiB/Highmax.
Le client et superviseur démarrent hors WT ; Cargo tourne dans le crate ; ELF
directs depuis /var/tmp avec CARGO_MANIFEST_DIR exact et exporteur natif à chaque
invocation. Premier rouge arrête l'arbre propre, conserve logs et fixtures ;
aucun retry automatique, reprise de budget ou prolongation. Les plafonds par
phase sont bornés à l'échéance globale absolue, attente publication comprise.

SDK29 identifiant complet habituel, Lucivy4.3.3 inchangé ; magasin libs explicitement
~/.cache/rag3db-libs, RAG3WEAVER_MOTEUR_ANCIEN=1 sauf complément CREATE exact.
-j8, BURN quatre rôles CPU, tampon natif explicite512MiB (pas minimum universel),
target UNIQUE EXISTANT cx-target-contenu-derive conservé, jamais nettoyé en lot,
refus au-delà32GiB. Aucun rebuild C++/SDK. Config/data/cache/state/journaux/TMP
et gabarits sont privés ; pas de .env/.vault lus/copiés ni base réelle ouverte.

Preuves neuves cx-passe-raccord-physique-union-v1 ; reçu neuf
cx-raccord-physique-global-confinement-v1. Chemins existants refusés, pas de reset.
Le finaliseur relit reçu VERT/code0/restes[], bus absent, scan indépendant du
contexte et réempreintes des4ELF copiés avant de composer un VERT. Il retire
uniquement son TMP navigateur marqué après quiescence ; pas le WT/target/PG.

## Phases dans cette unique portée

1. Source SHA40/propreté/absence rebase/retard, vrai cgroup et reçu, garde target,
   identité PG synthétique neuve (absence préalable, nom/OID/propriétaire/commentaire),
   DBus commun et paquets React présents. PG privée cx_phys_2191_20261113_u1,
   conteneur existant rag3weaver-pg : aucune autre base touchée. DSN de fixture
   déjà publiée lu depuis le pilote privé antérieur (aucun fichier de secrets).
2. check_features.sh réel **F8 8/8**, avec contrôles tickets/état, puis vrai minimal
   check --lib --no-default-features. Plafonds2100s puis120s.
3. Compilation canonique lib large, lib large+navigateur, e2e large/backend,
   total plafonné1500s. Même tag default=[] que run_e2e pour éviter un second ELF
   canonique implicite ; les8checks de features/minimal restent no-default.
4. Inventaires et ignorés AVANT runtime, source et223gabarits réels embarqués,
   SHA/paths/features issus messages Cargo (pas chemin ELF déduit). Dépôt du
   ledger inventaire-avant-runtime.json hors source. Le porteur publie son SHA
   par coordination puis écrit inventaire-publie avec CE SHA dans les preuves ;
   ce passage n'ouvre aucun nouveau budget.4ELF sont copiés avant runtime.
5. Lib large entière (plafond900s), inventaire exact jugé. Les20fonctions physiques
   et CPU requises ne sont pas ignorées ;7PHYSICAL_WITNESS et2BOOTSTRAP_WITNESS
   exigés,20brutes/19juges/8enfants réels. Les ignorés de la lib entière sont
   listés précisément, jamais comptés comme scénarios verts.
6. NAV40 sous son ELF inventorié, Chrome réel, bus commun, TMP btrfs privé marqué,
   plafond600s et quiescence immédiate. CLI migration_preflight5 sous e2e ELF,
   plafond180s. Aucun ancien lot ciblé1b/NAV hygiène rejoué séparément.
7. Le porteur coordonne le créneau RÉEL BGE7879, puis écrit creneau-7879=SHA40.
   Sonde /sante avant/après (modele bge-m3,1024dim, sparse vrai, non factice).
   Lectures27 brutes sous flagancien1 :26jugées +1non-jugée déclarée précisément.
   Complément CREATE1 exact même ELF/source/SDK29/profil contrôlé, SEUL flagancien
   absent ; primitives réelles d'état/résidus de run_e2e, aucun Cargo complément.
   Donc **27juges distincts /28invocations brutes**, filets du binaire séparés,
   aucune revendication format207701. Plafond180s chaque segment. Le créneau est
   rendu explicitement avant suite ; pas nouveau serveur/modèle/service lancé.
8. Hôte e2e_demon_hote13 (180s), PG36 (300s), quatre suites TX pertinentes
   (900s chacune bornées par échéance restante). Inventaires nonvides obligatoires,
   0ignoré pour suites jouées include-ignored, un appel run_e2e par suite,
   filets et hygiène TMP réels, ELF exécuté exactement celui du ledger.
9. Maps backend capturées à main via témoin d'inspection de stockage synthétique
   explicitement absent, aucun daemon/backend service démarré. Attestation finale
   4ELF/source/223gabarits/mapsSDK29/cgroups/CPU devices/événements mémoire, scan
   quiescence. OID/propriétaire/commentaire source réattestés avant dump ; dumpPG privé/OID/SHA gardés AVANT tout futur DROP,0session privée.
   La base reste pour revue ; retrait ultérieur coordonné seulement après preuves.

## Points de revue et limites

Le pilote [globale.py](raccord-physique/globale.py), son
[lanceur](raccord-physique/lancer-globale.sh), le
[complément CREATE](raccord-physique/create-exact.sh) et le
[finaliseur](raccord-physique/finaliser-globale.py) sont préparés avant GO. Syntaxes
Python/Bash et tickets peuvent être vérifiés CPU ; aucun résultat de ces contrôles
ne remplace F8/runtime. La sonde RO de préparation7879 a confirmé bge-m3/1024,
sparse/dual vrais/non-factice ; ce n'est ni réservation de créneau ni test de charge.

Aucun master push autorisé ici. Même une globale verte attendra revue des preuves,
source exacte, éventuels changements master, tour « à toi » de C7, poste push et
les trois contrôles. Aucune activation/migration d'une base réelle, GC ou couches
n'est incluse. Anciennes preuves rouges et P14 vert restent distinctes.


Revue RO ed13 par Mémoire/Recherche : favorable bornée au pilote, aucun runtime.
Avant gel, alignement de phase_caps sur lib900/nav600 réels et elapsed depuis
le début réel du confineur (deadline−6000), début pilote séparé. Reçu delai6000
asserté. Identité PG/OID/propriétaire/commentaire réattestée avant dump également ;
aucun retrait accordé. Contrôles syntaxe/tickets CPU à rejouer, sans Cargo.
