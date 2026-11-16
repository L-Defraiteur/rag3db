# Basculer les trois bases de code de Lucie vers les générations

- **État** : ouvert (16 novembre 2026), préparation active ; autorisation Lucie relayée par orchestration, opérations après FINI de G et contrôles, aucune nouvelle permission routinière requise.
- **Famille** : produit
- **Gravité** : bascule des données partagées préparée et répétée, pas encore effectuée ; conserver l'intégrité, la recherche et le retour.
- **Atteignable en service** : oui (code-rag3db-depot au portable, code-principal et code-lourd à luciepc).
- **Touche rag3weaver** : non pour le code produit ; opérations et preuves privées de migration.
- **Pour** : arbre principal (migrations et contrôles), G (lib, hôtes, fermeture/réouverture des services), banc (oracle des copies finales).
- **Signalé par** : reprise autorisée par orchestration le 16 novembre, après l'audit RO du périmètre.

Plan maître : `docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md` ; recette des bases : `docs/arbre-principal/15-11-2026/02-la-fenetre-de-la-bascule.md` ; parité et tolérances : `docs/banc/16-11-2026/01-la-parite-de-la-migration.md`, §12.

Source audit927941a67a5805450d49f1e04b39fdd542a6059b ; lib préparée par G59ac9c86bc2b3529b590a7462ab7051e3ad26b8a et hôtes rebâtis927/Lucivy4.3.9. Attendre ses artefacts réellement vérifiés/FINI, jamais employer ancien3177/373b/794 par défaut. A1 suspendu, aucune modification produit simultanée.

Pour fermer : pour chaque base, identité exacte, snapshot cohérent et RETOUR conservés72h, maintain frais borné, deux sens verts, versions1b lots2000 puis générations0publiée/1lu, repassage terminé (dernière ligne du journal, pas busy), comptes visibles/mirroirs0/0/vecteurs et oracle contemporain jugés, refus ou migrations partielles conservés sans faux vert. Retour et données d'avant gardés, aucun retrait maintenant. Une base non traitée ou oracle incomplet reste explicitement non qualifié.

Rapport et recette d'exécution : `docs/arbre-principal/16-11-2026/03-preparation-bascule-bases.md`.

## Préflight outil RO — rouge vu avant correction

G a compilé read-only-query.cpp04a contre59ac et joué une fixture privée1nœud : BUILD_ID_ERROR Binder exception, CALLBUILD_ID sansRETURN n’est pas une standalone table function. Journal portable ~/.cache/optimiseur-notes/bascule-16nov-927/query-read.log. Aucune basevivante ouverte. Corriger la seule requête d’identité en CALL BUILD_ID() RETURN *, puis recompilation/témoin parG avant usage.

## Première maintenance réelle — arrêt HTTP 422 avant purge

Recette be91b5b3c742209cc59c2ce2b2ac29f9a97b9213, portable code-rag3db-depot : premier POST /backend/maintain avec table File et dry_run=true refusé HTTP422. Preuves privées : ~/.cache/rag3weaver-build/bascule-live-20261116/code-rag3db-depot/maintenance-01/{phase,verdict}.json. Aucun relevé accepté ni appel de purge, aucune migration hors ligne ; autres bases non rejouées. La quiescence du producteur hébergé reste NON ATTESTÉE.

Diagnostic RO : daemon_host.rs:1038 et 1865 au source927 acceptent précisément manifest/table/dry_run ; header X-Rag3weaver-Session conforme. HTTP422 indique une réponse opérationnelle refusée, sans identifier sa cause. Défaut de recette confirmé : host-phase.py:58–59 intercepte HTTPError puis abandonne son corps ; seule la valeur du statut est conservée. La cause hôte est donc NON ATTRIBUÉE, aucun défaut produit établi. G est sollicité pour préserver le journal existant sans rejouer la route. Le corps initial n’est pas reconstituable depuis les deux fichiers client ; ne pas présenter une future réponse comme sa preuve.

Correction nécessaire avant toute nouvelle recette : archiver statut et corps de refus, sans jeton ni headers d’authentification, avant de lever l’erreur ; témoin hermétique HTTP422 pour vérifier conservation et arrêt. Ce paragraphe est publié avant modification du lanceur. Premier rouge conservé, pas de retry automatique ni de fermeture/snapshot comme si maintenance terminée.

### Conservation corrigée, cause initiale toujours inconnue

Journal existant G conservé host-journal-after-maintenance-01.log SHA2563d58adecef03aaaad5afe07263732813aad7085d7a0e9e2eabd1b1ecb1dab997 ; aucune entrée dans la fenêtre, journalctl utilisateur également vide. Aucun corps initial retrouvé. /etat observé par G : ouvert, sans held_closed/refused ; ce n'est pas une preuve de quiescence.

Correction privée : capture du corps HTTPError en hexadécimal, plafonnée à1MiB avec indicateur de troncature et longueur lue, répertoire chmod0700 ; route/statut archivés avant exception, aucun header ni auth copié. Le témoin hermétique de conservation est vu rouge avant ajout de la primitive, puis neuf témoins passent ; contrôle AST du véritable handler. Aucun HTTP réel, Cargo, base ou service dans ces témoins. Ancien verdict422 intact, aucune cause hôte déduite.

Piste RO distincte signalée par C7 : fts_releve_a_blanc utilise les handles déjà ouverts, dont l'ouverture est paresseuse ; backend ouvert ne prouve pas handle FTS ouvert. Ce n'est pas l'attribution du refus initial. Toute sonde dry_run File seule doit avoir recette et arbitrage distincts ; aucun retry maintenance autorisé implicitement par ce correctif.

### Sonde distincte live-ids — résultat réel

Source9a8ea38ce3, autorisation C7 distincte : /backend/fts-live-ids SEUL/File, session fraîche, client45s sous confineur60s/poste32GiB. Verdict ROUGE validation-502hg_hl ; preuves ~/.cache/rag3weaver-build/bascule-live-20261116/code-rag3db-depot/diagnostic-01. Corps422 intégral capturé, non tronqué : « pas d'index plein texte ouvert pour « File » (ouverts : []) », ok=false/transient=false. Cela établit l'absence de handle ouvert AU MOMENT DE CETTE SONDE, pas l'absence d'index sur disque, ni le corps du premier refus. Aucun maintain, purge, compact, ensure/warmup, autre base ou fermeture lancé. Aucune qualification de quiescence du producteur hôte. Maintenance reste arrêtée ; suite à arbitrer avec G/C7 sur recette nouvelle.

### Suite autorisée C7 : inventaire sur copie privée froide

Arbre prépare outil de lecture fichiers (configuration racine/shards/génération, inventaire SHA et noms par index/cellule sans supposer default), fixtures absente/corrompue avec preuve d'absence de mutation, et sonde native readOnly59ac déjà qualifiée. Entrée : snapshot G prélib à froid, ensemble base+extensions+fts/checkpoints copié par reflink vers destination privée neuve ; snapshot G et vivant jamais ouverts en écrivain. Avant/après : SHA des fichiers, pas seulement tailles. Lecture native sur COPIE uniquement, mémoire1GiB et poste commun32GiB borné. Inventaire physique seul ne qualifie pas décodage Lucivy ; ouverture snapshot immutable4.3.9 distincte à qualifier, aucun fallbackcreate. Recherche porte primitive réutilisable existing-only dans ticket distinct avant code ; aucun produit modifié par cet inventaire. Hôte reste ouvert, maintenance STOP. Publication recette/pins/fixtures avant lecture réelle.
