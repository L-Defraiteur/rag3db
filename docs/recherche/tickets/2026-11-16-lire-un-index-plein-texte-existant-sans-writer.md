# Lire un index plein texte existant sans writer

- **État** : ouvert, qualification ciblée verte, canonique union à jouer.
- **Pour** : Recherche.
- **Famille** : produit
- **Parent** : [maintenance générale](2026-11-16-la-maintenance-du-plein-texte-ne-peut-pas-ouvrir-un-index-existant.md), qui reste ouvert pour l’exigence d’un writer de purge/compact.

Portée distincte : ouvrir en snapshot public read_only les fichiers existants de la table physique/cellule exacte pour live_ids et dry-run. Aucune redirection publique du chemin, aucun writer/create/rebuild/reprise, refus absent/corrompu/IO/métadonnées/génération/promesse/pending et recontrôle après relevé entier. Ce ticket ne livre pas la purge froide.

Source qualifiée04634cfed667711192efd313ff1418ef8dfd5c26 : deux rouges pertinents (ouverture187 et pendingc1b après schéma Content valide), puis dix juges distincts/onze invocations ciblées verts. Rapport11/preuves47a8,30fichiers PC/local SHA identiques manifestedf97717f2c944ccadca3246492a076db98a81d08886387d132d6bf526a38d2c9. Rouges d’entrée/setup conservés nonqualifiants.

Suite avant clôture : union neuve après landing EDties d395, audit produit strict et préservation master ; recette canonique complète code_sync/lectures/code/TX/F8/preuve/lib/NAV sous lib59ac exacte, pilote/budget publiés avant GO C7. Pas de master ni retrait sans ÀTOI. Coût RAM proportionnel aux fichiers et snapshot, pas de garantie de snapshot DB natif. Handles writers déjà montés suivent leur chemin historique.

Préflight recette avant tout run : `e2e_tx` n’est pas un module monté ; run_e2e filtre exactement `nom::`, donc cette étiquette ne doit pas servir de suite. Remplacer dans le pilote par les cinq modules TX réels (contrat_transactions, tx_ligne_a_ligne, tx_merge_puis_copy, tx_par_paquet_arret, tx_vecteurs_apres_rollback), chacun avec journal actif et inventaire source. Aucun test n’a été lancé sur l’étiquette courte, aucun rouge ni résultat inventé.

Réserves RO Mémoire sur pilote65b, consignées AVANT correctif : les preuves de l’ancien cache sont vérifiées avant attente poste mais doivent être recapturées/appariées APRÈS poste, juste avant garde (dev/ino/mode/uid T+marqueur, ancienW propre/head, reçu terminé/verdict/manifest/ledger/ELFhash). Snapshot après poste publié avant toute assertion, refus de la moindre divergence ; aucune reprise fictive. Final composé doit relire etat=termine puis contrôler fraîchement la deadline dans l’opération de publication, après tous les hashes/scans. Pilote65b retiré/nonlancé, prepare-only seul acquis ; nouveau contrôle Qv2 pour ne pas écraser sa préparation. SourceAPP957 figée, pas de Cargo/autorité/caps modifiés.

Canoniqueunion957/pilote9a82 arrêtée au premierrouge TXblobs302/301, [ticket diagnostic](2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md), rapport14/65preuvesPC-local0écart. F8/preuve/lib/NAV NONJOUÉS, donc fils présent encore OUVERT malgré les dix cibles vertes antérieures. Pas d’attribution de régression existing-only, pas de master ni retry ; parentmaintenanceOPEN.
