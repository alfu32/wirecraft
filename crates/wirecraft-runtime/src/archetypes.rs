use serde_json::{Value, json};
use wirecraft_core::{Document, Error, Result, store};
include!(concat!(env!("OUT_DIR"), "/archetypes.rs"));
pub fn list() -> Value {
    json!({"archetypes":BUILTINS.iter().map(|(id,text)|match store::decode(text){Ok(d)=>json!({"id":id,"name":d.project_name(),"nodes":d.nodes.len(),"technologyId":d.effective(&d.root_node_id,"technologyId")}),Err(e)=>json!({"id":id,"error":e})}).collect::<Vec<_>>()})
}
pub fn load(id: &str) -> Result<Document> {
    let text = BUILTINS
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, text)| *text)
        .ok_or_else(|| Error::new("not_found", "Unknown archetype ID"))?;
    store::decode(text)
}
