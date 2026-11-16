# La garde des comptes en blobs reste à302documents pour301lignes

- **État** : ouvert, canonique arrêtée au premier rouge, attribution NON ÉTABLIE.
- **Pour** : Recherche, revue Hotfixs/ED coordonnée C7 avant action.
- **Famille** : produit
- **Parent** : [maintenance générale](2026-11-16-la-maintenance-du-plein-texte-ne-peut-pas-ouvrir-un-index-existant.md), ouvert ; [lecture sans writer](2026-11-16-lire-un-index-plein-texte-existant-sans-writer.md), cible verte mais canonique non qualifiée.

## Preuve exacte avant tout correctif

Source union9576079d62c3f46bdf241cd19c9ab30e9218333e, pilote9a82d4c8d331096bc681aefdfc53c918e0f995d2, native59ac9c86bc2b3529b590a7462ab7051e3ad26b8a complète. Passe canonique autorisée C7-FTS-CANON-9a82-16nov.

PremierFAILED actif : `e2e_tx_par_paquet_arret::la_garde_des_comptes_retire_un_document_sans_ligne_en_blobs`, `tests/e2e_tx_par_paquet_arret.rs:1018` : `la garde a rebâti l'index : un document par ligne`, left302/right301. Firstred1181259.031107475 ; watcher-premiermatch1181259.028946841 sur inode165196878/dev55,19231octets lus, SHA7c4752e610869b1cdd51b7a897a3e2aed6fcaf126c22b45c78354c44c3194c48. Aucun rejeu/après-rouge runtime, aucune exclusion ni résultat final inventé pour cette suite interrompue.

Logs PC : `~/.cache/rag3weaver-build/cx/recherche-fts-existing-canon-v1/{first-red.json,watcher-first-match.json,official/e2e_tx_par_paquet_arret.log,official/resume.log}`. Scope validation-ajgm4kao sous cx-scope-recherche-fts-existing-canon-v1 : ROUGE1/«descendants laissés après la commande», puis restes[]/nettoye=true/autorité terminée/bus absent. Scan indépendant983identités PID/start : alive[]/unknown[]. Archives PC/local fidèles en préparation `cx-archive-recherche-fts-existing-canon-v1`, y compris préfixe exact de première lecture hashvérifié et ELF copiés APRÈSrouge (pas d’attestation avanttests).

## Diagnostic RO seulement — faits et questions

Le helper de test appelle `la_garde_retire_un_document_sans_ligne(false)` : mode **Blobs**, pas le cas Files. Il crée sa base privée, synchronise, vérifie documents==lignes avant fantôme, ajoute `_node_id=9000000` par index_document puis commit et vérifie documents=lignes+1. Une recherche du texte fantomatique est attendue sans ligne rendue. Après une seconde synchronisation, l’assert de comptes observée échoue ; les assertions suivantes et le nettoyage explicite de fin de helper ne sont pas atteints.

`documents_et_lignes` lit le handle courant par `fts_handle(index_du_code)` et compte les lignes du nom physique `index_du_code`. Ces comptes seuls ne donnent PAS bijection IDsFTS↔offsets graphe, ni preuve que le nouvel existing-only a été appelé, ni régression attribuée. La nouvelle primitive est un fallback Files pour live_ids/dry-run sans handle ; ce constat statique n’innocente aucun entrant à l’exécution.

À établir avant tout correctif : table/cellule physique réelle et mapping public de ce cas, vivantsIDs vs graphrows/IDs, générations et commits avant/après, handle courant/cache/snapshot vs éventuel fallback, chemin de garde/rebâti réellement emprunté en blobs. Comparer Files séparément sans transformer un autre cas en preuve de celui-ci. Pas de bug nouveau affirmé sur302/301seul.

C7 demande revue source/preuves Hotfixs/ED ; éventuelle nativeDBRO sur **COPIE privée seulement**, recette600 à publier puis autoriser distinctement. Aucun writer, warmup, purge, rebuild, réexécution de juge ou baseLucie ; pas de correction automatique. F8/preuve/lib/NAV et suiteTXvecteurs NON JOUÉS dans cette canonique arrêtée. Créneau services/PG rendu à Embar après quiescence+health, sources/targets/preuves conservés.
