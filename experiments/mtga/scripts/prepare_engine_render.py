"""Attach product templates to existing search graphs, without changing semantics."""
import json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];B=ROOT/'experiments/mtga/backend'
def prepare():
 manifest=json.loads((B/'backend.json').read_text())
 for name,tool in manifest['tools'].items():
  source=Path(tool['graph'])
  if not source.is_absolute():source=B/source
  text=source.read_text()
  if 'RenderResultsNode' not in text:
   result=re.search(r'^%% result: (\w+)\.results$',text,re.M)
   if not result:continue
   source_node=result.group(1)
   text=text[:result.start()]+"%% result: presentation.results"+text[result.end():]
   text+='\n    presentation["RenderResultsNode(template=magic)"]\n    '+source_node+' -->|results| presentation\n'
  text=text.replace('RenderResultsNode"','RenderResultsNode(template=magic)"')
  target=B/'graphs'/f'readable-{name}.mmd'
  target.write_text(text)
  tool['graph']=str(target)
 (B/'backend.json').write_text(json.dumps(manifest,indent=2)+'\n')
if __name__=='__main__':prepare()
