# Un `using` C# ne produit aucune bibliothèque

- **État** : ouvert (14 novembre 2026) — une question de conception à trancher avant le code
- **Gravité** : manque de rappel : aucun espace de noms importé d'un fichier C# n'a de bibliothèque, donc
  ni provenance ni arête `USES_LIBRARY`, et les manifestes .NET lus depuis ce soir n'ont rien à relier
- **Atteignable en service** : oui, sur tout dépôt C# indexé
- **Touche rag3weaver** : oui (l'index de code ; aucune ligne de rag3weaver à changer)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au témoin de bout en bout du lot des manifestes .NET : l'analyse
  d'un `Lecteur.cs` qui fait `using Newtonsoft.Json.Linq;` et emploie `JObject` rend `libraries = []`

## Ce que c'est

`CSharpScopeExtractionParser` appelle `BaseScopeExtractionParser::extract_structured_imports`, dont les
expressions régulières sont celles de JavaScript, comme le faisait le parseur Go avant `6ba8478` : aucune
`ImportReference`, aucune bibliothèque.

## La question

Un `using Newtonsoft.Json.Linq;` ne lie **aucun nom** : il verse sans qualificatif tous les types de
l'espace dans la portée (`JObject`). Lire les `using_directive` sur l'arbre est simple ; savoir à quel
`using` rattacher un `JObject` employé ne l'est pas sans connaître le contenu du paquet. Trois voies :

1. **une bibliothèque par `using`**, sans symbole relié (la provenance seule ; `USES_LIBRARY` depuis la zone
   du fichier) — sûr, rien de deviné ;
2. **un nom inconnu du dépôt, employé dans un fichier qui a un seul `using` hors `System.*`**, rattaché à
   ce `using` — utile, mais une devinette quand il y en a deux ;
3. **les alias seuls** (`using J = Newtonsoft.Json.Linq.JObject;`, `using static …`) lient un nom ; le reste
   suit la voie 1.

La voie 1, plus 3 pour les alias, ne devine rien ; c'est la proposition.

## Ce qui est déjà prêt

La règle C# (`csharp.toml`, 417 espaces de noms tirés de la doc .NET), le plus long préfixe déclaré
(`Newtonsoft.Json.Linq` → le paquet `Newtonsoft.Json`, codeparsers `3474cea`), et la lecture des
manifestes .NET par rag3weaver (`.csproj`, `packages.config`, `Directory.Packages.props`).
