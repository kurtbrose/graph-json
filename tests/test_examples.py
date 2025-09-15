import graph_json as gj


def test_self_loop():
    a = {}
    a["self"] = a
    data = gj.dumps(a)
    b = gj.loads(data)
    assert b is b["self"]


def test_shared_subobject():
    left = {}
    right = {}
    left["buddy"] = right
    right["buddy"] = left
    root = {"left": left, "right": right}
    data = gj.dumps(root)
    obj = gj.loads(data)
    assert obj["left"]["buddy"] is obj["right"]
    assert obj["right"]["buddy"] is obj["left"]
