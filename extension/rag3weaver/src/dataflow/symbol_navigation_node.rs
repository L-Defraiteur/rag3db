//! Adapter des faits déclarés par un Plan à la navigation commune.
use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::symbol_navigation::{self, Navigation};
use serde_json::{json, Value};

pub struct SymbolNavigationNodeFactory;
struct SymbolNavigationNode {
    name: String,
    config: Value,
}
impl NodeFactory for SymbolNavigationNodeFactory {
    fn node_type(&self) -> &str {
        "SymbolNavigationNode"
    }
    fn snapshot_safe(&self, _: &Value) -> bool {
        true
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        for k in ["path", "mode", "symbol", "source", "format"] {
            if config.get(k).is_some_and(|v| !v.is_string()) {
                return Err(format!("SymbolNavigationNode: {k} doit être du texte"));
            }
        }
        for (k, default, max) in [
            ("depth", 2, symbol_navigation::MAX_DEPTH),
            ("limit", 80, symbol_navigation::MAX_NODES),
        ] {
            let n = config
                .get(k)
                .map(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(Some(default))
                .ok_or_else(|| format!("{k}: entier positif ou nul requis"))?;
            if n > max || (k == "limit" && n == 0) {
                return Err(format!("{k}: plafond {max}"));
            }
        }
        Ok(Box::new(SymbolNavigationNode {
            name: name.into(),
            config: config.clone(),
        }))
    }
    fn schema(&self) -> NodeSchema {
        let string = |name: &str, default: &str, description: &str| ConfigParam {
            name: name.to_owned().into(),
            param_type: ConfigParamType::String,
            required: name == "path",
            default: if name == "path" {
                None
            } else {
                Some(json!(default))
            },
            description: description.to_owned().into(),
            choices: None,
            json_schema: None,
        };
        NodeSchema{node_type:"SymbolNavigationNode".into(),description:"Naviguer dans les plages et membres partiels d'un index, avec contrôle de provenance/fraîcheur".into(),
            inputs:vec![PortDef::value("rows",PortType::Map,true),PortDef::value("total",PortType::Map,true)],
            outputs:vec![PortDef::value("result",PortType::Map,false)],config_params:vec![
                string("path","","Chemin relatif exact du fichier indexé"),string("source","","Source indexée, vide : refuser les sources ambiguës"),
                string("symbol","","UUID ou nom exact dans ce fichier/source ; obligatoire hors mode file"),
                ConfigParam{choices:Some(Choices::fixed(["file","members","parents","children"])),..string("mode","file","Vue de fichier ou parcours depuis un symbole")},
                ConfigParam{name:"depth".into(),param_type:ConfigParamType::Int,required:false,default:Some(json!(2)),description:"Arêtes depuis racine (0=racine), plafond16".into(),choices:None,json_schema:None},
                ConfigParam{name:"limit".into(),param_type:ConfigParamType::Int,required:false,default:Some(json!(80)),description:"Nœuds/faits rendus, plafond200 et budget96Kio".into(),choices:None,json_schema:None},
                ConfigParam{choices:Some(Choices::fixed(["markdown","json"])),..string("format","markdown","Rendu compact ou résultat structuré")}]}
    }
}
impl Node for SymbolNavigationNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "SymbolNavigationNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        SymbolNavigationNodeFactory.schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        SymbolNavigationNodeFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let rows = ctx
            .input("rows")
            .and_then(|p| p.downcast::<Value>())
            .and_then(Value::as_array)
            .ok_or("SymbolNavigationNode: rows requis")?;
        let total = ctx
            .input("total")
            .and_then(|p| p.downcast::<Value>())
            .and_then(Value::as_u64)
            .ok_or("SymbolNavigationNode: total requis")? as usize;
        let s = |k: &str, default: &str| self.config[k].as_str().unwrap_or(default).to_owned();
        let (path, source, mode, symbol, format) = (
            s("path", ""),
            s("source", ""),
            s("mode", "file"),
            s("symbol", ""),
            s("format", "markdown"),
        );
        let files = crate::code_tools::source_service(ctx);
        let result = symbol_navigation::navigate(
            rows,
            total,
            files.as_deref(),
            Navigation {
                path: &path,
                source: &source,
                mode: &mode,
                symbol: &symbol,
                depth: self.config["depth"].as_u64().unwrap_or(2) as usize,
                limit: self.config["limit"].as_u64().unwrap_or(80) as usize,
            },
        )?;
        let rendered = match format.as_str() {
            "json" => result,
            "markdown" => json!(symbol_navigation::markdown(&result)),
            _ => return Err("format: markdown | json".into()),
        };
        ctx.set_output("result", PortValue::new(rendered));
        Ok(())
    }
}
