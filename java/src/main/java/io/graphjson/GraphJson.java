package io.graphjson;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.util.*;

/**
 * Graph-JSON encoder/decoder for Java using Jackson.
 */
public final class GraphJson {
    private GraphJson() {}

    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final int MAX_REF_ID = 2_147_483_647;

    private static boolean isPrimitive(Object v) {
        return v == null || v instanceof String || v instanceof Number || v instanceof Boolean;
    }

    private static boolean isEscapeKey(String k) {
        if (k.isEmpty()) return false;
        for (int i = 0; i < k.length(); i++) {
            if (k.charAt(i) != '#') return false;
        }
        return true;
    }

    public static Object deflate(Object value) {
        class Frame {
            Object node;
            Object parent;
            String key; // only for map parent
            Frame(Object node, Object parent, String key) {this.node = node; this.parent = parent; this.key = key;}
        }
        class Seen { Integer refId; Object proxy; Seen(Integer r, Object p) {refId = r; proxy = p;} }

        IdentityHashMap<Object, Seen> seen = new IdentityHashMap<>();
        Deque<Frame> stack = new ArrayDeque<>();
        stack.push(new Frame(value, null, null));
        int counter = 0;
        Object result = null;

        while (!stack.isEmpty()) {
            Frame f = stack.pop();
            Object node = f.node;
            Object out;
            if (isPrimitive(node)) {
                out = node;
            } else if (node instanceof Map) {
                Seen entry = seen.get(node);
                if (entry != null) {
                    if (entry.refId == null) {
                        counter++;
                        if (counter > MAX_REF_ID) throw new RuntimeException("ref id overflow");
                        entry.refId = counter;
                        Map<String,Object> proxy = castMap(entry.proxy);
                        proxy.put("#", counter);
                    }
                    out = makeRefObject(entry.refId);
                } else {
                    Map<String, Object> proxy = new LinkedHashMap<>();
                    seen.put(node, new Seen(null, proxy));
                    out = proxy;
                    @SuppressWarnings("unchecked")
                    Map<String,Object> map = (Map<String,Object>) node;
                    List<Map.Entry<String,Object>> entries = new ArrayList<>(map.entrySet());
                    Collections.reverse(entries);
                    for (Map.Entry<String,Object> e : entries) {
                        String k = e.getKey();
                        String k2 = isEscapeKey(k) ? "#" + k : k;
                        stack.push(new Frame(e.getValue(), proxy, k2));
                    }
                }
            } else if (node instanceof List) {
                Seen entry = seen.get(node);
                if (entry != null) {
                    if (entry.refId == null) {
                        counter++;
                        if (counter > MAX_REF_ID) throw new RuntimeException("ref id overflow");
                        entry.refId = counter;
                        List<Object> proxy = castList(entry.proxy);
                        proxy.add(0, makeRefObject(counter));
                    }
                    out = makeRefObject(entry.refId);
                } else {
                    List<Object> proxy = new ArrayList<>();
                    seen.put(node, new Seen(null, proxy));
                    out = proxy;
                    @SuppressWarnings("unchecked")
                    List<Object> list = (List<Object>) node;
                    for (int i = list.size() - 1; i >= 0; i--) {
                        stack.push(new Frame(list.get(i), proxy, null));
                    }
                }
            } else {
                throw new RuntimeException("Unsupported type: " + node.getClass());
            }

            if (f.parent == null) {
                result = out;
            } else if (f.parent instanceof List) {
                castList(f.parent).add(out);
            } else {
                castMap(f.parent).put(f.key, out);
            }
        }
        return result;
    }

    private static Map<String,Object> castMap(Object o) { @SuppressWarnings("unchecked") Map<String,Object> m = (Map<String,Object>) o; return m; }
    private static List<Object> castList(Object o) { @SuppressWarnings("unchecked") List<Object> l = (List<Object>) o; return l; }

    private static Map<String,Object> makeRefObject(int id) {
        Map<String,Object> m = new LinkedHashMap<>();
        m.put("#", id);
        return m;
    }

    private static int extractRefId(Object v) {
        if (v instanceof Number) {
            long n = ((Number) v).longValue();
            if (n > 0 && n <= MAX_REF_ID) return (int) n;
        }
        throw new RuntimeException("invalid ref id");
    }

    public static Object inflate(Object value) {
        Map<Integer,Object> refMap = new HashMap<>();
        Set<Integer> owners = new HashSet<>();

        Object result = walk(value, refMap, owners);
        for (Integer id : refMap.keySet()) {
            if (!owners.contains(id)) {
                throw new RuntimeException("unknown ref id");
            }
        }
        return result;
    }

    private static Object walk(Object node, Map<Integer,Object> refMap, Set<Integer> owners) {
        if (isPrimitive(node)) return node;
        if (node instanceof List) {
            List<?> items = (List<?>) node;
            if (!items.isEmpty() && items.get(0) instanceof Map && ((Map<?,?>) items.get(0)).size() == 1 && ((Map<?,?>) items.get(0)).containsKey("#")) {
                int id = extractRefId(((Map<?,?>) items.get(0)).get("#"));
                if (owners.contains(id)) throw new RuntimeException("ref object with extra members");
                if (refMap.containsKey(id)) {
                    if (items.size() > 1) throw new RuntimeException("ref object with extra members");
                    return refMap.get(id);
                } else {
                    List<Object> out = new ArrayList<>();
                    refMap.put(id, out);
                    owners.add(id);
                    for (int i = 1; i < items.size(); i++) {
                        out.add(walk(items.get(i), refMap, owners));
                    }
                    return out;
                }
            }
            List<Object> out = new ArrayList<>();
            for (Object item : items) {
                out.add(walk(item, refMap, owners));
            }
            return out;
        }
        if (node instanceof Map) {
            Map<?,?> map = (Map<?,?>) node;
            if (map.size() == 1 && map.containsKey("#")) {
                int id = extractRefId(map.get("#"));
                if (refMap.containsKey(id)) return refMap.get(id);
                Map<String,Object> placeholder = new LinkedHashMap<>();
                refMap.put(id, placeholder);
                return placeholder;
            }
            Object out;
            if (map.containsKey("#")) {
                int id = extractRefId(map.get("#"));
                if (owners.contains(id)) throw new RuntimeException("ref object with extra members");
                owners.add(id);
                if (refMap.containsKey(id)) {
                    out = refMap.get(id);
                } else {
                    out = new LinkedHashMap<String,Object>();
                    refMap.put(id, out);
                }
            } else {
                out = new LinkedHashMap<String,Object>();
            }
            Map<String,Object> outMap = castMap(out);
            for (Map.Entry<?,?> e : map.entrySet()) {
                String k = (String) e.getKey();
                if (k.equals("#")) continue;
                String key = isEscapeKey(k) ? k.substring(1) : k;
                outMap.put(key, walk(e.getValue(), refMap, owners));
            }
            return outMap;
        }
        throw new RuntimeException("Unsupported type: " + node.getClass());
    }

    public static String dumps(Object value) {
        try {
            Object v = deflate(value);
            return MAPPER.writeValueAsString(v);
        } catch (JsonProcessingException e) {
            throw new RuntimeException(e);
        }
    }

    public static Object loads(String s) {
        try {
            Object v = MAPPER.readValue(s, Object.class);
            return inflate(v);
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }
}
