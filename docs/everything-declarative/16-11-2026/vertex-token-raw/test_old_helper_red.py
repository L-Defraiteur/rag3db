"""Historical helper exercised on immutable RAW; deliberately RED, never fixed."""
import ast,os,re,unittest
from pathlib import Path
ROOT=Path(os.environ.get('VERTEX_RAW_ARCHIVE',str(Path.home()/'.cache/rag3weaver-build/cx-preuves-ed-vertex-token-union-red-v1-local')))
source=(ROOT/'controller/qualify.py').read_text()
tree=ast.parse(source)
helper=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='library_summary')
ns={'re':re};exec(compile(ast.Module(body=[helper],type_ignores=[]),'<archived-968-library_summary>','exec'),ns)
class HistoricalRed(unittest.TestCase):
 def test_actual_lectures_with_two_contractual_phases_should_be_admitted(self):
  ns['library_summary']((ROOT/'client/official/e2e_lectures.log').read_text())
 def test_actual_filtered_code_phase_should_be_admitted_by_driver_assertion(self):
  driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  loop=next(n for n in driver.body if isinstance(n,ast.For) and ast.unparse(n.target)=='name')
  check=next(n for n in loop.body if isinstance(n,ast.Assert))
  text=(ROOT/'client/official/e2e_code.log').read_text().split('     Running tests/e2e_code.rs ',1)[1]
  env={'suites':{'e2e_code':ns['library_summary'](text)},'name':'e2e_code'}
  exec(compile(ast.Module(body=[check],type_ignores=[]),'<archived-968-filter-assert>','exec'),env)
if __name__=='__main__':unittest.main()
