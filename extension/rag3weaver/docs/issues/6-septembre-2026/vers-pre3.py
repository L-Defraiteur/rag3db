import re
p='Cargo.toml'; s=open(p).read()
s=s.replace('burn = { version = "0.22.0-pre.2"','burn = { version = "0.22.0-pre.3"').replace('burn-store = { version = "0.22.0-pre.2"','burn-store = { version = "0.22.0-pre.3"')
s=s.replace('branch = "rag3weaver/pre.2"','branch = "rag3weaver/pre.3"')
# burn-import/burn-onnx ne sont pas dans le lock ; cubecl-hip-sys reste au registre.
open(p,'w').write(s)
p='src/burn_device.rs'; s=open(p).read()
i=s.index('impl burn_store::ModuleAdapter for Flex32Adapter {')
s=s[:i]+open('docs/issues/6-septembre-2026/flex32-adapter-pre3.rs').read()
open(p,'w').write(s)
print("pre.3 posé")
