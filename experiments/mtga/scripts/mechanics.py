"""Extract Arena's tooltip glossary and evidence-based lexical card links."""
from collections import defaultdict
import re
import sqlite3
import unicodedata
from identity import stable_id

def normalize(text):
    return re.sub(r'[^\w]+', ' ', unicodedata.normalize('NFKC',text).casefold()).strip()

def extract(raw_dir, localizations, clean, ability_data=None):
    files = list(raw_dir.glob('Raw_ClientLocalization*.mtga'))
    if not files: return [], lambda texts, ability_ids=(): []
    f = max(files, key=lambda p:p.stat().st_mtime)
    db = sqlite3.connect(f.as_uri()+'?mode=ro',uri=True)
    db.row_factory = sqlite3.Row
    rows = list(db.execute("SELECT Key,enUS,frFR FROM Loc WHERE Key LIKE 'AbilityHanger/Keyword/%'"))
    db.close()
    groups = defaultdict(dict)
    for row in rows:
        tail = row['Key'].split('/')[-1]
        suffix = 'title' if tail.endswith('_Title') else 'body'
        base = re.sub(r'_(Title|Body)$','',tail)
        groups[base][suffix] = dict(row)
    # Reuse Arena translations of simple ability labels, e.g. Flying → Vol.
    names = {normalize(text): (text, localizations['frFR'].get(k) or text)
             for k,text in localizations['enUS'].items() if text and len(text) < 70 and '<' not in text}
    mechanics = []
    for key, group in sorted(groups.items()):
        title = group.get('title',{})
        body = group.get('body',{})
        inferred = re.sub(r'(?<=[a-z])(?=[A-Z])',' ',key).replace('_',' ')
        label_en = title.get('enUS') or inferred
        translated = names.get(normalize(label_en))
        if translated and not title.get('enUS'): label_en = translated[0]
        label_fr = title.get('frFR') or (translated[1] if translated else label_en)
        definition_en = clean(body.get('enUS')) or None
        definition_fr = clean(body.get('frFR')) or None
        aliases = [key, label_en, label_fr]
        if key == 'Descend': aliases += ['descended','descends','descent','descendez','descendu','descente']
        mechanics.append(dict(
            key=key, name_en=clean(label_en), name_fr=clean(label_fr), aliases=sorted(set(aliases)),
            definition_en=definition_en, definition_fr=definition_fr,
            definition_status='available' if definition_en or definition_fr else 'missing',
            has_placeholders=bool(re.search(r'\{(?![0-9WUBRGCXYZTQESP/]+\})[^}]+\}',(definition_en or '')+(definition_fr or ''))),
            source_kind='arena_client_tooltip', source_keys=sorted(r['Key'] for r in group.values()),
            rules_reference_url='https://magic.wizards.com/en/rules',
        ))
    # English rules text only, not names/flavor. Prefer longest labels when nested.
    lookup = defaultdict(list)
    for m in mechanics:
        labels = [m['name_en']]
        if m['key'] == 'Descend': labels += ['descended','descends']
        for label in labels:
            if len(label) >= 3: lookup[label.casefold()].append(m)
    pattern = re.compile(r'(?<!\w)('+'|'.join(re.escape(n) for n in sorted(lookup,key=len,reverse=True))+r')(?!\w)',re.I)
    # A parameterized keyword refers to a native base row (Flashback {3}{W} → 35).
    # Resolve only a unique whole base label and require the printed ability heading.
    base_lookup = defaultdict(list)
    for mechanic in mechanics:
        base_lookup[normalize(mechanic['name_en'])].append(mechanic)
    ability_data = ability_data or {}
    def link(texts, ability_ids=()):
        texts = list(texts)
        found = {}
        strength = {'text_mention':0,'ability_heading':1,'exact_ability':2}
        for text in texts:
            plain = clean(text)
            heading = re.match(r'^\s*<i>(.*?)</i>\s*(?:<nobr>)?\s*[—–]',text or '')
            for match in pattern.finditer(plain):
                for mechanic in lookup[match[0].casefold()]:
                    relation = 'text_mention'
                    if normalize(plain) == normalize(mechanic['name_en']): relation = 'exact_ability'
                    elif heading and normalize(clean(heading[1])) == normalize(mechanic['name_en']): relation='ability_heading'
                    # Descend N is a distinct ability word, not the act of descending.
                    if mechanic['key']=='Descend' and re.match(r'\s+\d',plain[match.end():]): continue
                    key = mechanic['key']
                    if key not in found or strength[relation] > strength[found[key]['relation']]:
                        found[key] = dict(mechanic_id=stable_id('mtga:mechanic:'+key),key=key,relation=relation,evidence=plain)
        for text, aid in zip(texts, ability_ids):
            row = ability_data.get(aid, {})
            base = ability_data.get(row.get('BaseId'), {})
            label = clean(localizations['enUS'].get(base.get('TextId'), ''))
            candidates = base_lookup.get(normalize(label), []) if label else []
            if len(candidates) != 1: continue
            # A reference inside a sentence is not possession of that keyword.
            plain = clean(text)
            if not re.match(r'^\s*'+re.escape(label)+r'(?:\s*$|\s*(?=[{0-9—–]))', plain, re.I): continue
            mechanic = candidates[0]
            found[mechanic['key']] = dict(mechanic_id=stable_id('mtga:mechanic:'+mechanic['key']),
                key=mechanic['key'], relation='native_base_ability', evidence=plain)
        return [found[k] for k in sorted(found)]
    return mechanics, link
