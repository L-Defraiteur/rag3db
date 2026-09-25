#!/usr/bin/env python3
"""Build an independent SQLite snapshot and JSONL from local Arena data."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import html
import json
from pathlib import Path
import re
import sqlite3
import sys
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from identity import stable_id
from mechanics import extract as extract_mechanics
DATA = ROOT / 'data'
STEAM = Path.home() / '.local/share/Steam/steamapps'

def clean(text, name=''):
    text = text or ''
    text = re.sub(r'<br\s*/?>', '\n', text, flags=re.I)
    text = re.sub(r'<[^>]+>', '', text)
    text = html.unescape(text).replace('CARDNAME', name)
    # Arena's old-school mana notation. Preserve unknown tokens verbatim.
    text = re.sub(r'\{o([^}]+)\}', lambda m: 'o'+m[1], text)
    text = re.sub(r'o\(([A-Za-z0-9/]+)\)', lambda m: '{'+m[1]+'}', text)
    text = re.sub(r'o(\d+|[WUBRGCXYZTQESP])', lambda m: '{'+m[1]+'}', text)
    return text.strip()

def numbers(value):
    return [int(s) for s in str(value or '').split(',') if s.strip().isdigit()]

def build(raw_dir, log_path):
    DATA.mkdir(exist_ok=True)
    source_path = DATA / 'source.json'
    if not source_path.exists():
        source_path.write_text(json.dumps({'source_id': str(uuid4()), 'scope': 'local-arena-profile'}, indent=2))
    source_id = json.loads(source_path.read_text())['source_id']
    databases = list(raw_dir.glob('Raw_CardDatabase_*.mtga'))
    if not databases:
        raise SystemExit(f'Base Arena absente dans {raw_dir}')
    source = max(databases, key=lambda p: p.stat().st_mtime)
    src = sqlite3.connect(source.as_uri()+'?mode=ro', uri=True)
    src.row_factory = sqlite3.Row
    collection_path = DATA / 'collection.json'
    collection = json.loads(collection_path.read_text()) if collection_path.exists() else None
    owned = {int(c['grpId']): c['qty'] for c in collection['cards']} if collection else {}
    loc = {}
    for lang in ('enUS', 'frFR'):
        loc[lang] = {r['LocId']: r['Loc'] for r in src.execute(f'SELECT * FROM Localizations_{lang} WHERE Formatted=1')}
    enum = {(r['Type'], r['Value']): r['LocId'] for r in src.execute('SELECT * FROM Enums')}
    ability_data = {r['Id']:dict(r) for r in src.execute('SELECT * FROM Abilities')}
    mechanics, link_mechanics = extract_mechanics(raw_dir, loc, clean, ability_data)
    def localized(loc_id, lang):
        return loc[lang].get(loc_id) or loc['enUS'].get(loc_id) or ''
    def labels(value, kind, lang):
        return [clean(localized(enum.get((kind, n)), lang)) or str(n) for n in numbers(value)]
    cards = []
    issues = Counter()
    for row in src.execute('SELECT * FROM Cards ORDER BY GrpId'):
        r = dict(row)
        texts = []
        ability_refs = []
        for part in r['AbilityIds'].split(','):
            if not part: continue
            fields = part.split(':')
            if len(fields) == 2 and all(v.isdigit() for v in fields):
                texts.append(int(fields[1]))
                ability_refs.append((int(fields[0]),int(fields[1])))
            else:
                issues['unparsed_ability_reference'] += 1
        card = {
            'arena_id': r['GrpId'], 'owned': owned.get(r['GrpId'], 0) if collection else None,
            'mana_cost': clean(r['OldSchoolManaText']),
            'colors': labels(r['Colors'], 'Color', 'enUS'),
            'color_identity': labels(r['ColorIdentity'], 'Color', 'enUS'),
            'set': r['ExpansionCode'], 'collector_number': r['CollectorNumber'],
            'rarity_code': r['Rarity'], 'power': r['Power'], 'toughness': r['Toughness'],
            'is_token': bool(r['IsToken']), 'is_primary': bool(r['IsPrimaryCard']),
            'is_digital_only': bool(r['IsDigitalOnly']), 'is_rebalanced': bool(r['IsRebalanced']),
            'linked_faces': numbers(r['LinkedFaceGrpIds']),
            'rebalanced_arena_id': r['RebalancedCardGrpId'] or None,
            'alternate_deck_limit': r['AlternateDeckLimit'],
            'ability_references_raw': r['AbilityIds'],
        }
        for lang, suffix in [('enUS', 'en'), ('frFR', 'fr')]:
            name = clean(localized(r['TitleId'], lang))
            card['name_'+suffix] = name
            types = labels(r['Supertypes'], 'SuperType', lang) + labels(r['Types'], 'CardType', lang)
            subs = labels(r['Subtypes'], 'SubType', lang)
            card['type_'+suffix] = ' '.join(types) + (' — '+' '.join(subs) if subs else '')
            raw = '\n'.join(localized(t, lang) for t in texts)
            card['text_'+suffix] = clean(raw, name)
            card['text_raw_'+suffix] = raw
            issues['missing_ability_text_'+suffix] += sum(not localized(t, lang) for t in texts)
        card['abilities'] = []
        for aid, tid in ability_refs:
            a = ability_data.get(aid)
            if a is None: issues['missing_ability_record'] += 1
            links = link_mechanics([localized(tid,'enUS')], [aid])
            card['abilities'].append({
                'ability_id': aid, 'text_id': tid,
                'category_code': a['Category'] if a else None,
                'subcategory_code': a['SubCategory'] if a else None,
                'ability_word_code': a['AbilityWord'] if a else None,
                'base_ability_id': a['BaseId'] if a else None,
                'is_intrinsic': bool(a['IsIntrinsicAbility']) if a else None,
                'text_en': clean(localized(tid,'enUS'),card['name_en']),
                'text_fr': clean(localized(tid,'frFR'),card['name_fr']),
                'glossary_ids': [x['mechanic_id'] for x in links if x['relation'] in ('exact_ability','ability_heading','native_base_ability')],
            })
        links = link_mechanics([localized(t,'enUS') for t in texts], [aid for aid, _ in ability_refs])
        card['mechanics'] = [x for x in links if x['relation'] != 'text_mention']
        card['mechanic_mentions'] = [x for x in links if x['relation'] == 'text_mention']
        cards.append(card)
    src.close()
    by_id = {c['arena_id']: c for c in cards}
    start = None
    for line in log_path.read_text(errors='replace').splitlines():
        if not line.lstrip().startswith('{'): continue
        try: obj = json.loads(line)
        except ValueError: continue
        if isinstance(obj, dict) and 'DecksInternal' in obj: start = obj
    if not start: raise SystemExit('Pas de StartHook dans Player.log : relancer Arena avec les journaux détaillés.')
    summaries = {s['DeckIdInternal']: s for s in start['DeckSummaries']}
    if set(summaries) != set(start['DecksInternal']):
        raise SystemExit('Les résumés et contenus de decks ne correspondent pas.')
    decks = []
    for deck_id, piles in start['DecksInternal'].items():
        summary = summaries[deck_id]
        attributes = {a['name']: a['value'] for a in summary.get('Attributes', [])}
        entries = {}
        for pile, values in piles.items():
            if pile not in ('MainDeck', 'Sideboard', 'CommandZone', 'Companions'): continue
            entries[pile] = [dict(v, name_en=by_id.get(v['cardId'], {}).get('name_en'),
                                  name_fr=by_id.get(v['cardId'], {}).get('name_fr')) for v in values]
        decks.append({'id': deck_id, 'name': summary['Name'], 'format': attributes.get('Format'),
                      'attributes': attributes, 'piles': entries})
    deck_ids = {c['cardId'] for d in decks for p in d['piles'].values() for c in p}
    status = {
        'source_id': source_id,
        'built_at': datetime.now(timezone.utc).isoformat(), 'card_database': str(source),
        'deck_source': str(log_path), 'catalog_entries': len(cards), 'decks': len(decks),
        'collection_available': collection is not None,
        'collection_captured_at': collection.get('captured_at') if collection else None,
        'owned_entries': sum(q > 0 for q in owned.values()), 'owned_copies': sum(owned.values()),
        'unmatched_collection_ids': sorted(set(owned)-by_id.keys()),
        'unmatched_deck_ids': sorted(deck_ids-by_id.keys()), 'text_issues': dict(issues),
        'mechanics': len(mechanics),
        'mechanics_with_definition': sum(m['definition_status']=='available' for m in mechanics),
        'cards_with_mechanic_links': sum(bool(c['mechanics']) for c in cards),
        'limitations': ['Les formats enregistrés des decks ne prouvent pas leur légalité actuelle.',
                       'Textes issus du client Arena, avec balises nettoyées et originaux conservés.',
                       'Les variantes, réimpressions et cartes à plusieurs faces gardent leurs identifiants distincts.',
                       'owned=0 concerne cette impression ; la disponibilité totale par carte peut inclure des réimpressions.',
                       'Pas encore de validation des règles ni de génération de decks.'],
    }
    inventory = {k: v for k, v in start['InventoryInfo'].items() if k in (
        'Gems','Gold','WildCardCommons','WildCardUnCommons','WildCardRares','WildCardMythics','TotalVaultProgress')}
    for filename, obj in [('decks.json', decks), ('inventory.json', inventory), ('status.json', status)]:
        (DATA / filename).write_text(json.dumps(obj, ensure_ascii=False, indent=2))
    pending = DATA / 'arena.build.sqlite'
    if pending.exists(): pending.unlink()
    db = sqlite3.connect(pending)
    db.executescript('''
        CREATE TABLE cards(arena_id INTEGER PRIMARY KEY, owned INTEGER, name_en TEXT, name_fr TEXT, data TEXT NOT NULL);
        CREATE VIRTUAL TABLE cards_fts USING fts5(name_en, name_fr, type_en, type_fr, text_en, text_fr, tokenize='unicode61 remove_diacritics 2');
        CREATE TABLE decks(id TEXT PRIMARY KEY, name TEXT, format TEXT, data TEXT NOT NULL);
        CREATE TABLE metadata(key TEXT PRIMARY KEY, data TEXT NOT NULL);
        CREATE TABLE mechanics(key TEXT PRIMARY KEY, data TEXT NOT NULL);
        CREATE TABLE record_keys(id TEXT PRIMARY KEY, kind TEXT NOT NULL, native_id TEXT NOT NULL);
    ''')
    with (DATA / 'cards.jsonl').open('w') as full, (DATA / 'owned-cards.jsonl').open('w') as mine, (DATA / 'documents.jsonl').open('w') as docs:
        for c in cards:
            encoded = json.dumps(c, ensure_ascii=False)
            full.write(encoded+'\n')
            if c['owned']: mine.write(encoded+'\n')
            db.execute('INSERT INTO cards VALUES(?,?,?,?,?)', (c['arena_id'], c['owned'], c['name_en'], c['name_fr'], encoded))
            db.execute('INSERT INTO record_keys VALUES(?,?,?)',(stable_id(f"mtga:card:{c['arena_id']}"),'card',str(c['arena_id'])))
            if c['owned']:
                db.execute('INSERT INTO record_keys VALUES(?,?,?)',(stable_id(f"mtga:{source_id}:collection:{c['arena_id']}"),'collection_entry',str(c['arena_id'])))
            db.execute('INSERT INTO cards_fts(rowid,name_en,name_fr,type_en,type_fr,text_en,text_fr) VALUES(?,?,?,?,?,?,?)',
                       (c['arena_id'], *(c[k] for k in ['name_en','name_fr','type_en','type_fr','text_en','text_fr'])))
            document = {'id': f"arena:card:{c['arena_id']}", 'kind': 'card',
                'text': '\n'.join([c['name_fr']+' / '+c['name_en'], c['mana_cost'], c['type_fr'], c['text_fr'], c['type_en'], c['text_en']]),
                'metadata': {k: c[k] for k in ['arena_id','owned','set','is_rebalanced','linked_faces']}}
            docs.write(json.dumps(document, ensure_ascii=False)+'\n')
        for d in decks:
            db.execute('INSERT INTO record_keys VALUES(?,?,?)',(stable_id(f"mtga:{source_id}:deck:{d['id']}"),'deck',d['id']))
            db.execute('INSERT INTO decks VALUES(?,?,?,?)', (d['id'],d['name'],d['format'],json.dumps(d,ensure_ascii=False)))
            text = '\n'.join([d['name'], d['format'] or '', *[f"{pile}: {c['quantity']} {c['name_en'] or c['cardId']}" for pile, cs in d['piles'].items() for c in cs]])
            docs.write(json.dumps({'id': 'arena:deck:'+d['id'], 'kind': 'deck', 'text': text, 'metadata': {'deck_id': d['id']}},ensure_ascii=False)+'\n')
    for m in mechanics:
        db.execute('INSERT INTO mechanics VALUES(?,?)',(m['key'],json.dumps(m,ensure_ascii=False)))
        db.execute('INSERT INTO record_keys VALUES(?,?,?)',(stable_id('mtga:mechanic:'+m['key']),'mechanic',m['key']))
    (DATA / 'mechanics.json').write_text(json.dumps(mechanics,ensure_ascii=False,indent=2))
    db.execute('INSERT INTO metadata VALUES(?,?)', ('status',json.dumps(status)))
    db.execute('INSERT INTO metadata VALUES(?,?)', ('inventory',json.dumps(inventory)))
    db.commit()
    assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    db.close()
    pending.replace(DATA / 'arena.sqlite')
    print(json.dumps(status, ensure_ascii=False, indent=2))

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--raw-dir', type=Path, default=STEAM/'common/MTGA/MTGA_Data/Downloads/Raw')
    parser.add_argument('--log', type=Path, default=STEAM/'compatdata/2141910/pfx/drive_c/users/steamuser/AppData/LocalLow/Wizards Of The Coast/MTGA/Player.log')
    args = parser.parse_args()
    build(args.raw_dir, args.log)
