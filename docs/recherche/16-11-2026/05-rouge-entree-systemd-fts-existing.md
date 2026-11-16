# Arrêt du pilote rouge FTS avant superviseur

Source tests `187d58a7b5ebb8e4a0a59b9b57ed75623e9db9a2`, pilote `fac5eeee11`, GO C7 rouge seul1200. Le lanceur s’arrête code1 avant toute entrée du superviseur commun :

```
Failed to start transient scope unit: Unit run-p3539890-i146086372.scope was already loaded or has a fragment file.
```

Le lanceur avait ajouté une portée systemd imbriquée pour HIGHmax ; son nom automatique entre en collision avec une portée existante. Aucune attribution à Lucivy/FTS ni au témoin : **rouge d’entrée, zéro compilation, zéro juge ; le rouge comportemental attendu reste NON OBSERVÉ**. Aucun retry ni correctif produit automatique.

Log brut conservé sur PC `~/.cache/rag3weaver-build/cx-launch-recherche-fts-existing-red-v1.log` et local `~/.cache/rag3weaver-build/cx-proofs-recherche-fts-existing-red-v1-local/cx-launch-recherche-fts-existing-red-v1.log`, SHA256 identique `189ee19f784f34e33952cf4ac10079270d810e4c5dd3a4d431893db48b3668a2`.

Contrôle après arrêt : target de session ABSENT, répertoires preuves et superviseur v1 ABSENTS (aucun reçu émis) ; checkout PC source187 propre. `poste --qui` ne montre plus ce pilote, conserve les deux autres lourds TSan/Archi. Leurs ressources n’ont pas été touchées. Aucun accès aux bases/service/modèle.

Suite à arbitrer par C7 après revue du lanceur : nom unique explicite de portée ou primitive HIGH existante, nouvelle entrée distincte seulement. La réserve Document physique malgré redirection publique reste à corriger dans les tests avant le correctif/full. Les cinq témoins demeurent NON JOUÉS ; aucune qualification verte.

## Correction du lanceur avant nouvelle admission

RO C7 : `poste` crée déjà sa portée via systemd-run ; réexécuter systemd-run sans unité distincte réutilise ici le même PID et son nom automatique. La couche imbriquée est supprimée, sans masquer le défaut par un nouveau nom.

Code poste PC/cache et source audité : `MEM=${POSTE_MEM_MAX:-40G}`, `SWAP=${POSTE_SWAP_MAX:-4G}`, puis propriétés MemoryMax, MemorySwapMax et CPUQuota à l’unique portée. Le lanceur pose donc POSTE_MEM_MAX=32G, POSTE_SWAP_MAX=0, POSTE_CPU_QUOTA=800% AVANT poste ; défaut HIGH illimité non abaissé. Le driver recontrôle fraîchement les fichiers cgroup réels (max32/highmax/swap0/quota<=8) et archive ces quatre valeurs. Une propriété manquante refuse avant compilation. Aucun marqueur ou reçu inventé, superviseur commun inchangé.

Témoins hermétiques ajoutés avant essai : le vrai launch.sh exécuté contre un faux poste enregistre exactement un argv poste→python common lancer1200 ; l’attestation réelle extraite AST accepte le profil attendu et refuse chacun des cinq écarts. Ces témoins n’appellent ni systemd, poste réel, Cargo, signal ou /proc réel. Source tests187 inchangée ; aucun nouveau GO déduit.

Publication du correctif+témoins AVANT exécution : `78437ce0a6`. Puis `python3 -B .../test_launch.py` : **2 fonctions vertes**, 7 cas (argv + profil valide + cinq refus), 0.019s. Aucun appel au vrai poste/systemd/Cargo ; aucun relancement natif. Source produit et pilote inchangés depuis cette publication ; cette ligne rapporte uniquement les petits témoins.

Préflight avant deuxième entrée : le superviseur commun vérifie son argv comme `python3 confiner_validation.py lancer` ; retirer `-B` devant ce script préserve cette signature existante. `-B` reste autorisé devant le driver seulement. Témoin argv exact adapté avant essai, aucune entrée réelle effectuée avec la forme précédente. Aucun changement de budget/profil/source.

Repassage hermétique avant entrée v2 : code `3dbcef4d40a520c0fc1a16750f01d4338507290a`, 2 fonctions/7cas VERTS en0.017s, profil et signature common inchangés ; source187 toujours figée. GO distinct C7 reçu pour1200 après poste ; preuves/control v2 neufs, aucun écrasement v1.
