"""Local, read-only ingestion source for MTG Arena snapshots."""
from contextlib import contextmanager
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import sqlite3
import unicodedata
from uuid import UUID
from typing import Annotated, Any, Generic, Literal, TypeVar

from fastapi import FastAPI, HTTPException, Query
from fastapi.responses import RedirectResponse, StreamingResponse
from pydantic import BaseModel, ConfigDict
from contract import SCHEMA_VERSION, NAMESPACE, Record, MechanicRecord, AbilityPayload, MechanicLink, schema_catalog
from records import KINDS, make_record, source_manifest

ROOT = Path(__file__).resolve().parent
DATABASE = ROOT / 'data/arena.sqlite'
app = FastAPI(
    title='MTGA — source locale pour ingestion', version='0.2.0',
    description=(
        'Collection, textes FR/EN et decks issus d’un export local d’Arena. '
        'Lecture seule ; le jeu peut être fermé. Les données restent figées jusqu’au prochain refresh. '
        'Pagination stable par identifiant. Réutiliser snapshot_id pour détecter un changement de source. '
        'Les quantités concernent chaque impression Arena ; aucune validation de légalité des decks.'
    ),
)

class Card(BaseModel):
    model_config = ConfigDict(extra='allow')
    arena_id: int
    owned: int | None
    name_en: str
    name_fr: str
    mana_cost: str
    type_en: str
    type_fr: str
    text_en: str
    text_fr: str
    colors: list[str]
    color_identity: list[str]
    linked_faces: list[int]
    is_rebalanced: bool
    is_digital_only: bool
    set: str
    collector_number: str
    power: str
    toughness: str
    abilities: list[AbilityPayload]
    mechanics: list[MechanicLink]
    mechanic_mentions: list[MechanicLink]

class CardDetail(BaseModel):
    snapshot_id: str
    card: Card
    linked_faces: list[Card]

class DeckSummary(BaseModel):
    id: str
    name: str
    format: str | None

class DeckEntry(BaseModel):
    cardId: int
    quantity: int
    name_en: str | None
    name_fr: str | None
    card: Card | None

class Deck(DeckSummary):
    attributes: dict[str, str]
    piles: dict[str, list[DeckEntry]]

class DeckDetail(BaseModel):
    snapshot_id: str
    deck: Deck

class Document(BaseModel):
    id: str
    source_uri: str
    kind: Literal['card', 'deck']
    text: str
    metadata: dict[str, Any]

T = TypeVar('T')
class Page(BaseModel, Generic[T]):
    snapshot_id: str
    total: int
    limit: int
    offset: int
    next_offset: int | None
    items: list[T]

Limit = Annotated[int, Query(ge=1, le=500)]
Offset = Annotated[int, Query(ge=0)]
Search = Annotated[str, Query(max_length=250)]
Snapshot = Annotated[str | None, Query(description='Identifiant renvoyé par /v1/status. HTTP 409 si le snapshot a changé.')]
Dataset = Literal['collection', 'cards', 'decks']
RecordDataset = Literal['collection', 'cards', 'decks', 'mechanics', 'boosters']

@contextmanager
def connect():
    if not DATABASE.is_file():
        raise HTTPException(503, 'Snapshot absent : exécuter scripts/build.py.')
    db = sqlite3.connect(DATABASE.as_uri()+'?mode=ro', uri=True)
    db.row_factory = sqlite3.Row
    db.execute('BEGIN')
    try:
        yield db
    finally:
        db.close()

def metadata(db, key):
    row = db.execute('SELECT data FROM metadata WHERE key=?', (key,)).fetchone()
    if row is None: raise HTTPException(503, 'Snapshot incomplet.')
    return json.loads(row['data'])

def snapshot(db, expected=None):
    status = metadata(db, 'status')
    identity = hashlib.sha256((SCHEMA_VERSION+json.dumps(status, sort_keys=True)).encode()).hexdigest()[:20]
    if expected is not None and expected != identity:
        raise HTTPException(409, 'Le snapshot a changé. Recommencer l’ingestion avec /v1/status.')
    return identity

def require_collection(db):
    if not metadata(db, 'status')['collection_available']:
        raise HTTPException(409, 'Collection non capturée : exécuter scripts/collect.cjs puis scripts/build.py.')

def page(identity, total, limit, offset, items):
    return dict(snapshot_id=identity, total=total, limit=limit, offset=offset,
                next_offset=offset+limit if offset+limit < total else None, items=items)

def card_search(db, q, owned_only, limit, offset, expected):
    identity = snapshot(db, expected)
    where, params = [], []
    if owned_only:
        require_collection(db)
        where.append('c.owned > 0')
    tokens = re.findall(r'\w+', q, flags=re.UNICODE)
    if tokens:
        where.append('c.arena_id IN (SELECT rowid FROM cards_fts WHERE cards_fts MATCH ?)')
        params.append(' AND '.join('"'+t+'"*' for t in tokens))
    elif q.strip():
        where.append('0')
    sql_where = ' WHERE '+' AND '.join(where) if where else ''
    total = db.execute('SELECT COUNT(*) FROM cards c'+sql_where, params).fetchone()[0]
    rows = db.execute('SELECT c.data FROM cards c'+sql_where+' ORDER BY c.arena_id LIMIT ? OFFSET ?', [*params,limit,offset])
    return page(identity, total, limit, offset, [json.loads(r['data']) for r in rows])

def card_by_id(db, arena_id):
    r = db.execute('SELECT data FROM cards WHERE arena_id=?', (arena_id,)).fetchone()
    return json.loads(r['data']) if r else None

def expand_deck(db, data):
    # Card rows include owned counts and rules text. No inferred ownership from deck lists.
    for entries in data['piles'].values():
        for entry in entries:
            entry['card'] = card_by_id(db, entry['cardId'])
    return data

def document(db, dataset, row, identity):
    if dataset == 'decks':
        d = json.loads(row['data'])
        text = '\n'.join([d['name'], d['format'] or '', *[
            f"{pile}: {c['quantity']} {c['name_en'] or c['cardId']}" for pile, cs in d['piles'].items() for c in cs]])
        return dict(id='arena:deck:'+d['id'], source_uri='mtga://deck/'+d['id'], kind='deck', text=text,
                    metadata={'snapshot_id': identity, **d})
    c = json.loads(row['data'])
    sections = []
    faces = [c]+[f for i in c['linked_faces'] if i != c['arena_id'] and (f := card_by_id(db, i))]
    for f in faces:
        sections.append('\n'.join([f['name_fr']+' / '+f['name_en'], f['mana_cost'], f['type_fr'],
                                    f['text_fr'], f['type_en'], f['text_en']]))
    return dict(id=f"arena:card:{c['arena_id']}", source_uri=f"mtga://card/{c['arena_id']}",
                kind='card', text='\n\n'.join(sections), metadata={'snapshot_id': identity, **c})

def dataset_sql(db, dataset):
    if dataset == 'collection': require_collection(db)
    if dataset == 'boosters': raise HTTPException(409,'Inventaire de boosters indisponible dans cette capture ; voir /v1/schema.')
    table = 'mechanics' if dataset=='mechanics' else 'decks' if dataset == 'decks' else 'cards'
    where = ' WHERE owned > 0' if dataset == 'collection' else ''
    order = 'key' if dataset=='mechanics' else 'id' if dataset == 'decks' else 'arena_id'
    return table, where, order

@app.get('/', include_in_schema=False)
def root():
    return RedirectResponse('/docs')

@app.get('/v1/status', tags=['Source'])
def status() -> dict[str, Any]:
    with connect() as db:
        return {'snapshot_id': snapshot(db), **metadata(db, 'status'), 'inventory': metadata(db, 'inventory')}

@app.get('/v1/schema', tags=['Ingestion structurée'])
def schema() -> dict[str, Any]:
    """Découverte des datasets, types de chaque payload, enums, références et suggestions d’index.

    Le JSON Schema et les réponses sont issus des mêmes modèles Pydantic. Les suggestions
    d’index ne créent pas d’index ni de vecteurs. Puissance variable (*, 1+*) reste une chaîne.
    """
    with connect() as db:
        info = metadata(db,'status')
        return dict(schema_version=SCHEMA_VERSION,snapshot_id=snapshot(db),source_id=info['source_id'],
                    datasets=source_manifest(info),payloads=schema_catalog(),identity={
                        'encoding':'UUIDv5', 'namespace':str(NAMESPACE),
                        'card':'mtga:card:{arena_id} — identité par impression/face, jamais par nom',
                        'collection_entry':'mtga:{source_id}:collection:{arena_id}',
                        'deck':'mtga:{source_id}:deck:{arena_deck_id}',
                        'mechanic':'mtga:mechanic:{client_key}',
                        'booster_holding':'Réservé : mtga:{source_id}:booster:{product_code} ; lot de produit, pas booster individuel',
                        'source_scope':'Profil local persistant, pas un identifiant de compte. Ne pas mélanger plusieurs comptes.',
                        'deduplication':'Upsert par id. Noms normalisés utiles pour recherche, pas pour fusion des entités.',
                    })

@app.get('/v1/records', response_model=Page[Record], tags=['Ingestion structurée'])
def records(dataset: RecordDataset = 'collection', limit: Limit = 100, offset: Offset = 0, snapshot_id: Snapshot = None):
    """Records typés : UUID stable + kind + payload structuré + projection textuelle.

    Ingestion conseillée : cards, collection, decks, mechanics ; joindre les références par UUID.
    Les capacités natives sont dans payload.abilities ; les mentions lexicales restent dans mechanic_mentions.
    """
    with connect() as db:
        identity = snapshot(db,snapshot_id)
        info = metadata(db,'status')
        table,where,order = dataset_sql(db,dataset)
        total = db.execute(f'SELECT COUNT(*) FROM {table}{where}').fetchone()[0]
        rows = db.execute(f'SELECT data FROM {table}{where} ORDER BY {order} LIMIT ? OFFSET ?',(limit,offset))
        return page(identity,total,limit,offset,[make_record(dataset,json.loads(r['data']),info,identity) for r in rows])

@app.get('/v1/records/{record_id}', response_model=Record, tags=['Ingestion structurée'])
def record(record_id: UUID, snapshot_id: Snapshot = None):
    """Résoudre un UUID et suivre une référence depuis collection, deck ou capacité."""
    with connect() as db:
        identity = snapshot(db,snapshot_id)
        key = db.execute('SELECT kind,native_id FROM record_keys WHERE id=?',(str(record_id),)).fetchone()
        if key is None: raise HTTPException(404,'Record inconnu.')
        dataset = next(d for d,k in KINDS.items() if k==key['kind'])
        table,_,order = dataset_sql(db,dataset)
        data = json.loads(db.execute(f'SELECT data FROM {table} WHERE {order}=?',(key['native_id'],)).fetchone()['data'])
        return make_record(dataset,data,metadata(db,'status'),identity)

@app.get('/v1/record-exports/{dataset}.jsonl', tags=['Ingestion structurée'],
         responses={200:{'content':{'application/x-ndjson':{}},'description':'Records typés, un JSON par ligne.'}})
def record_export(dataset: RecordDataset, snapshot_id: Snapshot = None):
    """Exporter un dataset complet au format des records typés."""
    with connect() as db:
        identity = snapshot(db,snapshot_id)
        info = metadata(db,'status')
        table,where,order = dataset_sql(db,dataset)
        lines = [make_record(dataset,json.loads(r['data']),info,identity).model_dump_json()+'\n'
                 for r in db.execute(f'SELECT data FROM {table}{where} ORDER BY {order}')]
    return StreamingResponse(iter(lines),media_type='application/x-ndjson',headers={
        'X-Snapshot-Id':identity,'Content-Disposition':f'attachment; filename="mtga-{dataset}-records.jsonl"'})

def folded(text):
    normalized = unicodedata.normalize('NFKD',text or '').casefold()
    return ' '.join(''.join(c for c in normalized if not unicodedata.combining(c)).split())

@app.get('/v1/mechanics', response_model=Page[MechanicRecord], tags=['Mécaniques'])
def mechanics(q: Search = '', limit: Limit = 100, offset: Offset = 0, snapshot_id: Snapshot = None):
    """Recherche insensible à la casse/aux accents dans les noms, alias et définitions du glossaire.

    Cette recherche accepte des fragments ; l’association native carte/capacité utilise une égalité de libellé.
    """
    with connect() as db:
        identity = snapshot(db,snapshot_id)
        matches = []
        for r in db.execute('SELECT data FROM mechanics ORDER BY key'):
            m = json.loads(r['data'])
            haystack = '\n'.join([m['name_en'],m['name_fr'],*m['aliases'],m['definition_en'] or '',m['definition_fr'] or ''])
            if folded(q) in folded(haystack): matches.append(m)
        return page(identity,len(matches),limit,offset,[make_record('mechanics',m,metadata(db,'status'),identity) for m in matches[offset:offset+limit]])

@app.get('/v1/mechanics/{key}', response_model=MechanicRecord, tags=['Mécaniques'])
def mechanic(key: str, snapshot_id: Snapshot = None):
    """Entrée par clé source exacte, par exemple Flying, Hexproof ou Descend."""
    with connect() as db:
        identity = snapshot(db,snapshot_id)
        r = db.execute('SELECT data FROM mechanics WHERE key=?',(key,)).fetchone()
        if r is None: raise HTTPException(404,'Mécanique inconnue ; rechercher via /v1/mechanics?q=...')
        return make_record('mechanics',json.loads(r['data']),metadata(db,'status'),identity)

@app.get('/v1/quality', tags=['Source'])
def quality() -> dict[str, Any]:
    """Mesure des alias et homonymes. Aucun rapprochement textuel n’efface une identité native."""
    with connect() as db:
        groups = defaultdict(list)
        names = defaultdict(list)
        for r in db.execute('SELECT data FROM mechanics'):
            m = json.loads(r['data'])
            # Accent differences are not ignored when establishing equivalence.
            normalize = lambda s: ' '.join(unicodedata.normalize('NFKC',s or '').casefold().split())
            if m['definition_status']=='available':
                groups[(normalize(m['name_en']),normalize(m['definition_en']),normalize(m['definition_fr']))].append(m['key'])
            names[normalize(m['name_en'])].append(m['key'])
        return dict(snapshot_id=snapshot(db),mechanic_equivalent_text_groups=[v for v in groups.values() if len(v)>1],
                    mechanic_same_label_groups=[v for v in names.values() if len(v)>1],
                    policy={'card_abilities':'Tableau natif Cards.AbilityIds → Abilities et localisations. Identité ability_id + text_id pour sa présentation.',
                            'glossary_links':'Libellé entier, titre balisé ou BaseId natif confirmé par le début du texte ; aucune simple mention ne devient une capacité.',
                            'mentions':'Détection séparée avec frontières de mots et expressions longues prioritaires ; non exhaustive.',
                            'normalization':'NFKC, casefold, espaces pour diagnostic ; accents ignorés seulement pour recherche.',
                            'merging':'Aucune fusion par nom ni par substring. Les groupes de textes équivalents sont exposés pour décision explicite.'})

@app.get('/v1/collection', response_model=Page[Card], tags=['Collection'])
def collection(q: Search = '', limit: Limit = 100, offset: Offset = 0, snapshot_id: Snapshot = None):
    """Cartes possédées uniquement, quantités par identifiant Arena, textes FR/EN inclus.

    q recherche des préfixes de mots dans les noms, types et capacités (tous les mots demandés).
    """
    with connect() as db:
        return card_search(db, q, True, limit, offset, snapshot_id)

@app.get('/v1/cards', response_model=Page[Card], tags=['Cartes'])
def cards(q: Search = '', owned_only: bool = False, limit: Limit = 100, offset: Offset = 0, snapshot_id: Snapshot = None):
    """Catalogue local Arena. owned=null si la collection n’a pas été capturée, sinon quantité de cette impression."""
    with connect() as db:
        return card_search(db, q, owned_only, limit, offset, snapshot_id)

@app.get('/v1/cards/{arena_id}', response_model=CardDetail, tags=['Cartes'])
def card(arena_id: int, snapshot_id: Snapshot = None):
    """Une carte et les données des faces associées."""
    with connect() as db:
        identity = snapshot(db, snapshot_id)
        c = card_by_id(db, arena_id)
        if c is None: raise HTTPException(404, 'Carte inconnue.')
        faces = [f for i in c['linked_faces'] if i != arena_id and (f := card_by_id(db, i))]
        return dict(snapshot_id=identity, card=c, linked_faces=faces)

@app.get('/v1/decks', response_model=Page[DeckSummary], tags=['Decks'])
def decks(name: Search = '', limit: Limit = 100, offset: Offset = 0, snapshot_id: Snapshot = None):
    """Liste des decks ; name filtre par fragment de nom, sans distinction de casse."""
    with connect() as db:
        identity = snapshot(db, snapshot_id)
        rows = [dict(r) for r in db.execute('SELECT id,name,format FROM decks ORDER BY id') if name.casefold() in r['name'].casefold()]
        return page(identity, len(rows), limit, offset, rows[offset:offset+limit])

@app.get('/v1/decks/{deck_id}', response_model=DeckDetail, tags=['Decks'])
def deck(deck_id: str, snapshot_id: Snapshot = None):
    """Deck complet : main, réserve, commandants, compagnons, textes et quantités possédées de chaque impression."""
    with connect() as db:
        identity = snapshot(db, snapshot_id)
        r = db.execute('SELECT data FROM decks WHERE id=?', (deck_id,)).fetchone()
        if r is None: raise HTTPException(404, 'Deck inconnu.')
        return dict(snapshot_id=identity, deck=expand_deck(db, json.loads(r['data'])))

@app.get('/v1/documents', response_model=Page[Document], tags=['Ingestion'])
def documents(dataset: Dataset = 'collection', limit: Limit = 100, offset: Offset = 0, snapshot_id: Snapshot = None):
    """Documents génériques id/text/metadata pour un futur adaptateur rag3weaver.

    Identifiants stables pour upsert. dataset=collection : cartes possédées ; cards : catalogue ; decks : decks.
    Les textes des faces associées sont inclus. Conserver les nombres dans metadata pour les filtres structurés.
    """
    with connect() as db:
        identity = snapshot(db, snapshot_id)
        table, where, order = dataset_sql(db, dataset)
        total = db.execute(f'SELECT COUNT(*) FROM {table}{where}').fetchone()[0]
        rows = db.execute(f'SELECT data FROM {table}{where} ORDER BY {order} LIMIT ? OFFSET ?', (limit,offset))
        return page(identity, total, limit, offset, [document(db,dataset,r,identity) for r in rows])

@app.get('/v1/exports/{dataset}.jsonl', tags=['Ingestion'],
         responses={200: {'content': {'application/x-ndjson': {}}, 'description': 'Un document JSON par ligne (id, source_uri, kind, text, metadata).'}})
def export(dataset: Dataset, snapshot_id: Snapshot = None):
    """Téléchargement complet, sans pagination, du même format que /v1/documents."""
    # Materialize a bounded local snapshot before streaming, so errors remain HTTP errors
    # and a refresh cannot mix two snapshots during a download.
    with connect() as db:
        identity = snapshot(db, snapshot_id)
        table, where, order = dataset_sql(db, dataset)
        lines = [json.dumps(document(db,dataset,r,identity),ensure_ascii=False)+'\n'
                 for r in db.execute(f'SELECT data FROM {table}{where} ORDER BY {order}')]
    return StreamingResponse(iter(lines), media_type='application/x-ndjson',
                             headers={'X-Snapshot-Id': identity,
                                      'Content-Disposition': f'attachment; filename="mtga-{dataset}.jsonl"'})
