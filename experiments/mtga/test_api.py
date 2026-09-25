"""Integration checks against the real, locally captured snapshot."""
import json
import unittest
from uuid import UUID
from fastapi.testclient import TestClient
from app import app, connect, metadata, snapshot
from contract import card_id, CardRecord
from records import make_record

class SnapshotAPI(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.client = TestClient(app)
        cls.status = cls.client.get('/v1/status').json()

    def test_collection_pagination_matches_capture(self):
        ids, copies, offset = set(), 0, 0
        while offset is not None:
            response = self.client.get('/v1/collection', params={'limit':500,'offset':offset,'snapshot_id':self.status['snapshot_id']})
            self.assertEqual(response.status_code, 200, response.text[:200])
            body = response.json()
            for c in body['items']:
                self.assertNotIn(c['arena_id'], ids)
                ids.add(c['arena_id'])
                self.assertGreater(c['owned'], 0)
                self.assertTrue(c['name_en'])
                copies += c['owned']
            offset = body['next_offset']
        self.assertEqual(len(ids), self.status['owned_entries'])
        self.assertEqual(copies, self.status['owned_copies'])

    def test_known_card_and_search(self):
        response = self.client.get('/v1/cards/93771')
        self.assertEqual(response.status_code, 200)
        c = response.json()['card']
        self.assertEqual(c['name_en'], 'Bloodthirsty Conqueror')
        self.assertIn('points de vie', c['text_fr'])
        self.assertEqual(c['mana_cost'], '{3}{B}{B}')
        self.assertNotIn('<i>', c['text_fr'])
        result = self.client.get('/v1/cards', params={'q':'conquerant assoiffe'}).json()
        self.assertIn(93771, [c['arena_id'] for c in result['items']])

    def test_deck_names_details_and_card_texts(self):
        result = self.client.get('/v1/decks', params={'name':'vampire'}).json()
        self.assertGreater(result['total'], 0)
        response = self.client.get('/v1/decks/'+result['items'][0]['id'])
        self.assertEqual(response.status_code, 200, response.text[:200])
        d = response.json()['deck']
        self.assertGreater(len(d['piles']['MainDeck']), 0)
        for entries in d['piles'].values():
            for e in entries:
                self.assertEqual(e['cardId'], e['card']['arena_id'])
                self.assertTrue(e['card']['name_en'])

    def test_documents_and_export_have_same_contract(self):
        p = self.client.get('/v1/documents', params={'dataset':'decks','limit':500}).json()
        r = self.client.get('/v1/exports/decks.jsonl')
        self.assertEqual(r.status_code, 200)
        self.assertIn('application/x-ndjson', r.headers['content-type'])
        docs = [json.loads(line) for line in r.text.splitlines()]
        self.assertEqual(docs, p['items'])
        self.assertEqual(len(docs), self.status['decks'])
        self.assertEqual(len({d['id'] for d in docs}), len(docs))
        cards = self.client.get('/v1/documents', params={'dataset':'collection','limit':1}).json()
        self.assertEqual(cards['total'], self.status['owned_entries'])
        self.assertTrue(cards['items'][0]['text'])
        self.assertGreater(cards['items'][0]['metadata']['owned'], 0)

    def test_errors_and_snapshot_consistency(self):
        for path in ['/v1/cards/999999999', '/v1/decks/absent']:
            self.assertEqual(self.client.get(path).status_code, 404)
        for path in ['/v1/collection','/v1/cards','/v1/decks','/v1/documents','/v1/exports/collection.jsonl']:
            self.assertEqual(self.client.get(path,params={'snapshot_id':'obsolete'}).status_code, 409)
        self.assertEqual(self.client.get('/v1/collection?limit=0').status_code, 422)
        self.assertEqual(self.client.get('/v1/collection?offset=-1').status_code, 422)
        self.assertEqual(self.client.get('/v1/exports/invalid.jsonl').status_code, 422)
        for q in ['" OR *', "'; DROP TABLE cards;--", '***']:
            self.assertEqual(self.client.get('/v1/cards',params={'q':q}).status_code, 200)
        schema = self.client.get('/openapi.json').json()
        self.assertIn('/v1/collection', schema['paths'])
        self.assertEqual(self.client.get('/docs').status_code, 200)

    def test_schema_has_nested_types_and_enums(self):
        r = self.client.get('/v1/schema')
        self.assertEqual(r.status_code,200)
        schema = r.json()
        self.assertFalse(schema['datasets']['boosters']['available'])
        fields = {f['path']:f for f in schema['payloads']['card']['fields']}
        self.assertEqual(fields['colors']['items']['type'],'enum')
        self.assertIn('Black',fields['colors']['items']['values'])
        self.assertEqual(fields['power']['type'],'keyword')
        self.assertEqual(fields['power_numeric']['type'],'integer')
        self.assertTrue(fields['power_numeric']['nullable'])
        abilities = {f['path']:f for f in fields['abilities']['items']['fields']}
        self.assertEqual(abilities['abilities[].ability_id']['type'],'integer')
        self.assertEqual(abilities['abilities[].text_fr']['type'],'text')

    def test_native_abilities_are_not_substring_guesses(self):
        r = self.client.get('/v1/records/'+card_id(93771))
        self.assertEqual(r.status_code,200,r.text[:300])
        c = r.json()['payload']
        self.assertEqual([a['ability_id'] for a in c['abilities']],[8,1,19060])
        self.assertEqual(c['abilities'][0]['text_fr'],'Vol')
        self.assertTrue(c['abilities'][0]['glossary_ids'])
        for a in c['abilities']:
            for ref in a['glossary_ids']:
                target = self.client.get('/v1/records/'+ref)
                self.assertEqual(target.status_code,200)
                self.assertEqual(target.json()['kind'],'mechanic')
        with connect() as db:
            conditional = next(json.loads(r['data']) for r in db.execute('SELECT data FROM cards')
                               if any('has flying as long as' in a['text_en'] for a in json.loads(r['data'])['abilities']))
        matching = [a for a in conditional['abilities'] if 'has flying as long as' in a['text_en']]
        flying = self.client.get('/v1/mechanics/Flying').json()['id']
        self.assertTrue(all(flying not in a['glossary_ids'] for a in matching))

    def test_parameterized_keywords_use_native_base_not_substrings(self):
        c = self.client.get('/v1/cards/86649').json()['card']
        flashback = self.client.get('/v1/mechanics/Flashback').json()['id']
        ability = next(a for a in c['abilities'] if a['base_ability_id']==35)
        self.assertIn(flashback, ability['glossary_ids'])
        self.assertTrue(any(m['key']=='Flashback' and m['relation']=='native_base_ability' for m in c['mechanics']))
        with connect() as db:
            matches=[]
            for row in db.execute('SELECT data FROM cards WHERE owned>0'):
                card=json.loads(row['data'])
                if card['is_primary'] and set(card['color_identity']) <= {'Black','Green'} and any(m['key']=='Flashback' for m in card['mechanics']):
                    matches.append(card['name_en'])
                # Qualified hexproof is not unconditional hexproof.
                for a in card['abilities']:
                    if a['text_en'].startswith('Hexproof from'):
                        self.assertNotIn(self.client.get('/v1/mechanics/Hexproof').json()['id'], a['glossary_ids'])
            self.assertIn("Dryad's Revival",matches)
            self.assertIn('Gnaw to the Bone',matches)
        self.assertNotIn(self.client.get('/v1/mechanics/Flash').json()['id'], ability['glossary_ids'])

    def test_structured_records_join_by_stable_uuid(self):
        p = self.client.get('/v1/records?dataset=collection&limit=2').json()
        self.assertEqual(p['total'],self.status['owned_entries'])
        for entry in p['items']:
            self.assertEqual(UUID(entry['id']).version,5)
            target = self.client.get('/v1/records/'+entry['payload']['card_id'])
            self.assertEqual(target.status_code,200,target.text[:300])
            card = target.json()
            self.assertEqual(card['payload']['arena_id'],entry['payload']['arena_id'])
            self.assertNotIn('owned',card['payload'])
            self.assertIsInstance(entry['payload']['quantity'],int)
        deck = self.client.get('/v1/records?dataset=decks&limit=1').json()['items'][0]
        for e in deck['payload']['entries']:
            self.assertEqual(self.client.get('/v1/records/'+e['card_id']).status_code,200)
        self.assertEqual(self.client.get('/v1/records?dataset=boosters').status_code,409)

    def test_mechanics_definitions_and_diagnostics(self):
        for key,fragment in [('Flying','bloquée'),('Hexproof','cible'),('Descend','cimetière')]:
            r = self.client.get('/v1/mechanics/'+key)
            self.assertEqual(r.status_code,200)
            self.assertIn(fragment,r.json()['payload']['definition_fr'])
        self.assertGreater(self.client.get('/v1/mechanics?q=DEFENSE%20TALISMANIQUE').json()['total'],0)
        self.assertGreater(len(self.client.get('/v1/quality').json()['mechanic_equivalent_text_groups']),0)

    def test_all_records_validate_and_ids_resolve(self):
        with connect() as db:
            info = metadata(db,'status')
            identity = snapshot(db)
            native_keys = {r[0] for r in db.execute('SELECT id FROM record_keys')}
            seen = set()
            for dataset,table in [('cards','cards'),('collection','cards'),('decks','decks'),('mechanics','mechanics')]:
                where = ' WHERE owned > 0' if dataset=='collection' else ''
                for row in db.execute(f'SELECT data FROM {table}{where}'):
                    rec = make_record(dataset,json.loads(row['data']),info,identity)
                    self.assertNotIn(str(rec.id),seen)
                    seen.add(str(rec.id))
                    self.assertIn(str(rec.id),native_keys)
                    if isinstance(rec,CardRecord):
                        refs = rec.payload.linked_card_ids + [x.mechanic_id for x in rec.payload.mechanics]
                        if rec.payload.rebalanced_card_id: refs.append(rec.payload.rebalanced_card_id)
                        for ref in refs: self.assertIn(str(ref),native_keys)
            self.assertEqual(seen,native_keys)

    def test_typed_export_matches_records(self):
        page = self.client.get('/v1/records?dataset=mechanics&limit=500').json()
        response = self.client.get('/v1/record-exports/mechanics.jsonl')
        self.assertEqual(response.status_code,200)
        self.assertEqual([json.loads(line) for line in response.text.splitlines()],page['items'])

if __name__ == '__main__': unittest.main()
