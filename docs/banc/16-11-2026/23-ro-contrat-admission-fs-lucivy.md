# Relecture secondaire du contrat d’admission FS Lucivy

Publication exacte `355ffadd01e9b227dac1a70492b58cf9f9bb21c2`, [contrat16](../../lucivy/16-11-2026/16-qualification-native-et-contrat-fs.md) et [ticket FS existant](../../lucivy/tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md), parents profil-core et indexed-false. **Avis secondaire favorable au contrat préparatoire, sans blocage supplémentaire identifié ; ne remplace pas la RO Recherche du consommateur.** Aucun code implémenté, test/CPU/Cargo, corpus même metadata, ELF, proc, DB ou service touché.

Les quatre fichiers SHA256SUMS du dossier native-preparation-1ee5 correspondent aux blobs publiés. Leurs profils/pins sont historiques, pas une nouvelle observation du corpus ou des ELF. Lecture Git du code1ee5 confirme les enchaînements décrits : le collector B est non trié et avale des erreurs ; v3_ground_truth_contains retourne si vide, crée le builder avant les requêtes et peut accepter une parité0=0. Ses oracles relisent le FS et ses chemins V3_INDEX_DIR réutilisent/persistent/suppriment ; ces comportements ne sont pas corrigés par les neuf mocks mémoire.

## Entrées et preuve observable

A reste le préfixe300 trié déjà fixé et connu sans mutex_lock : NON admissible pour la parité, pas une invitation à observer encore son refus prédit. Aucun nouveau runA ni permutation/extension autorisé. B possède quinze requêtes, pas les onze derived ; clone8507 n’est pas la vue ordonnée5000, laquelle reste non attestée. Aucun nouveau pin de vue n’est déduit du manifeste de tout l’arbre.

Le contrat ordonne correctement plan/politique/origine → contrôle des chemins → lecture au plus une fois/candidat et exactement une fois/retenu → SHA/UTF8 sur le même tampon → vue immuable → couverture du panel15 → manifeste durable → builder neuf → comparaison sur cette même vue. Ordinal/chemin/origine/SHA et identité complète de requête doivent rester liés ; deux occurrences à contenu égal ne fusionnent pas, et strict/relax restent distincts malgré le texte égal. Une seconde ouverture oracle détruirait cette garantie ; la référence-snapshot doit donc emprunter les cœurs purs déjà extraits plutôt que conserver les wrappers FS/lossy.

Les exclusions sont une décision avant lecture de contenu, pas une erreur de lecture renommée exclusion. Refus de lien/chemin sortant et de .env/.vault avant ouverture, provenance et erreurs Read/Utf8/Query/Uncovered explicites. Le report de qualification d’index ne doit pas être ouvert avant admission ; une preuve nouvelle de refus REFUSED peut en revanche être conservée comme le prévoit le contrat, sans troncature d’un ancien témoin.

## Quatre familles au vrai raccord

Les quatre vacuités visent bien la même entrée run_v3_contains_case que le consommateur natif, pas un helper parallèle. StdFs effectue les vraies opérations ; SpyStdFs les délègue sur petits fixtures et injecte l’erreur au même endroit, sans chmod censé échouer sous root.

- Vide : racine existante/vue retenue vide, refus EMPTY_VIEW observé au consommateur et zéro builder/compare/report d’index.
- IO : erreur enumerate/metadata/read avec d’autres fichiers valides disponibles ; publier l’opération et le chemin, aucun remplacement par le candidat suivant. Ces trois points doivent rester des souscas observables distincts si leur garantie est revendiquée sous une seule famille IO.
- UTF8 : fichier texte sélectionné invalide, erreur précise sur les octets empreintés, jamais contenu vide ni skip/lossy.
- Panel absent : vue valide non vide sans couverture, builder baseline atteint et comparaison0=0 qui ne doit pas devenir une réussite après correction ; toutes absences nommées avant builder.

Compagnons pertinents : positif FS séparé couvrant les quinze requêtes et passant une fois au builder ; read_count=1 par retenu ; mutation du backend après snapshot sans relecture ; manifeste ordonné publié avant builder ; pièges reuse/persist/delete à zéro. Un spy de lecture seul ne prouve pas l’absence des effets de l’ancien builder. L’extraction de sa branche fraîche sans capacités de chemin persistant est donc une vraie frontière, pas seulement une option désactivée.

L’extraction baseline devra conserver les quatre comportements défectueux avant publication des témoins rouges ; pas de faux rouge d’API inexistante ou compilation. Puis correction et vrais verts sous sources/GO neufs. Les signatures du contrat restent proposées, pas présentées comme du code déjà raccordé.

## Choix et coût restant avant code

Deux décisions annoncées doivent être figées AVANT contact corpus/résultat : transport du plan/manifeste et politique de capture. Un plan explicite rend l’ordre rejouable ; une première capture observée peut aussi fixer la vue si son ordre effectif et ses exclusions sont publiés une seule fois avant builder et conservés après refus. Le tri déterministe serait une politique nouvelle à approuver avant mesure, pas une réparation posthoc du vieux read_dir. V3_CORPUS seul ne transporte pas cette vue. Les flags hérités de reuse/relâchement et les overrides de panel doivent être refusés ou rendus inaccessibles dans cette entrée de correction ; pas transmis implicitement par l’environnement.

Coût bornable du raccord : adaptateur FS minimal, vue d’octets partagée, SHA sur le tampon déjà lu, admission des quinze requêtes, extraction builder frais/oracle-snapshot et quatre familles/spies. Coût de lecture/hachage proportionnel aux octets sélectionnés ; coût de panel dépend aussi des quinze parcours et de leurs modes, pas seulement du nombre de fichiers. Pas de nouvelle crate runtime/service ni code crypto artisanal nécessaire. La dépendance de tests SHA256 et son graphe offline doivent être épinglés à la future préparation ; la présence historique sha2 ne qualifie pas le cache présent.

Les300s dur/285validation et60s de petits fixtures ne sont que propositions à réserver après source/pilote/recette ; aucune durée native B ne peut être inférée de ces tests ou des anciennes compilations. Après modification, les ELF1ee historiques ne prouveront pas le nouveau raccord : nouveau source/ELF et recette native distincte. Si le noyau partagé des neuf mocks change, un éventuel rejeu sera justifié et explicitement alloué, pas automatique.

Conclusion préparatoire : les vacuités et vrais raccords sont explicitement visés, le coût reste celui d’un utilitaire de tests partagé et la vue observable évite de confondre arbre source et données indexées. Pas d’implémentation, témoin existant, couverture corpus, parité ou admissionFS déjà livrée déduits de cette RO. Source1ee/neuf mémoire verts restent leur périmètre historique ; core/9HOLD/7.2ULP/PR42 restent non qualifiés. Ticket FS et ses deux parents restent ouverts ; arbitrages C7 et RO Recherche avant étape suivante.
