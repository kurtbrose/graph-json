import json
from pathlib import Path
import sys
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import graph_json as gj

def _resolve(obj, pointer):
    parts = pointer.lstrip('/').split('/') if pointer != '/' else ['']
    cur = obj
    for part in parts:
        if part == '':
            continue
        part = part.replace('~1', '/').replace('~0', '~')
        if isinstance(cur, list):
            cur = cur[int(part)]
        else:
            cur = cur[part]
    return cur

with open(Path(__file__).with_name('golden.json')) as f:
    GOLDEN = json.load(f)

@pytest.mark.parametrize('name,case', GOLDEN['correct'].items())
def test_correct(name, case):
    obj = gj.loads(json.dumps(case['doc']))
    for group in case['aliases']:
        targets = [_resolve(obj, p) for p in group]
        first = targets[0]
        assert all(t is first for t in targets[1:])
    for path, keys in case.get('expect-keys', {}).items():
        target = _resolve(obj, path)
        for key in keys:
            assert key in target

@pytest.mark.parametrize('name,case', GOLDEN['invalid'].items())
def test_invalid(name, case):
    doc = dict(case['doc'])
    if name == 'ref-with-extras':
        ref_id = next(iter(doc.values()))['#']
        doc = {'owner': {'#': ref_id, 'v': 0}, **doc}
    with pytest.raises(Exception):
        gj.loads(json.dumps(doc))
