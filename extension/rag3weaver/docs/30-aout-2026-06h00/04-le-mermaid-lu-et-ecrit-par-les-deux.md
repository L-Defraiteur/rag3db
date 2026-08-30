# Le Mermaid, lu et écrit par les deux modèles

**30 août 2026.** Avant de convertir douze affichages faits à la main vers un
format commun, Lucie : *« testons le premier sur gemini, et pareil sur le llm
local, voir à quel point ils gèrent bien lecture/écriture de ce format »*.

On standardise **après** avoir vérifié que les consommateurs suivent.

## Le résultat

| | lecture | écriture |
|---|---|---|
| `gemini-3.5-flash` (Vertex) | **5/5** | ✓ 3 nœuds, 2 arêtes |
| `Qwen3-Coder-30B` Q6_K (local, HIP) | **5/5** | ✓ 3 nœuds, 2 arêtes |

Le format tient des deux côtés, y compris sur un modèle local de 24 Go qui
tourne sur une carte à côté. Le banc est `tests/e2e_lecture_mermaid.rs`.

## Ce qui rend la mesure honnête

- Les cinq questions ont des réponses **vérifiables dans le diagramme** —
  quelle relation va de A vers B, combien de cibles, laquelle porte tel champ,
  y a-t-il une arête sortante de F, laquelle boucle sur elle-même. La réponse
  est contrainte par un schéma JSON strict : on compare des chaînes, on ne
  juge pas une prose.
- L'écriture est validée par **notre propre parseur** (`parse_mermaid_template`).
  S'il relit ce que le modèle a écrit, c'est valide ; sinon, non. Aucun humain
  ne décide que « ça a l'air correct ».
- **Le test ne fixe pas de note.** Il mesure. Une note aurait été choisie après
  coup pour que ça passe.

## Ce que l'expérience a trouvé, et ce n'était pas ce qu'on cherchait

Au premier essai, **les deux modèles échouaient à l'écriture** — et ils avaient
raison. Gemini avait produit :

```
source["SearchSourceNode(target=Scope)"] -- results --> filtre["FilterNode(…)"]
```

C'est du Mermaid parfaitement valide. La langue accepte **deux orthographes**
pour une arête étiquetée — `a -->|p| b` et `a -- p --> b` — et notre parseur
n'en acceptait qu'une.

N'accepter qu'une des deux formes d'une langue qu'on n'a pas inventée, c'est
demander à l'autre de deviner laquelle. Le défaut était chez nous.

Et un second est tombé derrière : les modèles déclarent leurs nœuds **dans la
ligne d'arête**, ce qui est la forme courante, et nous n'y cherchions que les
arêtes. Résultat avant correction : deux arêtes, **zéro nœud** — un graphe qui
parsait et ne se construisait pas.

## Ce que ça valide, et ce que ça ne valide pas

**Validé** : le Mermaid comme format de sortie pour ce qui a la forme d'un
graphe. Les deux modèles le lisent sans erreur et le produisent correctement.

**Pas validé, et il ne faut pas l'étendre sans mesurer** : le Mermaid pour ce
qui n'est *pas* un graphe. Une liste de fichiers, un résultat de commande, un
diff — les convertir en diagramme serait forcer la forme. La standardisation
qui reste à faire porte sur le **mécanisme** (un gabarit, pas un `format!`),
pas sur le **format** (Mermaid partout).

Les douze affichages restants doivent donc passer par `rendre` et un gabarit
nommé — et chacun choisit sa forme : Mermaid pour un graphe, un tableau pour
une liste, du texte pour une sortie de commande.
