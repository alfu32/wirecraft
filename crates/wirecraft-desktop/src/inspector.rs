//! Structured collection editors retain unknown type-field data and stage a single patch.
use super::*;
pub struct MetadataRow {
    key: Entity<InputState>,
    value: Entity<InputState>,
}
pub struct TypeRow {
    name: Entity<InputState>,
    type_id: Entity<InputState>,
    collection: Entity<InputState>,
    key_type: Entity<InputState>,
    reference: bool,
    original: Value,
}
#[derive(Default)]
pub struct Tables {
    metadata: Vec<MetadataRow>,
    types: Vec<TypeRow>,
    baseline_metadata: Value,
    baseline_types: Value,
    is_type: bool,
    baseline_form: Value,
}
fn input(value: &str, w: &mut Window, cx: &mut Context<Wirecraft>) -> Entity<InputState> {
    let field = cx.new(|cx| {
        let mut s = InputState::new(w, cx);
        s.set_value(value.to_string(), w, cx);
        s
    });
    cx.subscribe(&field, |this, _, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            this.persist_ui(cx);
        }
    })
    .detach();
    field
}
impl Tables {
    pub fn load(value: &Value, w: &mut Window, cx: &mut Context<Wirecraft>) -> Self {
        let metadata = value["metadata"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, v)| MetadataRow {
                key: input(k, w, cx),
                value: input(v.as_str().unwrap_or(""), w, cx),
            })
            .collect();
        let types = value["typeDefinition"]["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| {
                let expression = &f["typeExpression"];
                let collection = expression["constructorId"].as_str().unwrap_or("");
                let type_id = match collection {
                    "array" | "list" => expression["arguments"][0]["typeId"].as_str(),
                    "map" => expression["arguments"][1]["typeId"].as_str(),
                    _ => f["typeId"].as_str(),
                }
                .unwrap_or("");
                TypeRow {
                    name: input(f["name"].as_str().unwrap_or(""), w, cx),
                    type_id: input(type_id, w, cx),
                    collection: input(collection, w, cx),
                    key_type: input(
                        if collection == "map" {
                            expression["arguments"][0]["typeId"].as_str().unwrap_or("")
                        } else {
                            ""
                        },
                        w,
                        cx,
                    ),
                    reference: f["isReference"].as_bool().unwrap_or(false),
                    original: f.clone(),
                }
            })
            .collect();
        let mut tables = Self {
            baseline_form: Value::Null,
            metadata,
            types,
            baseline_metadata: value["metadata"].clone(),
            baseline_types: value["typeDefinition"]["fields"].clone(),
            is_type: value["kind"] == "Type",
        };
        tables.baseline_form = tables.draft(cx);
        tables
    }
    pub fn draft(&self, cx: &App) -> Value {
        json!({"metadata":self.metadata.iter().map(|r|json!([r.key.read(cx).value().to_string(),r.value.read(cx).value().to_string()])).collect::<Vec<_>>(),"types":self.types.iter().map(|r|json!({"name":r.name.read(cx).value().to_string(),"typeId":r.type_id.read(cx).value().to_string(),"collection":r.collection.read(cx).value().to_string(),"keyType":r.key_type.read(cx).value().to_string(),"reference":r.reference,"original":r.original})).collect::<Vec<_>>()})
    }
    pub fn changed(&self, cx: &App) -> bool {
        !self.baseline_form.is_null() && self.draft(cx) != self.baseline_form
    }
    pub fn restore(&mut self, value: &Value, w: &mut Window, cx: &mut Context<Wirecraft>) {
        self.metadata = value["metadata"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| MetadataRow {
                key: input(v[0].as_str().unwrap_or(""), w, cx),
                value: input(v[1].as_str().unwrap_or(""), w, cx),
            })
            .collect();
        self.types = value["types"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| TypeRow {
                name: input(v["name"].as_str().unwrap_or(""), w, cx),
                type_id: input(v["typeId"].as_str().unwrap_or(""), w, cx),
                collection: input(v["collection"].as_str().unwrap_or(""), w, cx),
                key_type: input(v["keyType"].as_str().unwrap_or(""), w, cx),
                reference: v["reference"].as_bool().unwrap_or(false),
                original: v["original"].clone(),
            })
            .collect();
    }
    pub fn merge(&self, value: &mut Value, cx: &App) -> Result<(), String> {
        let mut metadata = serde_json::Map::new();
        for row in &self.metadata {
            let key = row.key.read(cx).value().to_string();
            let content = row.value.read(cx).value().to_string();
            if key.trim().is_empty() {
                if content.is_empty() {
                    continue;
                }
                return Err("Metadata keys must not be empty".into());
            }
            if metadata.insert(key, json!(content)).is_some() {
                return Err("Metadata keys must be unique".into());
            }
        }
        let metadata = Value::Object(metadata);
        if metadata != self.baseline_metadata {
            value["metadata"] = metadata;
        }
        if self.is_type {
            let mut fields = Vec::new();
            let mut names = BTreeSet::new();
            for row in &self.types {
                let mut field = row.original.clone();
                let name = row.name.read(cx).value().to_string();
                if name.trim().is_empty() || !names.insert(name.clone()) {
                    return Err("Type fields need nonempty unique names".into());
                }
                if row.original["name"].as_str().unwrap_or("") != name {
                    field["name"] = json!(name);
                }
                let type_id = row.type_id.read(cx).value().to_string();
                if row.original["isReference"].as_bool().unwrap_or(false) != row.reference {
                    field["isReference"] = json!(row.reference);
                }
                let constructor = row.collection.read(cx).value().to_string();
                if !["", "array", "list", "map"].contains(&constructor.as_str()) {
                    return Err("Collection type must be empty, array, list or map".into());
                }
                // Retain advanced expressions exactly until one of their visible type controls changes.
                let original_collection = row.original["typeExpression"]["constructorId"]
                    .as_str()
                    .unwrap_or("");
                let original_type = match original_collection {
                    "array" | "list" => {
                        row.original["typeExpression"]["arguments"][0]["typeId"].as_str()
                    }
                    "map" => row.original["typeExpression"]["arguments"][1]["typeId"].as_str(),
                    _ => row.original["typeId"].as_str(),
                }
                .unwrap_or("");
                if type_id != original_type {
                    field["typeId"] = json!(type_id);
                }
                let key = row.key_type.read(cx).value().to_string();
                let original_key = row.original["typeExpression"]["arguments"][0]["typeId"]
                    .as_str()
                    .unwrap_or("");
                if constructor != original_collection
                    || type_id != original_type
                    || (constructor == "map" && key != original_key)
                {
                    field["typeExpression"] = if constructor.is_empty() {
                        Value::Null
                    } else {
                        let mut arguments = vec![json!({"typeId":type_id})];
                        if constructor == "map" {
                            arguments.insert(0, json!({"typeId":key}));
                        }
                        json!({"constructorId":constructor,"arguments":arguments,"typeId":""})
                    };
                }
                fields.push(field);
            }
            let fields = json!(fields);
            if fields != self.baseline_types {
                value["typeDefinition"]["fields"] = fields;
            }
        }
        Ok(())
    }
}
impl Wirecraft {
    pub(super) fn render_tables(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut form = div().flex().flex_col().gap_2();
        if self.tables.is_type {
            form = form.child(
                div()
                    .text_color(crate::chrome::color(0x70d5c6))
                    .mt_3()
                    .child("TYPE FIELDS"),
            );
            for (i, row) in self.tables.types.iter().enumerate() {
                form = form.child(
                    div()
                        .id(("type-field", i))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .bg(crate::chrome::color(0x202b35))
                        .rounded_md()
                        .child(
                            div()
                                .text_xs()
                                .child("Name / Type ID (primitive or entity)"),
                        )
                        .child(Input::new(&row.name))
                        .child(Input::new(&row.type_id))
                        .child(
                            div()
                                .text_xs()
                                .child("Collection (array / list / map) / Key type ID"),
                        )
                        .child(Input::new(&row.collection))
                        .child(Input::new(&row.key_type))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    button(
                                        "field-reference",
                                        if row.reference {
                                            "☑ Reference"
                                        } else {
                                            "☐ Reference"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.tables.types[i].reference =
                                                !this.tables.types[i].reference;
                                            this.persist_ui(cx);
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(button("remove-field", "Remove field").on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.tables.types.remove(i);
                                        this.persist_ui(cx);
                                        cx.notify();
                                    }),
                                )),
                        ),
                );
            }
            form = form.child(button("add-type-field", "Add field").on_click(cx.listener(
                |this, _, w, cx| {
                    let name = format!("field_{}", this.tables.types.len() + 1);
                    this.tables.types.push(TypeRow {
                        name: input(&name, w, cx),
                        type_id: input("string", w, cx),
                        collection: input("", w, cx),
                        key_type: input("", w, cx),
                        reference: false,
                        original: json!({}),
                    });
                    this.persist_ui(cx);
                    cx.notify();
                },
            )));
        }
        form = form.child(
            div()
                .text_color(crate::chrome::color(0x70d5c6))
                .mt_3()
                .child("METADATA"),
        );
        for (i, row) in self.tables.metadata.iter().enumerate() {
            form = form.child(
                div()
                    .id(("metadata", i))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .bg(crate::chrome::color(0x202b35))
                    .rounded_md()
                    .child(Input::new(&row.key))
                    .child(Input::new(&row.value))
                    .child(button("remove-metadata", "Remove").on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.tables.metadata.remove(i);
                            this.persist_ui(cx);
                            cx.notify();
                        },
                    ))),
            );
        }
        form.child(button("add-metadata", "Add metadata").on_click(cx.listener(
            |this, _, w, cx| {
                this.tables.metadata.push(MetadataRow {
                    key: input("", w, cx),
                    value: input("", w, cx),
                });
                cx.notify();
            },
        )))
        .into_any_element()
    }
}
