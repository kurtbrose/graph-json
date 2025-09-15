# Graph‑JSON Specification (v0.1.0)

> \*\*Status \*\* — *Draft for community review, September 2025*.  Please send issues or pull‑requests to [https://github.com/yourname/graph‑json](https://github.com/yourname/graph‑json).

---

## 1  Scope & Motivation

Graph‑JSON defines a **minimal extension** to vanilla JSON that allows a single document to encode a directed graph – including cycles and shared sub‑objects – **without using external references, multiple passes, or non‑JSON tokens**.

### 1.1  In‑Scope

* JSON objects and arrays whose elements are themselves valid JSON values.
* Preservation of **object identity** (two references to the same object stay the same after a round‑trip).
* Encoding/decoding in a **single streaming pass**.
* Deterministic, runtime‑agnostic format suitable for wire protocols and storage.

### 1.2  Out‑of‑Scope

* Binary blobs, dates, bigints beyond IEEE‑754, custom classes.
* Cross‑document references or URI resolution.
* Security concerns unrelated to cycles (e.g. JSON injection, transport layer).

---

## 2  Conventions & Terminology

* **JSON value** — any entity permitted by \[RFC 8259] section 3.
* **Node** — a JSON object or array considered by identity (memory address).
* **Edge** — a key→value pair (for objects) or index→value (for arrays).
* **First occurrence** — first time an encoder encounters a node while walking the graph.
* **Back‑edge** — any subsequent encounter of the *same* node.
* **Ref‑ID** — positive integer assigned on the first back‑edge to symbolise that node.
* **Escape key** — a key that begins with one or more `#` characters **and nothing else**.

---

## 3  Serialization Rules (Deflation)

### 3.1  Overview

1. Walk the input graph depth‑first, maintaining a *seen‑table* mapping `pointer → { ref_id?, proxy }`.
2. For a primitive (null, bool, number, string) **emit it verbatim**.
3. For the *first occurrence* of an object/array:

   * Create a **proxy** (same container type, initially empty) and store it in *seen‑table*.
   * Recursively serialize each field/element into the **same proxy**.
   * For every key that is an *escape key* (matches `^#+$`) **prefix one extra `#`**.
4. For every *back‑edge*:

   * If the node has **no `ref_id` yet**, assign `ref_id ← ++counter`.
   * **Emit** the JSON object `{ "#": <ref_id> }` (exact one‑key form).

### 3.2  Streaming Property

Because each node is serialized at most once and references are forward‑only `{ "#": n }`, the encoder operates in **O(nodes + edges)** time and memory, and may write output as it walks – suitable for SAX/DOM hybrids.

### 3.3  Escaping Detail

Keys that already consist solely of `#` characters would collide with the reference key.  They must be escaped by **adding one leading `#`** during deflation and **removing exactly one** during inflation.

*Example*

```json
{ "#": "value", "##": "value" }   →   { "##": "value", "###": "value" }
```

---

## 4  Deserialization Rules (Inflation)

### 4.1  Overview

1. Walk the JSON tree depth‑first, maintaining a map `ref_id → node`.
2. For a primitive, **return it**.
3. For an object **that contains a top‑level key `"#"`**:

   * Let `n ← value of "#"` (must be positive integer).
   * If `n` **exists** in the map, return the mapped node.
   * Otherwise, create an **empty placeholder** of the same container type, store it in the map, then merge the remaining members (after unescaping) into it, and return the placeholder.
4. For any other object/array:

   * Create an output container of the same type.
   * Recursively inflate each member and insert into the container (after unescaping keys).

### 4.2  Correctness Criteria

Inflate(Deflate(X)) must satisfy the following for all JSON graphs X:

* **Structure equality** — the resulting tree is isomorphic to X.
* **Identity preservation** — `is` relationships between nodes in X are preserved.
* **No duplicates** — each ref‑ID corresponds to exactly one node.

---

## 5  Semantic Profile (no new grammar)

Graph-JSON **does not add** any new JSON syntax. Documents are plain RFC 8259 JSON. Graph-JSON defines an **interpretation** of a particular object shape:

```
REF-OBJECT := an object that has a single member named "#" whose value is an integer in the range [1, 2_147_483_647]
```

When a conforming decoder encounters a REF-OBJECT, it treats it as a **reference to another node within the same document**, identified by that integer (the *ref-id*). All other JSON is interpreted per RFC 8259 with no changes.

---

## 6  Examples

### 6.1  Self‑loop

```json
# Input (Python notation)
a = {}
a["self"] = a
```

**Deflated**

```json
{ "self": { "#": 1 } }
```

### 6.2  Shared Sub‑object

```json
# Input
root = { "left": {}, "right": {} }
root["left"]["buddy"] = root["right"]
root["right"]["buddy"] = root["left"]
```

**Deflated**

```json
{
  "left":  { "buddy": { "#": 1 } },
  "right": { "buddy": { "#": 0 } }
}
```

(Implicit ref‑ID assignment order shown.)

### 6.3  Escaping Edge Case

```json
{ "#": "literal hash", "##": "double hash" }
```

Becomes

```json
{ "##": "literal hash", "###": "double hash" }
```

Round‑trips losslessly.

---

## 7  Conformance Requirements

* An **encoder** MUST emit only the forms described in §3.
* An **encoder** MUST assign ref-IDs starting from 1 and increment by 1, and MUST keep each ref-ID within **\[1, 2\_147\_483\_647]**.
* A **decoder** MUST reject:

  * Non-integer, non-positive, or out-of-range (`> 2_147_483_647`) `"#"` values.
  * Duplicate ref-IDs.
  * `REF-OBJECT` that contains additional keys after inflation (i.e. circular self-reference malformed).
* A conforming **implementation** MUST pass all examples in `/examples` and the property-based round-trip tests defined in `/tests`.

---

## 8  Security Considerations

* **Cycle bombs** – Decoders SHOULD fail with a clear error once a configurable limit on total nodes or maximum depth is exceeded.
* **Key prefix injection** – Data producers must escape `^#+$` keys; otherwise they risk collisions.
* **Ref-ID bounds** – Implementations SHOULD cap ref-IDs at **2\_147\_483\_647 (2³¹−1)**. Encoders MUST NOT exceed this range; decoders SHOULD reject larger values.

---

## 9  IANA & Media Type

This document registers the media type `application/graph‑json`.

```
Type name:  application
Subtype:    graph‑json
Encoding:   binary (UTF‑8)
Extensions: .gjson .graphjson
```

---

## 10  Reference Implementations

| Language                | Package                       | Notes                    |
| ----------------------- | ----------------------------- | ------------------------ |
| Python                  | `graph‑json` on PyPI          | Reference oracle, MIT    |
| JavaScript / TypeScript | `graph‑json` on npm           | Zero‑dependency, ESM+CJS |
| Rust                    | `graph‑json` crate            | `serde` feature flag     |
| JVM                     | `io.graphjson:graphjson`      | Includes Jackson module  |
| C                       | Single header + cJSON adapter | BSD‑2‑Clause             |

---

## 11  Change Log

* **v0.1.0** — Initial public draft.

---

## 12  References

* \[RFC 8259] Crockford, JSON Data Interchange Format, 2017.
* \[JSON Pointer] RFC 6901, 2013.
* \[JSON Schema] IETF draft 2020‑12.

---
