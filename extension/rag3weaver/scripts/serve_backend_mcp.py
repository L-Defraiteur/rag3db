#!/usr/bin/env python3
"""Expose a rag3weaver backend's declared tools through the official MCP SDK."""
import argparse
import asyncio
import json
import sys
from pathlib import Path
from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Tool, CallToolResult, TextContent

# Receipts of tools without harness: nothing to report, keep text views short.
QUIET=({'accepted':True,'errors':[],'warnings':[]},{'ok':True,'results':[]})

def harness_refusal(result):
    """A harness refusal or failed delivery is a tool error for MCP clients, not a success to parse."""
    validation=result.get('validation')
    delivery=result.get('delivery')
    lines=[]
    if isinstance(validation,dict) and validation.get('accepted') is False:
        stage=result.get('stage','?')
        executed='executed, effects kept' if result.get('executed') else 'not executed'
        lines.append(f'Rejected by harness at stage {stage} ({executed}). Fix the arguments and call the tool again.')
        for issue in validation.get('errors',[]):
            lines.append(f"- error {issue.get('code','?')} at {issue.get('path') or '/'}: {issue.get('message','')}")
    elif isinstance(delivery,dict) and delivery.get('ok') is False:
        lines.append(f"Accepted, but delivery failed: {delivery.get('error')}")
    else:
        return None
    for issue in (validation or {}).get('warnings',[]):
        lines.append(f"- warning {issue.get('code','?')} at {issue.get('path') or '/'}: {issue.get('message','')}")
    return '\n'.join(lines)

async def serve(manifest, binary, response_format="json", hidden=(), fixed_format=False):
    visible=lambda name: not any(name.startswith(prefix) for prefix in hidden)
    process = await asyncio.create_subprocess_exec(str(binary), str(manifest), stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE, stderr=sys.stderr, limit=32*1024*1024, start_new_session=True)
    lock = asyncio.Lock()
    async def request(command):
        async with lock:
            process.stdin.write((json.dumps(command,ensure_ascii=False)+'\n').encode())
            await process.stdin.drain()
            line = await process.stdout.readline()
            if not line: raise RuntimeError('backend stopped; inspect stderr')
            response = json.loads(line)
            if not response['ok']: raise RuntimeError(response['error'])
            return response['result']
    try:
        description = await request({'op':'describe'})
        server = Server(description['name'])
        @server.list_tools()
        async def list_tools():
            result=[]
            for tool in description['tools']:
                if not visible(tool['name']): continue
                tool=json.loads(json.dumps(tool))
                props=tool['inputSchema'].setdefault('properties',{})
                if 'response_format' in props: raise ValueError('response_format is reserved by MCP transport')
                if not fixed_format: props['response_format']={'type':'string','enum':['text','json'],'default':response_format,'description':'text: rendered view without duplicated JSON; json: complete structured result for programs.'}
                result.append(Tool(**tool))
            if visible('close_backend'): result.append(Tool(name='close_backend',description='Flush and close this backend session. Batch clients should await this before disconnecting to ensure durable shutdown.',inputSchema={'type':'object','properties':{},'additionalProperties':False}))
            return result
        @server.call_tool()
        async def call_tool(name, arguments):
            arguments=dict(arguments or {})
            format_requested=response_format if fixed_format else arguments.pop('response_format',response_format)
            if format_requested not in ('text','json'):
                return CallToolResult(isError=True,content=[TextContent(type='text',text='response_format must be text or json')])
            # Once submitted, finish the roundtrip even if the client cancels,
            # so the next call never consumes the previous response.
            if not visible(name):
                return CallToolResult(isError=True,content=[TextContent(type='text',text=f'unexposed tool {name}')])
            command={'op':'shutdown'} if name=='close_backend' else {'op':'call','name':name,'arguments':arguments or {}}
            task = asyncio.create_task(request(command))
            try:
                result = await asyncio.shield(task)
            except asyncio.CancelledError:
                await task
                raise
            except Exception as exc:
                return CallToolResult(isError=True,content=[TextContent(type='text',text=str(exc))])
            if isinstance(result,dict):
                refusal=harness_refusal(result)
                if refusal:
                    return CallToolResult(isError=True,content=[TextContent(type='text',text=refusal)],structuredContent=result)
                presentation=result.pop('presentation',None)
                if presentation is None and isinstance(result.get('result'),str): presentation=result['result']
                delivered=(result.get('delivery') or {}).get('results') or []
                if presentation is None and delivered and all(isinstance(text,str) for text in delivered):
                    # Accepted harnessed submission: show what on_accept rendered, verbatim.
                    presentation='\n\n'.join(delivered)
                    for issue in (result.get('validation') or {}).get('warnings',[]):
                        presentation+=f"\n- warning {issue.get('code','?')}: {issue.get('message','')}"
                    if format_requested=='text': return CallToolResult(content=[TextContent(type='text',text=presentation)])
                if format_requested=='text' and isinstance(presentation,str):
                    for key,value in result.items():
                        if key!='result' and value not in ({},None) and value not in QUIET:
                            presentation+='\n'+key+': '+json.dumps(value,ensure_ascii=False,separators=(',',':'))
                    return CallToolResult(content=[TextContent(type='text',text=presentation)])
            return result
        async with stdio_server() as streams:
            await server.run(*streams,server.create_initialization_options())
    finally:
        if process.stdin:
            process.stdin.close()  # EOF → catalogue.shutdown() → durable indexes.
        try: await asyncio.wait_for(process.wait(),30)
        except asyncio.TimeoutError:
            process.terminate()
            await process.wait()

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest',type=Path)
    parser.add_argument('--binary',type=Path,default=Path(__file__).resolve().parents[1]/'target/debug/rag3weaver-backend')
    parser.add_argument('--response-format',choices=['text','json'],default='json')
    parser.add_argument('--fixed-format',action='store_true',help='Do not let the client choose response_format (small models tend to ask for verbose json).')
    parser.add_argument('--hide',action='append',default=[],metavar='PREFIX',help='Do not expose tools whose name starts with PREFIX (repeatable).')
    args=parser.parse_args()
    asyncio.run(serve(args.manifest.resolve(),args.binary.resolve(),args.response_format,tuple(args.hide),args.fixed_format))
