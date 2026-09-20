# Bonsai 2 27B : candidat pour l'agent local, à tester

Date : 20-09-2026. Demande de Lucie : conserver cette piste pour un essai sur son llama.cpp. Statut initial : repérage documentaire. Mise à jour le même jour : téléchargement, essais ROCm, comparaison agentique et mesures du cache réalisés ; voir [rapport expérimental 18](18-comparatif-local-bonsai-qwen-magic.md). Le llama.cpp installé reste inchangé.

## Pourquoi le tester

PrismML annonce Bonsai 2 27B, dérivé de Qwen3.8 27B, disponible depuis le 17 septembre 2026 sous Apache-2.0. L'annonce rapporte 83,9 de moyenne sur 20 benchmarks contre 85,4 pour sa référence pleine précision, soit 98,2 %, et jusqu'à 143 tokens/s sur RTX 5090. Ce sont les mesures du fournisseur, pas celles de notre environnement. [Annonce officielle](https://prismml.com/news/prismml-launches-bonsai-2-27b).

Le message transmis par Lucie avance également : 5,93 Go, 1,76 bit/poids, 26,2 M paramètres conservés à plus haute précision, 47 tokens/s sur M5 Max et 28 sur M5 Pro à environ 27 W ; scores maths 96,57 contre 97,06, code 81,58 contre 82,17, suivi des instructions 82,66 contre 81,25. À conserver comme chiffres annoncés à confronter au format exact, aux réglages et au protocole. Le rapprochement avec Qwen3.6 à 83,6 compare aussi deux générations de modèles, pas seulement deux précisions du même modèle.

Intérêt produit : un LLM local pour l'agent de recherche/construction de decks, branché par un endpoint compatible chat avec appels d'outils. Il ne remplace pas automatiquement le modèle d'embeddings du catalogue : les deux rôles restent configurables séparément.

## Points concrets vérifiés dans la fiche actuelle

La [fiche GGUF officielle](https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf) indique :

- PTQ1_0 : 5,95 Go ; PQ2_0 : 7,21 Go.
- Vision : composant `mmproj` optionnel supplémentaire, 0,63 Go en Q8_0 ou 0,93 Go en BF16. Le petit fichier du modèle de langage n'inclut donc pas à lui seul ce composant.
- Contexte annoncé jusqu'à 262 144 tokens ; il faut mesurer la mémoire d'exécution, pas seulement le fichier des poids.
- Ces fichiers Bonsai 2 requièrent le fork `PrismML-Eng/llama.cpp`. L'amont ne prend pas en charge leurs kernels ternaires/hybrid-attention ; certains formats peuvent même se charger avec une sortie incorrecte.
- Le dépôt de démonstration fait référence pour les commandes de lancement ; ne pas reprendre aveuglément les suggestions automatiques de l'interface Hugging Face.

Ne pas confondre le support amont des anciens Bonsai avec celui de Bonsai 2. La compatibilité du binaire déjà installé chez Lucie n'a pas été inspectée. [Démonstration officielle et backends](https://github.com/PrismML-Eng/Bonsai-demo), [fork llama.cpp](https://github.com/PrismML-Eng/llama.cpp).

## Protocole local proposé

1. Relever le commit, les options de compilation et les backends du llama.cpp actuel ; inventorier GPU, RAM et VRAM disponibles. Conserver l'installation existante.
2. Choisir un fichier précis et épingler sa révision/hash avec une version compatible du fork. Vérifier les kernels pour le GPU ciblé avant un gros téléchargement.
3. Démarrer sur un port distinct, en texte seul avec un contexte modeste, puis augmenter progressivement. Ajouter la vision dans une expérience séparée.
4. Mesurer temps de chargement, traitement du prompt, premier token, génération, RAM/VRAM maximales et stabilité. Comparer à contexte, longueur de réponse et backend identiques ; ne pas extrapoler un débit sur prompt court au contexte maximal.
5. Tester le protocole agent : appels d'outils structurés, JSON valide, appariement des identifiants d'appels/résultats, plusieurs tours, erreurs d'outils et annulation.
6. Tester le métier : chercher un effet par capacité puis retrouver les cartes, filtrer la possession, proposer une substitution dans un budget de jokers, expliquer une interaction avec les textes sources, produire un export Arena valide.
7. Tester un second domaine de démonstration (notes/documents) avec la même boucle d'agent et les mêmes interfaces TUI/web pour vérifier la généricité.
8. Comparer au provider habituel sur une petite liste de tâches figées : réussite complète, erreurs de règles, appels inutiles, coût/latence et mémoire. Ne pas conclure à la qualité de l'agent à partir de la moyenne de benchmarks seule.

## Critères d'intégration

- Le serveur répond au contrat du provider configurable sans cas spécial dans la boucle d'agent.
- Les outils sont réellement exécutés et leurs résultats utilisés ; aucun appel simplement imprimé dans la réponse finale.
- Les performances restent acceptables avec le catalogue chargé et les embeddings actifs sur la configuration choisie.
- La version testée, le modèle, le contexte, les réglages et les limites sont consignés avant d'en faire un choix proposé aux utilisateurs.

Prochaine action : banc local dédié. Ce document réserve l'essai ; il ne marque aucune de ces vérifications comme accomplie.

## Complément : raisonnement et vitesse

Relecture de la fiche officielle le 20-09-2026 : elle présente désormais une évaluation distincte en mode thinking sur 14 benchmarks, avec 84,78 pour Bonsai contre 86,32 pour la référence FP16. Ne pas mélanger cette table avec les 20 benchmarks de l'annonce ci-dessus. Le raisonnement est actif par défaut ; la fiche indique `xhigh` par défaut et `medium` pour des réponses plus courtes (`low` n'est pas pris en charge correctement).

La table actuelle mesure sur RTX 5090 : 129,9 tokens/s en PQ2_0 contre 120,5 en PTQ1_0 ; sur RTX 4090 : respectivement 81,2 et 91,1. Protocole : `llama-bench`, batch 1, profondeur 0, génération de 128 tokens, sans vision. Le fichier le plus compact n'est donc pas systématiquement le plus rapide. Les chiffres M5 Max et certains chiffres d'annonce correspondent à une version antérieure, signalée comme à remesurer.

Ces débits ne donnent pas à eux seuls une accélération relative au modèle actuellement servi chez Lucie : comparer sur la même machine le traitement du contexte, la génération et le temps total d'une tâche réussie, à budget de réflexion comparable. Aucun gain local encore mesuré. [Fiche et protocole officiels](https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf).
