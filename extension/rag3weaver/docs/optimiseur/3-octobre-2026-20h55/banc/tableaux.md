# Exactitude du verdict (choix le plus probable)

## sujets : même / variante / sans rapport (42 paires, hasard 33 %)

| modèle | toutes | français | anglais | mixtes | confusions les plus fréquentes |
|---|---|---|---|---|---|
| laya | 45 % | 36 % | 43 % | 57 % | variant→unrelated ×12, same→unrelated ×5, variant→same ×3 |
| gliner | 31 % | 29 % | 29 % | 36 % | variant→same ×15, unrelated→same ×12, same→unrelated ×2 |
| jevk5 | 64 % | 57 % | 71 % | 64 % | variant→unrelated ×10, variant→same ×4, same→unrelated ×1 |

## mémoires : même / complète / contredit / neuve (20 paires, hasard 25 %)

| modèle | toutes | français | anglais | mixtes | confusions les plus fréquentes |
|---|---|---|---|---|---|
| laya | 60 % | 62 % | 43 % | 80 % | completes→new ×4, same→new ×2, same→contradicts ×1 |
| gliner | 15 % | 0 % | 14 % | 40 % | completes→new ×4, contradicts→new ×3, same→new ×2 |
| jevk5 | 85 % | 88 % | 86 % | 80 % | same→contradicts ×1, same→completes ×1, new→completes ×1 |

# Le score à critère fixe : un seuil unique sépare-t-il « même chose » du reste ?

## sujets : score = P(« same ») pour les modèles, score brut pour les références

| modèle | AUC | AUC fr | AUC en | AUC mixtes | vraies : min / médiane / max | autres : min / médiane / max | meilleur seuil unique (exactitude) | rappel sans aucune fausse fusion |
|---|---|---|---|---|---|---|---|---|
| laya | 0.77 | 0.62 | 0.80 | 0.96 | 0.03 / 0.51 / 0.72 | 0.00 / 0.19 / 0.60 | 0.34 (79 %) | 33 % (seuil > 0.60) |
| gliner | 0.60 | 0.69 | 0.80 | 0.27 | 0.28 / 0.59 / 0.78 | 0.45 / 0.57 / 0.65 | 0.62 (74 %) | 27 % (seuil > 0.65) |
| jevk5 | 0.98 | 0.87 | 1.00 | 1.00 | 0.37 / 0.71 / 0.89 | 0.06 / 0.23 / 0.50 | 0.43 (93 %) | 80 % (seuil > 0.50) |
| granite | 0.89 | 0.82 | 0.73 | 1.00 | 0.60 / 0.78 / 0.93 | 0.48 / 0.56 / 0.77 | 0.77 (83 %) | 53 % (seuil > 0.77) |
| reranker | 0.91 | 0.78 | 0.87 | 1.00 | 0.00 / 0.30 / 1.00 | 0.00 / 0.00 / 0.15 | 0.00 (86 %) | 60 % (seuil > 0.15) |

## memoires : score = P(« same ») pour les modèles, score brut pour les références

| modèle | AUC | AUC fr | AUC en | AUC mixtes | vraies : min / médiane / max | autres : min / médiane / max | meilleur seuil unique (exactitude) | rappel sans aucune fausse fusion |
|---|---|---|---|---|---|---|---|---|
| laya | 0.77 | 0.50 | 0.90 | 1.00 | 0.00 / 0.19 / 0.79 | 0.01 / 0.06 / 0.18 | 0.19 (90 %) | 60 % (seuil > 0.18) |
| gliner | 0.61 | 0.83 | 0.20 | 0.75 | 0.24 / 0.27 / 0.29 | 0.07 / 0.25 / 0.47 | 0.27 (70 %) | 0 % (seuil > 0.47) |
| jevk5 | 0.97 | 1.00 | 1.00 | 1.00 | 0.04 / 0.44 / 0.78 | 0.00 / 0.01 / 0.07 | 0.30 (95 %) | 80 % (seuil > 0.07) |
| granite | 0.71 | 0.75 | 0.50 | 1.00 | 0.66 / 0.81 / 0.95 | 0.50 / 0.75 / 0.87 | 0.95 (85 %) | 40 % (seuil > 0.87) |
| reranker | 0.83 | 0.75 | 0.80 | 1.00 | 0.01 / 0.87 / 1.00 | 0.00 / 0.01 / 0.96 | 0.87 (85 %) | 40 % (seuil > 0.96) |

## mémoires : ce que les scores des références valent par étiquette

| modèle | même | complète | contredit | neuve |
|---|---|---|---|---|
| granite | 0.83 | 0.74 | 0.85 | 0.55 |
| reranker | 0.61 | 0.45 | 0.12 | 0.00 |

# Calibration : l'assurance annoncée vaut-elle l'exactitude ?

| modèle | jeu | assurance moyenne | exactitude | écart de calibration (ECE, 5 bacs) | quand l'assurance dépasse 0,9 : cas, justes |
|---|---|---|---|---|---|
| laya | sujets | 0.69 | 45 % | 0.24 | 9, 4 |
| laya | memoires | 0.71 | 60 % | 0.22 | 5, 4 |
| gliner | sujets | 0.58 | 31 % | 0.27 | 0, 0 |
| gliner | memoires | 0.41 | 15 % | 0.26 | 0, 0 |
| jevk5 | sujets | 0.67 | 64 % | 0.11 | 2, 2 |
| jevk5 | memoires | 0.74 | 85 % | 0.11 | 6, 6 |

# Abstention : avec une option « unsure » en plus (sujets)

| modèle | fois où « unsure » est choisi | dont sur une paire que le modèle avait fausse sans elle | exactitude sur le reste |
|---|---|---|---|
| laya | 5 sur 42 | 2 | 43 % |
| gliner | 0 sur 42 | 0 | 33 % |
| jevk5 | 0 sur 42 | 0 | 64 % |

# Déterminisme et latence (CPU, huit fils)

| modèle | écart max entre deux passes | une décision à 3 options : médiane (max) | à 4 options | une décision à dix candidats : médiane (max) | bon candidat sur dix (6 cas) | premier appel |
|---|---|---|---|---|---|---|
| laya | 0.0e+00 | 54 ms (90 ms) | 56 ms | 63 ms (68 ms) | 6 sur 6 | 5.1 s |
| gliner | 0.0e+00 | 90 ms (123 ms) | 102 ms | 117 ms (130 ms) | 5 sur 6 | 0.2 s |
| jevk5 | 0.0e+00 | 1955 ms (2378 ms) | 2453 ms | 4405 ms (4896 ms) | 6 sur 6 | 2.0 s |
| granite | 0.0e+00 | 13 ms (17 ms) | 16 ms | 133 ms (152 ms) | 5 sur 5 (le cas « aucun » n'a pas de sens sans seuil) | 0.0 s |
| reranker | 0.0e+00 | 104 ms (162 ms) | 120 ms | 1060 ms (1251 ms) | 5 sur 5 (le cas « aucun » n'a pas de sens sans seuil) | 0.1 s |

# Détail par paire (sujets)

| paire | langue | attendu | laya | gliner | jevk5 | granite | reranker |
|---|---|---|---|---|---|---|---|
| s01 | fr | same | variant (0.38) | same (0.68) | same (0.63) | 0.81 | 0.92 |
| s02 | fr | same | unrelated (0.97) | same (0.58) | same (0.82) | 0.77 | 0.65 |
| s03 | fr | same | unrelated (0.95) | same (0.59) | same (0.43) | 0.71 | 0.01 |
| s04 | fr | same | same (0.51) | same (0.58) | unrelated (0.38) | 0.60 | 0.00 |
| s05 | fr | same | same (0.63) | unrelated (0.45) | same (0.46) | 0.64 | 0.00 |
| s06 | fr | variant | unrelated (0.74) | same (0.51) | unrelated (0.62) | 0.65 | 0.00 |
| s07 | fr | variant | unrelated (0.74) | same (0.46) | same (0.43) | 0.69 | 0.03 |
| s08 | fr | variant | unrelated (0.81) | same (0.53) | same (0.47) | 0.63 | 0.02 |
| s09 | fr | variant | same (0.60) | same (0.46) | unrelated (0.43) | 0.59 | 0.00 |
| s10 | fr | variant | unrelated (0.82) | same (0.45) | same (0.43) | 0.72 | 0.00 |
| s11 | fr | unrelated | same (0.44) | same (0.50) | unrelated (0.40) | 0.50 | 0.00 |
| s12 | fr | unrelated | unrelated (0.99) | same (0.59) | unrelated (0.89) | 0.49 | 0.00 |
| s13 | fr | unrelated | unrelated (0.98) | same (0.61) | unrelated (0.70) | 0.53 | 0.00 |
| s14 | fr | unrelated | unrelated (0.73) | same (0.50) | unrelated (0.82) | 0.52 | 0.00 |
| s15 | en | same | same (0.55) | same (0.78) | same (0.67) | 0.67 | 0.43 |
| s16 | en | same | unrelated (0.97) | same (0.67) | same (0.67) | 0.65 | 0.00 |
| s17 | en | same | same (0.63) | unrelated (0.61) | same (0.73) | 0.78 | 0.28 |
| s18 | en | same | same (0.59) | same (0.62) | same (0.71) | 0.64 | 0.02 |
| s19 | en | same | unrelated (0.45) | same (0.72) | same (0.70) | 0.78 | 0.43 |
| s20 | en | variant | unrelated (0.63) | same (0.59) | unrelated (0.77) | 0.66 | 0.00 |
| s21 | en | variant | unrelated (0.63) | same (0.56) | unrelated (0.52) | 0.77 | 0.11 |
| s22 | en | variant | same (0.37) | same (0.57) | variant (0.46) | 0.67 | 0.06 |
| s23 | en | variant | unrelated (0.92) | same (0.60) | unrelated (0.80) | 0.66 | 0.00 |
| s24 | en | variant | unrelated (0.44) | same (0.57) | unrelated (0.40) | 0.74 | 0.15 |
| s25 | en | unrelated | unrelated (0.70) | same (0.55) | unrelated (0.88) | 0.54 | 0.00 |
| s26 | en | unrelated | same (0.51) | same (0.59) | unrelated (0.90) | 0.48 | 0.00 |
| s27 | en | unrelated | unrelated (0.96) | same (0.59) | unrelated (0.88) | 0.53 | 0.00 |
| s28 | en | unrelated | unrelated (0.85) | same (0.61) | unrelated (0.93) | 0.57 | 0.00 |
| s29 | mixte | same | same (0.39) | same (0.52) | same (0.86) | 0.92 | 0.30 |
| s30 | mixte | same | same (0.72) | same (0.59) | same (0.75) | 0.81 | 0.06 |
| s31 | mixte | same | unrelated (0.54) | same (0.55) | same (0.77) | 0.89 | 0.97 |
| s32 | mixte | same | same (0.66) | same (0.55) | same (0.89) | 0.93 | 1.00 |
| s33 | mixte | same | same (0.65) | same (0.60) | same (0.82) | 0.89 | 0.65 |
| s34 | mixte | variant | unrelated (0.51) | same (0.56) | unrelated (0.53) | 0.69 | 0.00 |
| s35 | mixte | variant | unrelated (0.91) | same (0.65) | same (0.50) | 0.54 | 0.00 |
| s36 | mixte | variant | unrelated (0.48) | same (0.56) | unrelated (0.60) | 0.60 | 0.00 |
| s37 | mixte | variant | unrelated (0.85) | same (0.61) | unrelated (0.66) | 0.55 | 0.00 |
| s38 | mixte | variant | same (0.54) | same (0.60) | unrelated (0.65) | 0.56 | 0.00 |
| s39 | mixte | unrelated | unrelated (0.69) | same (0.54) | unrelated (0.41) | 0.55 | 0.00 |
| s40 | mixte | unrelated | unrelated (0.76) | same (0.59) | unrelated (0.80) | 0.52 | 0.00 |
| s41 | mixte | unrelated | unrelated (0.81) | same (0.62) | unrelated (0.91) | 0.52 | 0.00 |
| s42 | mixte | unrelated | unrelated (0.98) | same (0.58) | unrelated (0.88) | 0.56 | 0.00 |
