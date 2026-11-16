# Arrêt du pilote rouge FTS avant superviseur

Source tests `187d58a7b5ebb8e4a0a59b9b57ed75623e9db9a2`, pilote `fac5eeee11`, GO C7 rouge seul1200. Le lanceur s’arrête code1 avant toute entrée du superviseur commun :

```
Failed to start transient scope unit: Unit run-p3539890-i146086372.scope was already loaded or has a fragment file.
```

Le lanceur avait ajouté une portée systemd imbriquée pour HIGHmax ; son nom automatique entre en collision avec une portée existante. Aucune attribution à Lucivy/FTS ni au témoin : **rouge d’entrée, zéro compilation, zéro juge ; le rouge comportemental attendu reste NON OBSERVÉ**. Aucun retry ni correctif produit automatique.

Log brut conservé sur PC `~/.cache/rag3weaver-build/cx-launch-recherche-fts-existing-red-v1.log` et local `~/.cache/rag3weaver-build/cx-proofs-recherche-fts-existing-red-v1-local/cx-launch-recherche-fts-existing-red-v1.log`, SHA256 identique `189ee19f784f34e33952cf4ac10079270d810e4c5dd3a4d431893db48b3668a2`.

Contrôle après arrêt : target de session ABSENT, répertoires preuves et superviseur v1 ABSENTS (aucun reçu émis) ; checkout PC source187 propre. `poste --qui` ne montre plus ce pilote, conserve les deux autres lourds TSan/Archi. Leurs ressources n’ont pas été touchées. Aucun accès aux bases/service/modèle.

Suite à arbitrer par C7 après revue du lanceur : nom unique explicite de portée ou primitive HIGH existante, nouvelle entrée distincte seulement. La réserve Document physique malgré redirection publique reste à corriger dans les tests avant le correctif/full. Les cinq témoins demeurent NON JOUÉS ; aucune qualification verte.
