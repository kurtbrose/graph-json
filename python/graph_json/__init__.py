import json

__all__ = ["deflate", "inflate", "dumps", "loads"]

_MAX_REF_ID = 2**31


def _is_primitive(value):
    return value is None or isinstance(value, (str, int, float, bool))


def deflate(value):
    """Deflate a Python object graph into Graph-JSON primitives."""
    seen = {}
    counter = 0
    stack = [(value, None, None)]
    result = None

    while stack:
        node, parent, key = stack.pop()
        if _is_primitive(node):
            out = node
        else:
            node_id = id(node)
            if node_id in seen:
                entry = seen[node_id]
                if entry["ref_id"] is None:
                    counter += 1
                    if counter > _MAX_REF_ID:
                        raise ValueError("ref id overflow")
                    entry["ref_id"] = counter
                    proxy = entry["proxy"]
                    if isinstance(proxy, dict):
                        proxy["#"] = counter
                    else:
                        proxy.insert(0, {"#": counter})
                out = {"#": entry["ref_id"]}
            elif isinstance(node, dict):
                proxy = {}
                seen[node_id] = {"ref_id": None, "proxy": proxy}
                out = proxy
                for k, v in reversed(list(node.items())):
                    key2 = k
                    if key2 and key2[0] == "#" and key2.count("#") == len(key2):
                        key2 = "#" + key2
                    stack.append((v, proxy, key2))
            elif isinstance(node, list):
                proxy = []
                seen[node_id] = {"ref_id": None, "proxy": proxy}
                out = proxy
                for item in reversed(node):
                    stack.append((item, proxy, None))
            else:
                raise TypeError(f"Unsupported type: {type(node)!r}")
        if parent is None:
            result = out
        else:
            if isinstance(parent, list):
                if key is None:
                    parent.append(out)
                else:
                    parent[key] = out
            else:
                parent[key] = out

    return result


def inflate(value):
    """Inflate a Graph-JSON structure back into a Python object graph."""
    ref_map = {}
    owners = set()

    def walk(node):
        if _is_primitive(node):
            return node
        if isinstance(node, list):
            items = node
            if items and isinstance(items[0], dict) and set(items[0].keys()) == {"#"}:
                n = items[0]["#"]
                if not isinstance(n, int) or n <= 0 or n > _MAX_REF_ID:
                    raise ValueError("invalid ref id")
                if n in owners:
                    raise ValueError("ref object with extra members")
                if n in ref_map:
                    out = ref_map[n]
                    if len(items) > 1:
                        raise ValueError("ref object with extra members")
                else:
                    out = []
                    ref_map[n] = out
                    owners.add(n)
                    for item in items[1:]:
                        out.append(walk(item))
                return out
            out_list = []
            for item in items:
                out_list.append(walk(item))
            return out_list
        if isinstance(node, dict):
            if set(node.keys()) == {"#"}:
                n = node["#"]
                if not isinstance(n, int) or n <= 0 or n > _MAX_REF_ID:
                    raise ValueError("invalid ref id")
                if n in ref_map:
                    return ref_map[n]
                placeholder = {}
                ref_map[n] = placeholder
                return placeholder
            n = None
            if "#" in node:
                n = node["#"]
                if not isinstance(n, int) or n <= 0 or n > _MAX_REF_ID:
                    raise ValueError("invalid ref id")
                node = {k: v for k, v in node.items() if k != "#"}
                if n in owners:
                    raise ValueError("ref object with extra members")
                if n in ref_map:
                    out = ref_map[n]
                else:
                    out = {}
                    ref_map[n] = out
                owners.add(n)
            else:
                out = {}
            for k, v in node.items():
                key = k
                if key and key[0] == "#" and key.count("#") == len(key):
                    key = key[1:]
                out[key] = walk(v)
            return out
        raise TypeError(f"Unsupported type: {type(node)!r}")

    result = walk(value)
    unresolved = set(ref_map.keys()) - owners
    if unresolved:
        raise ValueError("unknown ref id")
    return result


def dumps(value, **kwargs):
    return json.dumps(deflate(value), **kwargs)


def loads(s, **kwargs):
    return inflate(json.loads(s, **kwargs))
