// archive_rapport — le rapport des décisions, tel que le ForEach du Decide le rend ; sous `trace: true`
// (il porte alors `results`), le total de l'usage du modèle sur la passe (`usage_total`, 15 novembre
// 2026 : le banc lit un total, pas une ligne par section). Une section sans compteurs (un modèle qui ne
// rend pas de bloc usage, ou un échec) compte dans `without_usage`, pas pour zéro.
let r = input;
if type_of(r.results) == "array" {
    let t = #{ calls: 0, without_usage: 0, prompt_tokens: 0, cached_prompt_tokens: 0, completion_tokens: 0, ms: 0 };
    for x in r.results {
        t.calls += 1;
        let u = if type_of(x) == "map" { x.usage } else { () };
        if u == () { t.without_usage += 1; continue; }
        if u.ms != () { t.ms += u.ms; }
        if u.prompt_tokens == () {
            t.without_usage += 1;
        } else {
            t.prompt_tokens += u.prompt_tokens;
            t.cached_prompt_tokens += u.cached_prompt_tokens;
            t.completion_tokens += u.completion_tokens;
        }
    }
    r.usage_total = t;
}
r
