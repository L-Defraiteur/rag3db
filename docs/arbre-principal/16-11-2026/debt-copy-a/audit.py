"""Read-only graph signatures and independent vector/visibility judges for one COPY."""
import collections,hashlib,json,math,re
from typed_rows import text
COL='embedding__granite_278m';MARK='_embed_hash__granite_278m';TABLE='ScopeContent_Chunk'
DEBT=f'(n.{COL} IS NULL OR n.{MARK} IS NULL OR n.{MARK}<>n._text_hash)'
EXCLUDED_INTERNAL='Operational internal row data are not compared; all table schemas plus generation/cell and stable config/model metadata are compared.'
def identifier(v):
 if not re.fullmatch('[A-Za-z_][A-Za-z0-9_]*',v):raise ValueError('unexpected schema identifier')
 return '`'+v+'`'
def number(value):
 if value[1] is None:raise ValueError('unexpected NULL scalar')
 return int(value[1])
def strict_json(value):
 def unique(pairs):
  out={}
  for k,v in pairs:
   if k in out:raise ValueError('duplicate metadata JSON key')
   out[k]=v
  return out
 return json.loads(value,object_pairs_hook=unique)

def stable_metadata(reader):
 rows=reader.collect("MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'entity_config:' OR m._key STARTS WITH 'relation:' OR m._key STARTS WITH 'embedding_model:' OR m._key STARTS WITH 'fts_generation:' OR m._key IN ['code_scope_form','storage_descriptor:v1','schema_version'] RETURN m._key,m._value")
 result={}
 for row in rows:
  key,val=map(text,row)
  if key in result:raise ValueError('duplicate stable metadata key')
  result[key]=strict_json(val) if key.startswith(('entity_config:','relation:','embedding_model:','storage_descriptor:')) else val
 if 'storage_descriptor:v1' in result:raise ValueError('this frozen CLI cannot admit a versioned storage descriptor; no bypass allowed')
 if result.get('code_scope_form')!='code-scope-v1/content/generations':raise ValueError('not the qualified generation form')
 if result.get('embedding_model:granite_278m')!={'name':'granite-278m','dim':768,'storage':'suffixed'}:raise ValueError('persisted model identity differs')
 if result['entity_config:Scope'].get('searchIndex')!='ScopeContent':raise ValueError('public Scope association differs')
 return result

def snapshot(reader,cohort):
 ids=','.join("'"+x+"'" for x in sorted(cohort));tables=[];signatures={};schemas={};offsets={}
 for row in reader.collect('CALL show_tables() RETURN name,type'):
  name,kind=map(text,row);identifier(name)
  if kind not in ['NODE','REL'] or any(x[0]==name for x in tables):raise ValueError('invalid table inventory')
  tables.append((name,kind))
 for name,kind in sorted(tables):
  q=identifier(name);columns=[text(x[0]) for x in reader.collect("CALL table_info('"+name+"') RETURN name,type")]
  for col in columns:identifier(col)
  schemas[name]=reader.digest("CALL table_info('"+name+"') RETURN *")
  if kind=='REL':schemas[name+'-connections']=reader.digest("CALL show_connection('"+name+"') RETURN *")
  if name.startswith('_') and name not in ['_generation','_Org','_Project']:continue
  if kind=='NODE':
   if name=='_generation':
    expressions=','.join('n.'+identifier(c) for c in columns)
    signatures[name]=reader.digest('MATCH (n:'+q+') RETURN '+expressions);continue
   if '_uuid' not in columns:raise ValueError('logical node has no UUID '+name)
   raw=reader.collect('MATCH (n:'+q+') RETURN OFFSET(id(n))');physical=[number(x[0]) for x in raw]
   if any(x<0 or x>=2**63 for x in physical) or len(set(physical))!=len(physical):raise ValueError('duplicate physical ID '+name)
   offsets[name]=sorted(physical)
   if name==TABLE:
    kept=[c for c in columns if c not in [COL,MARK,'_embed_claim']]
    signatures[name+'-cohort']=reader.digest('MATCH (n:'+q+') WHERE n._uuid IN ['+ids+'] RETURN OFFSET(id(n)),'+','.join('n.'+identifier(c) for c in kept))
    signatures[name+'-outside']=reader.digest('MATCH (n:'+q+') WHERE NOT (n._uuid IN ['+ids+']) RETURN OFFSET(id(n)),'+','.join('n.'+identifier(c) for c in columns))
   else:signatures[name]=reader.digest('MATCH (n:'+q+') RETURN OFFSET(id(n)),'+','.join('n.'+identifier(c) for c in columns))
  else:
   expressions=['a._uuid','b._uuid']+['r.'+identifier(c) for c in columns]
   signatures[name]=reader.digest('MATCH (a)-[r:'+q+']->(b) RETURN '+','.join(expressions))
 meta=stable_metadata(reader)
 generation=reader.collect('MATCH (g:_generation) RETURN g.couche,g.rang')
 if [[text(r[0]),number(r[1])] for r in generation]!=[['backend',0]]:raise ValueError('generation is not backend0')
 visibility={}
 for name in ['File','Scope']:
  visible=reader.collect('MATCH (n:'+identifier(name)+") WHERE n._couche='backend' AND n._depuis<=1 AND (n._jusqu_a IS NULL OR n._jusqu_a>1) RETURN OFFSET(id(n)),n._uuid")
  visible_ids=[number(x[0]) for x in visible];uuids=[text(x[1]) for x in visible]
  if sorted(visible_ids)!=offsets[name] or len(set(uuids))!=len(uuids):raise ValueError('visible IDs or UUIDs do not equal physical occurrences')
  visibility[name]=dict(ids=sorted(visible_ids),uuids=sorted(uuids),read_rank=1)
 links=reader.collect('MATCH (v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) RETURN s._uuid,v._uuid')
 pairs=[tuple(map(text,x)) for x in links]
 if collections.Counter(x[0] for x in pairs)!=collections.Counter(visibility['Scope']['uuids']):raise ValueError('content association not exactly once per occurrence')
 return dict(tables=sorted(tables),schemas=schemas,rows=signatures,offsets=offsets,stable_metadata=meta,visibility=visibility,occurrence_to_content=sorted(pairs),internal_limit=EXCLUDED_INTERNAL)

def inputs(reader,cohort,blake):
 rows=reader.collect(f'MATCH (n:{TABLE}) WHERE '+DEBT+' RETURN n._uuid,n._text,n._title,n._text_hash,n._parent_field,n._index,n._org,n._project,n._embed_claim')
 schema={text(r[0]):text(r[1]) for r in reader.collect("CALL table_info('ScopeContent_Chunk') RETURN name,type")}
 if schema.get(COL)!='FLOAT[768]' or schema.get(MARK)!='STRING':raise ValueError('actual dense storage column/marker type differs')
 records=[];seen=set()
 for row in rows:
  uid,body=text(row[0]),text(row[1]);title='' if row[2][1] is None else text(row[2]);h=text(row[3]);field=text(row[4])
  if row[8][1] not in [None,b'']:raise ValueError('source cohort already has embedding claims')
  if uid not in cohort or uid in seen or field!='content' or text(row[6])!='default' or text(row[7])!='default':raise ValueError('unexpected debt identity/field/cell')
  if blake(body.encode())!=h:raise ValueError('persisted text differs from its hash')
  seen.add(uid);records.append(dict(uuid=uid,text_hash=h,text_sha256=hashlib.sha256(body.encode()).hexdigest(),title_sha256=hashlib.sha256(title.encode()).hexdigest(),bytes=len(body.encode()),chars=len(body),field=field,index=number(row[5]),embedding_input='_text exactly; no _title prefix in current EmbedNode'))
 if seen!=set(cohort) or len(records)!=469:raise ValueError('debt cohort changed')
 return sorted(records,key=lambda x:x['uuid'])

def finished(reader,cohort):
 bad=reader.collect(f'MATCH (n:{TABLE}) WHERE '+DEBT+' RETURN n._uuid')
 if bad:raise ValueError('dense debt remains or changed hash/marker')
 checked=[];claims=[];all_ids=set()
 def check(row):
  uid=text(row[0]);vector=row[1][1]
  if uid in all_ids or not isinstance(vector,list) or len(vector)!=768 or any(v[0] not in ['FLOAT','DOUBLE'] or not isinstance(v[1],float) or not math.isfinite(v[1]) for v in vector):raise ValueError('vector ID/dimension/finite judge failed')
  all_ids.add(uid)
  if text(row[2])!=text(row[3]):raise ValueError('marker remains invalid')
  if uid in cohort:
   if row[4][1] not in [None,b'']:
    claim=text(row[4])
    if not re.fullmatch(r'[0-9]{20}\|[0-9a-f-]{36}',claim):raise ValueError('malformed operational claim')
    claims.append(claim)
   checked.append(uid)
 reader.visit(f'MATCH (n:{TABLE}) RETURN n._uuid,n.{COL},n._text_hash,n.{MARK},n._embed_claim',check)
 if collections.Counter(checked)!=collections.Counter(cohort):raise ValueError('completed vector coverage differs')
 # Readiness is judged independently; source.meta never qualifies the physical index.
 indexes=reader.collect('CALL show_indexes() RETURN table_name,index_name,state,extension_loaded')
 found=[r for r in indexes if text(r[0])==TABLE and text(r[1])==TABLE+'_vec__granite_278m']
 if len(found)!=1 or text(found[0][2])!='READY' or found[0][3][0] not in ['BOOL','BOOLEAN'] or str(found[0][3][1]).lower()!='true':raise ValueError('physical vector index not READY/loaded')
 if len({v.split('|',1)[1] for v in claims})>1:raise ValueError('multiple writers claimed private cohort')
 return dict(total_vectors_checked=len(all_ids),operational_claims_remaining=len(claims),operational_claims_sha256=hashlib.sha256(json.dumps(sorted(claims)).encode()).hexdigest(),claims_are_not_vector_debt=True,debt=0,covered=sorted(checked),dimension=768,all_finite=True,index_ready=True,reused_count=None,computed_count=None,minimum_calculations_claimed=False)
