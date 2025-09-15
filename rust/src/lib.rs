use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use serde_json::{self, Map, Number, Value as JsonValue};

pub const MAX_REF_ID: i32 = 2_147_483_647;

#[derive(Clone)]
pub enum GNode {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(GArray),
    Object(GObject),
}

#[derive(Clone)]
pub struct GArray(pub Vec<GraphValue>);

#[derive(Clone)]
pub struct GObject(pub BTreeMap<String, GraphValue>);

pub type GraphValue = Rc<RefCell<GNode>>;

fn new_null() -> GraphValue {
    Rc::new(RefCell::new(GNode::Null))
}

fn new_bool(b: bool) -> GraphValue {
    Rc::new(RefCell::new(GNode::Bool(b)))
}

fn new_number(n: Number) -> GraphValue {
    Rc::new(RefCell::new(GNode::Number(n)))
}

fn new_string(s: String) -> GraphValue {
    Rc::new(RefCell::new(GNode::String(s)))
}

fn new_array() -> GraphValue {
    Rc::new(RefCell::new(GNode::Array(GArray(Vec::new()))))
}

fn new_object() -> GraphValue {
    Rc::new(RefCell::new(GNode::Object(GObject(BTreeMap::new()))))
}

fn clone_primitive(node: &GraphValue) -> GraphValue {
    match &*node.borrow() {
        GNode::Null => new_null(),
        GNode::Bool(b) => new_bool(*b),
        GNode::Number(n) => new_number(n.clone()),
        GNode::String(s) => new_string(s.clone()),
        _ => unreachable!(),
    }
}

fn make_ref_object(id: i32) -> GraphValue {
    let mut map = BTreeMap::new();
    map.insert("#".to_string(), new_number(Number::from(id as i64)));
    Rc::new(RefCell::new(GNode::Object(GObject(map))))
}

pub fn to_json(node: &GraphValue) -> JsonValue {
    match &*node.borrow() {
        GNode::Null => JsonValue::Null,
        GNode::Bool(b) => JsonValue::Bool(*b),
        GNode::Number(n) => JsonValue::Number(n.clone()),
        GNode::String(s) => JsonValue::String(s.clone()),
        GNode::Array(GArray(items)) => {
            let vec = items.iter().map(|v| to_json(v)).collect();
            JsonValue::Array(vec)
        }
        GNode::Object(GObject(map)) => {
            let mut obj = Map::new();
            for (k, v) in map.iter() {
                obj.insert(k.clone(), to_json(v));
            }
            JsonValue::Object(obj)
        }
    }
}

pub fn from_json(value: &JsonValue) -> GraphValue {
    match value {
        JsonValue::Null => new_null(),
        JsonValue::Bool(b) => new_bool(*b),
        JsonValue::Number(n) => new_number(n.clone()),
        JsonValue::String(s) => new_string(s.clone()),
        JsonValue::Array(arr) => {
            let gv = new_array();
            if let GNode::Array(GArray(ref mut vec)) = &mut *gv.borrow_mut() {
                for item in arr {
                    vec.push(from_json(item));
                }
            }
            gv
        }
        JsonValue::Object(map) => {
            let gv = new_object();
            if let GNode::Object(GObject(ref mut m)) = &mut *gv.borrow_mut() {
                for (k, v) in map {
                    m.insert(k.clone(), from_json(v));
                }
            }
            gv
        }
    }
}

struct SeenEntry {
    ref_id: Option<i32>,
    proxy: GraphValue,
}

fn is_escape_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c == '#')
}

pub fn deflate(value: &GraphValue) -> Result<JsonValue, String> {
    let mut seen: HashMap<usize, SeenEntry> = HashMap::new();
    let mut counter: i32 = 0;
    let mut result: Option<GraphValue> = None;
    let mut stack: Vec<(GraphValue, Option<GraphValue>, Option<String>)> =
        vec![(value.clone(), None, None)];

    while let Some((node, parent, key)) = stack.pop() {
        let out: GraphValue;
        match &*node.borrow() {
            GNode::Null | GNode::Bool(_) | GNode::Number(_) | GNode::String(_) => {
                out = clone_primitive(&node);
            }
            GNode::Object(GObject(map)) => {
                let ptr = Rc::as_ptr(&node) as usize;
                if let Some(entry) = seen.get_mut(&ptr) {
                    if entry.ref_id.is_none() {
                        counter += 1;
                        if counter > MAX_REF_ID {
                            return Err("ref id overflow".into());
                        }
                        entry.ref_id = Some(counter);
                        if let GNode::Object(GObject(ref mut m)) = &mut *entry.proxy.borrow_mut() {
                            m.insert("#".to_string(), new_number(Number::from(counter as i64)));
                        }
                    }
                    out = make_ref_object(entry.ref_id.unwrap());
                } else {
                    let proxy = new_object();
                    seen.insert(ptr, SeenEntry { ref_id: None, proxy: proxy.clone() });
                    out = proxy.clone();
                    let mut entries: Vec<(String, GraphValue)> =
                        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    for (k, v) in entries.into_iter().rev() {
                        let key2 = if is_escape_key(&k) {
                            format!("#{}", k)
                        } else {
                            k
                        };
                        stack.push((v, Some(proxy.clone()), Some(key2)));
                    }
                }
            }
            GNode::Array(GArray(items)) => {
                let ptr = Rc::as_ptr(&node) as usize;
                if let Some(entry) = seen.get_mut(&ptr) {
                    if entry.ref_id.is_none() {
                        counter += 1;
                        if counter > MAX_REF_ID {
                            return Err("ref id overflow".into());
                        }
                        entry.ref_id = Some(counter);
                        if let GNode::Array(GArray(ref mut arr)) = &mut *entry.proxy.borrow_mut() {
                            arr.insert(0, make_ref_object(counter));
                        }
                    }
                    out = make_ref_object(entry.ref_id.unwrap());
                } else {
                    let proxy = new_array();
                    seen.insert(ptr, SeenEntry { ref_id: None, proxy: proxy.clone() });
                    out = proxy.clone();
                    for item in items.iter().rev() {
                        stack.push((item.clone(), Some(proxy.clone()), None));
                    }
                }
            }
        }

        if let Some(parent) = parent {
            match &mut *parent.borrow_mut() {
                GNode::Array(GArray(ref mut vec)) => vec.push(out),
                GNode::Object(GObject(ref mut m)) => {
                    m.insert(key.unwrap(), out);
                }
                _ => unreachable!(),
            }
        } else {
            result = Some(out);
        }
    }

    Ok(to_json(&result.unwrap()))
}

fn extract_ref_id(v: &JsonValue) -> Result<i32, String> {
    if let JsonValue::Number(n) = v {
        if let Some(id) = n.as_i64() {
            if id > 0 && id <= MAX_REF_ID as i64 {
                return Ok(id as i32);
            }
        }
    }
    Err("invalid ref id".into())
}

pub fn inflate(value: &JsonValue) -> Result<GraphValue, String> {
    fn walk(
        node: &JsonValue,
        ref_map: &mut HashMap<i32, GraphValue>,
        owners: &mut HashSet<i32>,
    ) -> Result<GraphValue, String> {
        match node {
            JsonValue::Null => Ok(new_null()),
            JsonValue::Bool(b) => Ok(new_bool(*b)),
            JsonValue::Number(n) => Ok(new_number(n.clone())),
            JsonValue::String(s) => Ok(new_string(s.clone())),
            JsonValue::Array(items) => {
                if let Some(JsonValue::Object(first)) = items.get(0) {
                    if first.len() == 1 && first.contains_key("#") {
                        let id = extract_ref_id(&first["#"])?;
                        if owners.contains(&id) {
                            return Err("ref object with extra members".into());
                        }
                        if let Some(existing) = ref_map.get(&id) {
                            if items.len() > 1 {
                                return Err("ref object with extra members".into());
                            }
                            return Ok(existing.clone());
                        } else {
                            let out = new_array();
                            ref_map.insert(id, out.clone());
                            owners.insert(id);
                            for item in items.iter().skip(1) {
                                let child = walk(item, ref_map, owners)?;
                                if let GNode::Array(GArray(ref mut vec)) = &mut *out.borrow_mut() {
                                    vec.push(child);
                                }
                            }
                            return Ok(out);
                        }
                    }
                }
                let out = new_array();
                for item in items {
                    let child = walk(item, ref_map, owners)?;
                    if let GNode::Array(GArray(ref mut vec)) = &mut *out.borrow_mut() {
                        vec.push(child);
                    }
                }
                Ok(out)
            }
            JsonValue::Object(map) => {
                if map.len() == 1 && map.contains_key("#") {
                    let id = extract_ref_id(&map["#"])?;
                    if let Some(existing) = ref_map.get(&id) {
                        return Ok(existing.clone());
                    } else {
                        let placeholder = new_object();
                        ref_map.insert(id, placeholder.clone());
                        return Ok(placeholder);
                    }
                }
                let out = if let Some(n_val) = map.get("#") {
                    let id = extract_ref_id(n_val)?;
                    if owners.contains(&id) {
                        return Err("ref object with extra members".into());
                    }
                    owners.insert(id);
                    if let Some(existing) = ref_map.get(&id) {
                        existing.clone()
                    } else {
                        let o = new_object();
                        ref_map.insert(id, o.clone());
                        o
                    }
                } else {
                    new_object()
                };
                for (k, v) in map.iter() {
                    if k == "#" {
                        continue;
                    }
                    let mut key = k.clone();
                    if is_escape_key(&key) {
                        key.remove(0);
                    }
                    let child = walk(v, ref_map, owners)?;
                    if let GNode::Object(GObject(ref mut m)) = &mut *out.borrow_mut() {
                        m.insert(key, child);
                    }
                }
                Ok(out)
            }
        }
    }

    let mut ref_map: HashMap<i32, GraphValue> = HashMap::new();
    let mut owners: HashSet<i32> = HashSet::new();
    let result = walk(value, &mut ref_map, &mut owners)?;
    for id in ref_map.keys() {
        if !owners.contains(id) {
            return Err("unknown ref id".into());
        }
    }
    Ok(result)
}

pub fn dumps(value: &GraphValue) -> Result<String, String> {
    let v = deflate(value)?;
    serde_json::to_string(&v).map_err(|e| e.to_string())
}

pub fn loads(s: &str) -> Result<GraphValue, String> {
    let v: JsonValue = serde_json::from_str(s).map_err(|e| e.to_string())?;
    inflate(&v)
}
