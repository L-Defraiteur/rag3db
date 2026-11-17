"""Private fixture-only Cargo adapter. Real guard/Cargo/ELF remain inside Landlock."""
from pathlib import Path
import hashlib,json,os,socket,subprocess,sys,time

def message(value):
 path=os.environ['RAG3WEAVER_D6_SOCKET']
 with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as connection:
  connection.settimeout(900);connection.connect(path);connection.sendall((json.dumps(value)+'\n').encode())
  stream=connection.makefile('rb');response=json.loads(stream.readline(2*1024*1024))
  if not response.get('ok'):raise RuntimeError('fixture Cargo admission refused: '+response.get('error','no reason'))
  return response

def main(argv):
 cwd=Path.cwd().resolve();target=Path(os.environ['CARGO_TARGET_DIR']).resolve();root=Path(os.environ['RAG3WEAVER_D6_OUTPUT']);root.mkdir(parents=True,exist_ok=True)
 base=dict(pid=os.getpid(),cwd=str(cwd),target=str(target),argv=argv)
 admission=message(dict(base,kind='admit'));identity=admission['identity'];stem=root/identity
 compiler=Path(str(stem)+'.build.jsonl');tests=Path(str(stem)+'.test.log')
 env=dict(os.environ,CARGO_MANIFEST_DIR=str(cwd),LD_LIBRARY_PATH=admission['ld'])
 # This removes ONLY this private adapter from its own guard/actual-Cargo path, avoiding recursion.
 env['PATH']=str(Path(admission['cargo']).parent)+':'+':'.join(p for p in env['PATH'].split(':') if p!=str(Path(__file__).parent/'bin'))
 command=['bash','-c','. "$1"; garder_le_target "$2" "$3" --offline --locked','task-cargo',admission['guard'],str(target),str(cwd)]
 with compiler.open('x') as output:
  message(dict(base,kind='begin',identity=identity,compile_log=str(compiler)))
  code=subprocess.run(command,env=env,stdout=output,stderr=subprocess.STDOUT).returncode
  if code==0:code=subprocess.run([admission['cargo'],*admission['build_argv']],env=env,stdout=output,stderr=subprocess.STDOUT).returncode
 if code:
  message(dict(base,kind='compile-failed',identity=identity,code=code,compile_log=str(compiler)))
  return code
 published=message(dict(base,kind='built',identity=identity,compile_log=str(compiler)))
 # The broker has copied/hashed the actual ELF and waits for publication before permitting runtime.
 message(dict(base,kind='test-begin',identity=identity,test_log=str(tests)))
 with tests.open('x') as output:code=subprocess.run([published['binary'],*admission['test_argv']],env=env,stdout=output,stderr=subprocess.STDOUT).returncode
 text=tests.read_text();sys.stdout.write(text);sys.stdout.flush()
 message(dict(base,kind='tested',identity=identity,code=code,test_log=str(tests)))
 return code

if __name__=='__main__':
 try:sys.exit(main(sys.argv[1:]))
 except Exception as error:
  print('private Cargo refused: '+repr(error),file=sys.stderr);sys.exit(97)
