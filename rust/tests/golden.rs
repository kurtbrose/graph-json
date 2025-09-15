use std::fs;
use std::rc::Rc;

use graph_json as gj;
use gj::GraphValue;
use serde_json::Value;

fn resolve(root: GraphValue, pointer: &str) -> GraphValue {
    let mut cur = root;
    if pointer != "/" {
        for part in pointer.trim_start_matches('/').split('/') {
            let part = part.replace("~1", "/").replace("~0", "~");
            let next = {
                let borrow = cur.borrow();
                match &*borrow {
                    gj::GNode::Array(gj::GArray(items)) => {
                        let idx: usize = part.parse().unwrap();
                        items[idx].clone()
                    }
                    gj::GNode::Object(gj::GObject(map)) => map.get(&part).unwrap().clone(),
                    _ => panic!("not container"),
                }
            };
            cur = next;
        }
    }
    cur
}

fn load_golden() -> Value {
    let text = fs::read_to_string("../tests/golden.json").unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn test_correct_cases() {
    let golden = load_golden();
    let correct = golden.get("correct").unwrap().as_object().unwrap();
    for (name, case) in correct {
        let doc = case.get("doc").unwrap();
        let obj = gj::inflate(doc).unwrap();
        for group in case.get("aliases").unwrap().as_array().unwrap() {
            let arr = group.as_array().unwrap();
            let mut iter = arr.iter();
            let first = resolve(obj.clone(), iter.next().unwrap().as_str().unwrap());
            for p in iter {
                let target = resolve(obj.clone(), p.as_str().unwrap());
                assert!(Rc::ptr_eq(&first, &target), "alias mismatch in {}", name);
            }
        }
        if let Some(expect) = case.get("expect-keys") {
            let map = expect.as_object().unwrap();
            for (path, keys_val) in map {
                let target = resolve(obj.clone(), path);
                let json = gj::to_json(&target);
                let obj_map = json.as_object().unwrap();
                for k in keys_val.as_array().unwrap() {
                    assert!(obj_map.contains_key(k.as_str().unwrap()), "missing key {} in {}", k.as_str().unwrap(), name);
                }
            }
        }
    }
}

#[test]
fn test_invalid_cases() {
    let golden = load_golden();
    let invalid = golden.get("invalid").unwrap().as_object().unwrap();
    for (name, case) in invalid {
        let mut doc = case.get("doc").unwrap().clone();
        if name == "ref-with-extras" {
            if let Value::Object(ref mut m) = doc {
                let ref_id = m.values().next().unwrap().get("#").unwrap().as_i64().unwrap();
                m.insert("owner".to_string(), serde_json::json!({"#": ref_id, "v": 0}));
            }
        }
        assert!(gj::inflate(&doc).is_err(), "invalid case {} succeeded", name);
    }
}
