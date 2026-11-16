"""Real snapshot and real driver condition across its JSON boundary; no DB."""
import ast,copy,json,tempfile,unittest
from pathlib import Path
import audit,audit_result as r

def text(s):return ('STRING',s.encode())
class Reader:
 def collect(self,sql):
  if sql=='CALL show_tables() RETURN name,type':return [[text(n),text(k)] for n,k in [('File','NODE'),('Scope','NODE'),('ScopeContent','NODE'),('Link','REL'),('_generation','NODE')]]
  if sql.startswith('CALL table_info('):return [[text('_uuid'),text('STRING')]]
  if sql.startswith('MATCH (m:_catalog_meta)'):
   return [[text(k),text(v)] for k,v in [('code_scope_form','code-scope-v1/content/generations'),('embedding_model:granite_278m','{"name":"granite-278m","dim":768,"storage":"suffixed"}'),('entity_config:Scope','{"searchIndex":"ScopeContent"}')]]
  if sql=='MATCH (g:_generation) RETURN g.couche,g.rang':return [[text('backend'),('INT64',0)]]
  return []
 def digest(self,sql):return dict(count=0,rows={})

def driver_refuses(graph,reference):
 tree=ast.parse(Path(r.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
 condition=next(n for n in ast.walk(driver) if isinstance(n,ast.If) and any(isinstance(s,ast.Raise) and 'graph/config/gen/visibility changed' in ast.unparse(s) for s in n.body))
 with tempfile.TemporaryDirectory() as d:
  root=Path(d);(root/'graph-before.json').write_text(json.dumps(reference))
  namespace=dict(vars(r),graph=graph,OLD=root)
  return eval(compile(ast.Expression(condition.test),'<actual-audit-driver-condition>','eval'),namespace)

class PublicationContract(unittest.TestCase):
 def setUp(self):
  self.graph=audit.snapshot(Reader(),[]);self.reference=json.loads(json.dumps(self.graph))
 def test_actual_snapshot_matches_its_published_reference(self):
  self.assertIsInstance(self.graph['tables'][0],tuple)
  self.assertFalse(driver_refuses(self.graph,self.reference))
 def test_rows_edges_tables_types_scalars_and_duplicates_stay_strict(self):
  variants=[]
  row=copy.deepcopy(self.graph);row['rows']['Scope']['count']=1;variants.append(row)
  edge=copy.deepcopy(self.graph);edge['rows']['Link']['rows']={'edge':1};variants.append(edge)
  duplicate=copy.deepcopy(edge);duplicate['rows']['Link']['rows']['edge']=2;variants.append(duplicate)
  table=copy.deepcopy(self.graph);table['tables'].append(('Extra','NODE'));variants.append(table)
  typename=copy.deepcopy(self.graph);typename['tables'][0]=(typename['tables'][0][0],'REL');variants.append(typename)
  for scalar in [False,0.0,'0',None]:
   changed=copy.deepcopy(self.graph);changed['rows']['Scope']['count']=scalar;variants.append(changed)
  for i,variant in enumerate(variants):
   with self.subTest(case=i):self.assertTrue(driver_refuses(variant,self.reference))

if __name__=='__main__':unittest.main(failfast=True)
