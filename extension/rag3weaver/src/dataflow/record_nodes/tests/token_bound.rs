//! The witness of the lot `token-count-only-for-chunking` that the pusher plays (21 November 2026): a row to chunk
//! whose chunker is missing is refused, naming why — never written without its chunks.
use super::*;

/// **A row to chunk without its chunker is refused, naming why** (c7, 21 November 2026). A chunking bounded in tokens
/// for a model whose counter this build or this machine lacks is left out of the chunker cache; `ChunkRecordNode` used
/// to give such a row no chunk at all, in silence, and the row was written without them. It refuses now, with the
/// counter's own reason: the missing tokenizer under `token-count`, the missing feature without it.
#[test]
fn a_row_to_chunk_without_its_chunker_is_refused_naming_why() {
    let mut config = make_entity_config();
    config.chunking.token_bound = Some(crate::tokens::TokenBound { model: "un-modele-sans-tokenizer".into(), window: 512 });
    let mut configs = HashMap::new();
    configs.insert("Product".to_string(), config);
    let mut services = crate::dataflow::services::ServiceRegistry::new();
    services.register("entity_configs", configs);
    services.register("chunker_cache", Arc::new(HashMap::<ChunkerConfig, Chunker>::new()));
    let mut ctx = NodeContext::with_services(Arc::new(services));
    let mut data = make_product_data("Lampe", "Une lampe de bureau en laiton, à bras articulé, pour lire tard.", "Ampoule E27.");
    data.insert("_uuid".into(), CypherValue::String("u1".into()));
    let rec = EntityRecord {
        entity_name: "Product".into(),
        data,
        entity_ref: crate::refs::EntityRef::pre_resolved(&String::new(), "u1", "u1"),
        resolver: None,
        vectors: None,
    };
    ctx.set_input("entities", PortValue::new(BatchPayload::new(PortType::Entities, vec![rec])));
    let e = ChunkRecordNode::new("chunk").execute(&mut ctx).err().expect("a row without its chunker is refused, not written without chunks");
    assert!(e.contains("un-modele-sans-tokenizer"), "the refusal names the model: {e}");
}
