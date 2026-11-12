```python
import os,time,json,pathlib,signal,hashlib
import google.auth,google.auth.transport.requests,requests
p=pathlib.Path(__file__).parent
t=time.monotonic();meta={'model':'google/gemini-3.8-flash','location':'global','max_seconds':120,'attempts':0,'source':'d2cb7f0636d099823f63de2cef674b630a46bc6f','prompt_sha256':hashlib.sha256((p/'prompt.txt').read_bytes()).hexdigest()}
def stop(*_): raise TimeoutError('deadline globale 120s')
signal.signal(signal.SIGALRM,stop);signal.alarm(120)
try:
 os.environ['GOOGLE_APPLICATION_CREDENTIALS']='/home/lucied/git_workspaces/rag3db/.vault/vertex-sa.json'
 creds,project=google.auth.default(scopes=['https://www.googleapis.com/auth/cloud-platform'])
 creds.refresh(google.auth.transport.requests.Request())
 url=f'https://aiplatform.googleapis.com/v1/projects/{project}/locations/global/endpoints/openapi/chat/completions'
 meta['endpoint']=url
 body={'model':meta['model'],'messages':[{'role':'user','content':(p/'prompt.txt').read_text()}],'temperature':0,'max_tokens':16000,'stream':False}
 meta['attempts']=1
 r=requests.post(url,json=body,headers={'Authorization':'Bearer '+creds.token},timeout=max(1,119-(time.monotonic()-t)))
 meta['http_status']=r.status_code
 (p/'response.json').write_text(r.text)
 r.raise_for_status();data=r.json();meta['usage']=data.get('usage');meta['id']=data.get('id');meta['finish_reason']=data['choices'][0].get('finish_reason');(p/'response.txt').write_text(data['choices'][0]['message'].get('content') or '')
 meta['status']='response_received'
except Exception as e:
 meta['status']='error';meta['error_type']=type(e).__name__
 (p/'error.txt').write_text(type(e).__name__+'\n')
finally:
 signal.alarm(0);meta['elapsed_seconds']=time.monotonic()-t;(p/'metadata.json').write_text(json.dumps(meta,indent=2));print(json.dumps(meta))

```
