"""Classify raw historical evidence, never rewrite or normalize its stream."""
import re
FAILED=re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.|panicked at',re.M)
CLASSIC='code_sync::fts_guard_double_tests::classic_sync_removes_a_committed_fts_ghost'
GEN='code_sync::fts_guard_double_tests::generations_sync_removes_a_committed_fts_ghost'
def classify_classic(output,stderr,command,result,ledger,inventory):
 assert not FAILED.search(output) and not FAILED.search(stderr)
 assert result['rc']==0 and ledger['names']==[CLASSIC,GEN] and ledger['ignored']==0
 names=sorted(line[:-6] for line in inventory.splitlines() if line.endswith(': test'))
 assert names==[CLASSIC,GEN] and names.count(CLASSIC)==1
 argv=command['argv'];assert argv.count('--exact')==1 and argv.count(CLASSIC)==1 and GEN not in argv
 assert '--args' in argv and argv[argv.index('--args')+1:]==[ledger['copied_executable'],CLASSIC,'--exact','--nocapture']
 assert len(re.findall(r'^running 1 test$',output,re.M))==1
 assert len(re.findall(r'^test '+re.escape(CLASSIC)+r' \.\.\. ',output,re.M))==1
 assert len(re.findall(r'^test result: ok\. 1 passed; 0 failed; 0 ignored;',output,re.M))==1
 assert re.findall(r'^test (.+?) \.\.\. ',output,re.M)==[CLASSIC]
 assert len(re.findall(r'^test result: ',output,re.M))==1
 assert 'exited normally' in output
 traces=re.findall(r'^FTS_DOUBLE_TRACE unchanged (.+?) result=Ok\(',stderr,re.M)
 assert len(traces)==1 and traces[0]=='["dispatch:classic", "counts:begin", "counts:end:1", "rebuild:begin", "rebuild:end:1"]'
 assert re.search(r'^FTS_DOUBLE_AFTER form=code-scope-v1/content graph=\[0\] fts=\[0\].*fts_without_row=\[\] rows_without_fts=\[\]$',stderr,re.M)
 return dict(status='CLASSIC_RAW_PASSED_ONLY',historical_scope_still_red=True,classic_rerun=False,generations_qualified=False)
