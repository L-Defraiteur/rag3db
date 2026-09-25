"""Typed ingestion payloads: schema and records share these definitions."""
from datetime import datetime
from typing import Annotated, Literal
from uuid import UUID
from identity import NAMESPACE, stable_id, card_id

from pydantic import BaseModel, ConfigDict, Field

SCHEMA_VERSION = '1.1.0'
def field(kind, description, index=None, **kwargs):
    return Field(description=description, json_schema_extra={
        'payload_type': kind, 'suggested_index': index,
    }, **kwargs)

class Payload(BaseModel):
    model_config = ConfigDict(extra='forbid')

Color = Literal['White', 'Blue', 'Black', 'Red', 'Green']
Pile = Literal['MainDeck', 'Sideboard', 'CommandZone', 'Companions']

class MechanicLink(Payload):
    mechanic_id: UUID = field('reference', 'Identifiant stable dans le registre des mécaniques.', 'keyword')
    key: str = field('keyword', 'Clé du glossaire Arena.', 'keyword')
    relation: Literal['exact_ability', 'ability_heading', 'native_base_ability', 'text_mention'] = field('enum', 'Preuve du lien textuel ou natif ; une mention ne signifie pas que la carte possède la capacité.', 'keyword')
    evidence: str = field('text', 'Texte de capacité anglais sur lequel le lien a été établi.')

class AbilityPayload(Payload):
    ability_id: int = field('integer', 'Identifiant natif dans la table Abilities ; la liste provient de Cards.AbilityIds.', 'integer')
    text_id: int = field('integer', 'Identifiant de localisation natif.', 'integer')
    category_code: int | None = field('integer', 'Code de catégorie natif non interprété.', 'integer')
    subcategory_code: int | None = field('integer', 'Code de sous-catégorie natif non interprété.')
    ability_word_code: int | None = field('integer', 'Code AbilityWord natif, non utilisé comme identifiant unique de mécanique.')
    base_ability_id: int | None = field('integer', 'BaseId natif ; zéro si aucune base référencée.')
    is_intrinsic: bool | None = field('boolean', 'Indicateur natif IsIntrinsicAbility.', 'bool')
    text_en: str = field('text', 'Texte anglais de cette capacité ; peut être une phrase complexe.', 'text')
    text_fr: str = field('text', 'Texte français de cette capacité.', 'text')
    glossary_ids: list[UUID] = field('array', 'Liens au glossaire par libellé entier, titre balisé ou BaseId natif dont le libellé correspond au début de la capacité ; jamais par simple mention.', 'keyword')

class CardPayload(Payload):
    arena_id: int = field('integer', 'Identifiant Arena de cette impression ou face.', 'integer', gt=0)
    name_en: str = field('text', 'Nom anglais ; ne sert pas d’identifiant.', 'text')
    name_fr: str = field('text', 'Nom français, repli anglais si absent.', 'text')
    mana_cost: str = field('keyword', 'Coût symbolique : {3}{B}{B}. Les cartes doubles peuvent avoir un coût composite.')
    type_en: str = field('text', 'Ligne de types anglaise.', 'text')
    type_fr: str = field('text', 'Ligne de types française.', 'text')
    text_en: str = field('text', 'Capacités en anglais, balises nettoyées.', 'text')
    text_fr: str = field('text', 'Capacités en français, balises nettoyées.', 'text')
    text_raw_en: str = field('text', 'Texte source anglais avec balises de présentation.')
    text_raw_fr: str = field('text', 'Texte source français avec balises de présentation.')
    colors: list[Color] = field('array', 'Couleurs de la carte ; liste vide pour incolore.', 'keyword')
    color_identity: list[Color] = field('array', 'Identité couleur.', 'keyword')
    set_code: str = field('keyword', 'Code d’extension du client Arena.', 'keyword')
    collector_number: str = field('keyword', 'Numéro de collection, conservé en chaîne (suffixes possibles).', 'keyword')
    rarity_code: int = field('integer', 'Code de rareté brut du client ; aucune traduction supposée.', 'integer')
    power: str = field('keyword', 'Force imprimée, pouvant être *, 1+*, -2 ou vide.')
    toughness: str = field('keyword', 'Endurance imprimée, pouvant être variable ou vide.')
    power_numeric: int | None = field('integer', 'Force seulement si le texte est un entier fixe.', 'integer')
    toughness_numeric: int | None = field('integer', 'Endurance seulement si le texte est un entier fixe.', 'integer')
    is_token: bool = field('boolean', 'Entrée de jeton de jeu.', 'bool')
    is_primary: bool = field('boolean', 'Entrée primaire selon le catalogue Arena.', 'bool')
    is_digital_only: bool = field('boolean', 'Carte uniquement numérique.', 'bool')
    is_rebalanced: bool = field('boolean', 'Version rééquilibrée.', 'bool')
    linked_card_ids: list[UUID] = field('array', 'Références aux identifiants stables des faces associées.', 'keyword')
    rebalanced_card_id: UUID | None = field('reference', 'Référence à la version rééquilibrée si indiquée.', 'keyword')
    alternate_deck_limit: int = field('integer', 'Valeur brute Arena ; 0 ne signifie pas une limite de zéro carte.')
    abilities: list[AbilityPayload] = field('array', 'Tableau natif des capacités de la carte ; ne provient pas d’une analyse du texte.')
    mechanics: list[MechanicLink] = field('array', 'Correspondances exactes entre une capacité ou son titre balisé et le glossaire.')
    mechanic_mentions: list[MechanicLink] = field('array', 'Mentions lexicales supplémentaires ; ne signifient PAS que la carte possède la capacité.')

class CollectionPayload(Payload):
    source_id: UUID = field('keyword', 'Identité persistante de ce profil local d’export.', 'keyword')
    card_id: UUID = field('reference', 'Référence au record de carte, dataset=cards.', 'keyword')
    arena_id: int = field('integer', 'Identifiant d’impression Arena.', 'integer', gt=0)
    quantity: int = field('integer', 'Exemplaires possédés de cette impression.', 'integer', ge=0)
    captured_at: datetime = field('datetime', 'Date UTC de la lecture mémoire.', 'datetime')

class DeckEntryPayload(Payload):
    card_id: UUID = field('reference', 'Référence au record de carte.', 'keyword')
    arena_id: int = field('integer', 'Identifiant d’impression Arena.', 'integer', gt=0)
    quantity: int = field('integer', 'Exemplaires demandés par le deck.', 'integer', ge=0)
    pile: Pile = field('enum', 'Emplacement dans le deck.', 'keyword')

class DeckPayload(Payload):
    source_id: UUID = field('keyword', 'Identité persistante du profil local.', 'keyword')
    arena_deck_id: UUID = field('keyword', 'Identifiant natif du deck dans Arena.', 'keyword')
    name: str = field('text', 'Nom modifiable ; ne sert pas d’identifiant.', 'text')
    format: str | None = field('keyword', 'Format sauvegardé, vocabulaire ouvert ; pas une preuve de légalité.', 'keyword')
    main_deck_count: int = field('integer', 'Nombre de cartes du deck principal.', 'integer', ge=0)
    sideboard_count: int = field('integer', 'Nombre de cartes de réserve.', 'integer', ge=0)
    entries: list[DeckEntryPayload] = field('array', 'Liste structurée des cartes, quantités et emplacements.')
    attributes: dict[str, str] = field('object', 'Attributs bruts du jeu ; valeurs conservées comme chaînes.')

class BoosterPayload(Payload):
    """Reserved contract. No inventory available in the current capture."""
    source_id: UUID = field('keyword', 'Identité du profil local.', 'keyword')
    product_code: str = field('keyword', 'Code produit stable provenant du jeu, jamais déduit du nom.', 'keyword')
    name: str | None = field('text', 'Nom du produit si disponible.', 'text')
    set_code: str | None = field('keyword', 'Extension si disponible.', 'keyword')
    quantity: int = field('integer', 'Boosters non ouverts de ce produit, et non cartes contenues.', 'integer', ge=0)
    captured_at: datetime = field('datetime', 'Date de capture.', 'datetime')

class MechanicPayload(Payload):
    key: str = field('keyword', 'Clé stable du glossaire client.', 'keyword')
    name_en: str = field('text', 'Libellé anglais.', 'text')
    name_fr: str = field('text', 'Libellé français, anglais si indisponible.', 'text')
    aliases: list[str] = field('array', 'Formes de recherche complémentaires.', 'keyword')
    definition_en: str | None = field('text', 'Explication anglaise issue du client ; ce n’est pas le texte exhaustif des règles.', 'text')
    definition_fr: str | None = field('text', 'Explication française issue du client.', 'text')
    definition_status: Literal['available', 'missing'] = field('enum', 'Disponibilité d’une explication dans le client.', 'keyword')
    has_placeholders: bool = field('boolean', 'Le texte contient des paramètres de présentation non résolus.', 'bool')
    source_kind: Literal['arena_client_tooltip'] = field('enum', 'Provenance : infobulle locale du client Arena.', 'keyword')
    source_keys: list[str] = field('array', 'Clés exactes des localisations sources.')
    rules_reference_url: str = field('keyword', 'Point d’entrée vers les règles complètes pour les cas non couverts.')

class RecordBase(BaseModel):
    model_config = ConfigDict(extra='forbid')
    id: UUID = Field(description='UUIDv5 stable, utilisable comme identifiant de point ; indépendant du snapshot et du texte.')
    logical_id: str
    schema_version: Literal['1.0.0'] = SCHEMA_VERSION
    snapshot_id: str
    source_uri: str
    text: str = Field(description='Projection pour recherche textuelle/embedding ; payload reste la source structurée.')

class CardRecord(RecordBase):
    kind: Literal['card'] = 'card'
    payload: CardPayload

class CollectionRecord(RecordBase):
    kind: Literal['collection_entry'] = 'collection_entry'
    payload: CollectionPayload

class DeckRecord(RecordBase):
    kind: Literal['deck'] = 'deck'
    payload: DeckPayload

class BoosterRecord(RecordBase):
    kind: Literal['booster_holding'] = 'booster_holding'
    payload: BoosterPayload

class MechanicRecord(RecordBase):
    kind: Literal['mechanic'] = 'mechanic'
    payload: MechanicPayload

Record = Annotated[CardRecord | CollectionRecord | DeckRecord | BoosterRecord | MechanicRecord, Field(discriminator='kind')]
MODELS = {'card': CardPayload, 'collection_entry': CollectionPayload, 'deck': DeckPayload, 'booster_holding': BoosterPayload, 'mechanic': MechanicPayload}

def describe_fields(schema):
    """Flatten nested field descriptors, preserving the complete JSON Schema too."""
    definitions = schema.get('$defs', {})
    def resolve(node):
        return {**definitions[node['$ref'].split('/')[-1]], **{k:v for k,v in node.items() if k != '$ref'}} if '$ref' in node else node
    def descriptor(node, path, required):
        node = resolve(node)
        choices = node.get('anyOf', [])
        nullable = any(x.get('type') == 'null' for x in choices)
        core = resolve(next((x for x in choices if x.get('type') != 'null'), node))
        datatype = node.get('payload_type') or ('enum' if 'enum' in core or 'const' in core else 'reference' if core.get('format') == 'uuid' else {'string':'text','number':'float'}.get(core.get('type'),core.get('type','object')))
        result = dict(path=path, type=datatype, nullable=nullable, required=required,
                      description=node.get('description',''), suggested_index=node.get('suggested_index'))
        if 'format' in core: result['format'] = core['format']
        if 'enum' in core: result['values'] = core['enum']
        if 'const' in core: result['values'] = [core['const']]
        if core.get('type') == 'array':
            item = descriptor(core['items'], path+'[]', True)
            result['items'] = item
        if core.get('type') == 'object':
            result['fields'] = [descriptor(n, path+'.'+k, k in core.get('required',[])) for k,n in core.get('properties',{}).items()]
            if isinstance(core.get('additionalProperties'),dict):
                result['values'] = descriptor(core['additionalProperties'],path+'.*',True)
        return result
    return [descriptor(n,k,k in schema.get('required',[])) for k,n in schema.get('properties',{}).items()]

def schema_catalog():
    result = {}
    for kind, model in MODELS.items():
        schema = model.model_json_schema()
        result[kind] = {'fields':describe_fields(schema),'json_schema':schema}
    return result
