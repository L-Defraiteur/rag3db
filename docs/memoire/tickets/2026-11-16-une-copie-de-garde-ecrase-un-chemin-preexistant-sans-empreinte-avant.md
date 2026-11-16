# Une copie de garde écrase un chemin préexistant sans empreinte avant

- **État** : ouvert — ancien SHA du chemin exact inconnu ; aucune réparation autorisée.
- **Famille** : produit.
- **Gravité** : incident opératoire sur une copie commune préexistante, provenance antérieure non établie.
- **Atteignable en service** : non.
- **Touche rag3weaver** : non, uniquement une copie de garde dans le cache PC.
- **Pour** : mémoire pour les traces en lecture seule ; C7 pour l'arbitrage du chemin partagé.

## Incident et preuve originale

Pendant le [nettoyage des fixtures](2026-11-16-retirer-les-ressources-privees-des-fixtures-apres-atterrissage.md),
un premier scp a visé le cache commun PC au lieu du sous-dossier privé prévu.
Chemin préexistant écrasé :
`/home/lucied/.cache/rag3weaver-build/garde_worktree.py`.
**Ancien SHA INCONNU ; aucune identité avant/après n'est affirmée.**

Appel outil original conservé, pas recréé : session mémoire
`01a1cb5c-4f39-7421-9a9f-4aa288d69402`, journal
`~/.codex/sessions/2026/11/11/rollout-2026-11-11T17-06-26-01a1cb5c-4f39-7421-9a9f-4aa288d69402.jsonl`,
ligne11953 à **2026-11-16T15:39:27.979Z** (16:39:27 heure locale),
call_id `call_50fa8786a61e4990816573dd58b40e7c`.
Commande scp effectivement journalisée :

```bash
scp -q /home/lucied/.cache/rag3weaver-build/archives/cx-memoire-script-fixtures-cleanup-16nov-v1/{prepare.py,garde_worktree.py} luciepc:/home/lucied/.cache/rag3weaver-build/
```

Ligne originale SHA256 **c8dda2fbc28b8532465c76dc0771636a24b05b0d5fbbb64cd4210ebaedaee704**.
Retour outil original ligne11956 à15:39:28.297Z, exit0, SHA256
**d72d9685c69e86539885af10e2bd38869e24c02dcbc99ed36b0bb698f99ccc8e**.

## Métadonnées réellement observées

Garde commune actuelle : fichier régulier0644, UID/GID1000/1000,19608octets,
birthtime **2026-11-15 16:31:20.741484038 +0100** ; mtime/ctime
**2026-11-16 16:39:28.280481193 +0100** ; SHA256 actuel
**6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c**.
Ce SHA correspond à la garde courante, pas à une preuve de son état préincident.
La copie commune n'a pas été exécutée par ce nettoyage et n'a pas été supprimée.
Toutes ses opérations de garde utilisent exclusivement la copie privée vérifiée.

Le second fichier `prepare.py` était neuf : birthtime16:39:28.153108808,
mtime16:39:28.163304280, régulier0644 UID/GID1000/1000,2907octets,
SHA256 **dfadec8877556f7e6ea735bdd50849dae46996178eb9f11b715377c2113ba850**.
Il a été déplacé à16:46:41 dans
`~/.cache/rag3weaver-build/archives/cx-memoire-script-fixtures-cleanup-16nov-v1/initial-misplaced-prepare.py`,
avant le message C7 de gel ; il reste conservé aux deux postes. Ce déplacement
n'est pas une suppression de preuve. Aucun nouveau déplacement ou effacement suit le gel.

## Recherche bornée et copies

Recherche exacte du chemin avant15:39:27.979Z dans les journaux mémoire et C7 :
zéro occurrence antérieure. Recherche dans journaux*/archives/archives-worktrees
et dossiers contenantc7 aux deux postes, texte<=4MiB sans tar/gz/pack ni chemins
protégés : seule occurrence postincident dans le JSON d'incident connu.
**Aucun SHA préincident établi par cette recherche bornée** ; ce n'est pas une
preuve d'absence dans toutes les archives possibles, ni un motif de recréer une preuve.

Huit fichiers de preuves originales/métadonnées/périmètre de recherche,
répliqués et réempreintés sansécart aux deux postes dans
`~/.cache/rag3weaver-build/archives/cx-memoire-cache-copy-incident-16nov-r1/`.
Manifeste extérieur suffixe `-manifest.json`, SHA256
**0a15e39b8052cb7cb5584d2073e9dadd81cfdf8b52d4ca6b5d7d29f95aa0d61f**.
L'ancien manifeste de nettoyage363entrées reste intact et distinct.

## Suite et critères de fin

STOP écritures cache commun, aucune restauration/suppression par intuition.
Le chemin garde préexistant reste conservé. Toute trace supplémentaire doit
être préexistante et attribuée au **même chemin PC exact avant l'incident**.
Une décision C7 documentée est nécessaire pour terminer le traitement du chemin
partagé ; une égalité avec la source du dépôt ne suffit pas à clore l'incident.
