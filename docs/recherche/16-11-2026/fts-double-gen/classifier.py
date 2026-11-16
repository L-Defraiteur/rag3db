"""Scaffold: exact old predicate retained to demonstrate the real refusal."""
import re
def classify_classic(output,stderr,command,result,ledger,inventory):
 name=ledger["names"][0]
 assert name + ' ... ok' in output or re.search('test ' + re.escape(name) + '\\s+\\.\\.\\.\\s+ok', output)
 return {"status":"CLASSIC_RAW_PASSED_ONLY"}
