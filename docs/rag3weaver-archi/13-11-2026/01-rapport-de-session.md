# Archi — 13 novembre2026 : contexte optionnel du grep

## État courant

Ticket : docs/rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md.
Suite autorisée par Lucie/C7, Recherche confirme aucun chevauchement. WT neuf
~/.cache/rag3weaver-build/cx-wt-grep-symboles aux deux postes, branche
cx-archi-grep-symboles-2, target unique PC cx-archi-target, scratch court cx-gs.
Les biens du premier lot ont été retirés ; le target de session est neuf.
Base798ab1aa7 initiale, rebase sur garde intégrée e16768186, Embar inclus.

Source ea7dcbb0b4bb15f89c6f8d7c3a7a2f542afee428 : ciblé VERT12/0/0 navigation
en1,14s et2/0/0 grep en0,07s. Nouveau test : vrai chat du framework piloté par
CallbackLlm de fixture, appel réel de grep_files avec option, index réel et relais
atelier ; puis fichier modifié sans montre et match conservé avec stale_index.
Ce n'est pas un appel à un fournisseur de modèle extérieur.

PreparedNavigation prépare fraîcheur et parents une fois par fichier. Le grep
réutilise le Plan déclaré symbol_navigation.json et le contenu déjà observé.
Option symbol_hierarchy_depth0 : sorties et lectures habituelles ; positif≤16 :
contenant minimal sans choix arbitraire puis au plus depth parents. UTF-8/CRLF,
plages égales/croisées, index absent/périmé et troncature restent explicites.
Budget partagé JSON/Markdown96Kio,8fichiers interrogés,200nœuds/candidats cumulés ;
les comptes totaux ne sont pas tronqués. Couverture partielle et plages de lignes
ne garantissent ni parent AST, ni appartenance déclarée, ni héritage.

## Preuves

Cache des deux postes : cx-grep-symboles-ea7dcbb0b-cible-20261113.
ELF d5b8dd4ca105a0588315f2916869921de0f6abb57627233d3a28b38e2b509fc9,
223gabarits/aucun absent ; PID2370583/start87775220/native29/Max32 observés.
Primitive validation-1_rx2ina VERT0/restes[]/nettoyé=true, quiescence target[].
Le même budget initial1800s a couvert compilation et corrections de fixture.
Les rouges restent conservés : dea typeString/MockLlm, d752 champ de manifeste
catch_up_on_open placé sous workspace au lieu de embeddings. Les assertions et
le runtime n'ont pas été modifiés par ces corrections.

## Reprendre

Ne pas nettoyer ce nouveau lot avant revue/livraison. Le premier lot symboles
(master3edbb8475 puis notes8f604328a) est terminé/nettoyé, ne pas recréer ses biens.
Revue statique favorable C7 sur dea, ciblé correctif vert sur ea7 ; aucune globale
ni master de la suite grep autorisés à ce stade. Relire entrants ED features et
garde Recherche avant recette globale ; source/pilote publiés avant exécution.
Pas de modèle/service/7879/GPU utilisé pour le ciblé, aucun daemon personnel posé.

## Globale autorisée, avant entrée

C7 donne GO6000 après intégration garde Recherche e16768186 (SHA6f3dfceb…).
Branche neuve cx-archi-grep-symboles-3 ; diff ea7→HEAD src/templates/Cargo vide.
Le ciblé n'est pas rejoué sur MD/scripts seuls. ED n'a pas de correctif Rust
features disponible : cette passe ne qualifie pas daemon seul ni native+daemon
sans code, qui restent son lot indépendant.

Une primitive/budget6000, CPU/j8/Max32G/target32Go, native29 fixe, source et
pilote publiés avant entrée. F5≤2100, minimal0≤120, trois compilations sous
borne partagée1500, inventaires et attendus publiés avant lib≤900, NAV40≤600,
puis lectures27/hôte13/PG36 et filets2 chacun≤120. Modèles/API retirés ;
seul BGE7879 pour lectures après coordination, santé avant/après et rendu rapide.
PG neuve privée cx_archi_grep_symboles_20261113 : absence vérifiée avant création,
identité/propriétaire/OID conservés ; dump+SHA et zéro session avant futur drop.
Première erreur inattendue : stop/diagnostic, pas de retry inchangé ni de reset6000.
Le pilote prouve trois ELF réellement observés/native29 et quiescence finale.
Aucun tour master accordé à ce stade.
