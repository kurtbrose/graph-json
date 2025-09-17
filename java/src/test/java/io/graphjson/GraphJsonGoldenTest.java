package io.graphjson;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.*;

import static org.junit.Assert.*;

public class GraphJsonGoldenTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    private Object resolve(Object root, String pointer) {
        String[] parts = pointer.equals("/") ? new String[]{""} : pointer.substring(1).split("/");
        Object cur = root;
        for (String part : parts) {
            if (part.isEmpty()) continue;
            part = part.replace("~1", "/").replace("~0", "~");
            if (cur instanceof List) {
                cur = ((List<?>) cur).get(Integer.parseInt(part));
            } else {
                cur = ((Map<?,?>) cur).get(part);
            }
        }
        return cur;
    }

    @Test
    public void testGoldenCorrect() throws Exception {
        Path goldenPath = Paths.get("..", "tests", "golden.json").toAbsolutePath().normalize();
        Map<String,Object> golden = MAPPER.readValue(Files.newBufferedReader(goldenPath), new TypeReference<Map<String,Object>>(){});
        Map<String,Object> correct = castMap(golden.get("correct"));
        for (Map.Entry<String,Object> entry : correct.entrySet()) {
            Map<String,Object> caseMap = castMap(entry.getValue());
            Object doc = caseMap.get("doc");
            Object obj = GraphJson.loads(MAPPER.writeValueAsString(doc));
            List<List<String>> aliases = castListList(caseMap.get("aliases"));
            for (List<String> group : aliases) {
                Object first = resolve(obj, group.get(0));
                for (int i = 1; i < group.size(); i++) {
                    Object t = resolve(obj, group.get(i));
                    assertSame(first, t);
                }
            }
            Object expectKeysObj = caseMap.get("expect-keys");
            if (expectKeysObj != null) {
                Map<String,List<String>> expectKeys = castMapList(expectKeysObj);
                for (Map.Entry<String,List<String>> e : expectKeys.entrySet()) {
                    Object target = resolve(obj, e.getKey());
                    assertTrue(target instanceof Map);
                    Map<?,?> targetMap = (Map<?,?>) target;
                    for (String k : e.getValue()) {
                        assertTrue("missing key " + k, targetMap.containsKey(k));
                    }
                }
            }
        }
    }

    @Test
    public void testGoldenInvalid() throws Exception {
        Path goldenPath = Paths.get("..", "tests", "golden.json").toAbsolutePath().normalize();
        Map<String,Object> golden = MAPPER.readValue(Files.newBufferedReader(goldenPath), new TypeReference<Map<String,Object>>(){});
        Map<String,Object> invalid = castMap(golden.get("invalid"));
        for (Map.Entry<String,Object> entry : invalid.entrySet()) {
            String name = entry.getKey();
            Map<String,Object> caseMap = castMap(entry.getValue());
            Map<String,Object> doc = new LinkedHashMap<>(castMap(caseMap.get("doc")));
            if (name.equals("ref-with-extras")) {
                Map.Entry<String,Object> first = doc.entrySet().iterator().next();
                Map<String,Object> firstObj = castMap(first.getValue());
                int refId = ((Number) firstObj.get("#")).intValue();
                Map<String,Object> owner = new LinkedHashMap<>();
                owner.put("#", refId);
                owner.put("v", 0);
                doc.put("owner", owner);
            }
            String json = MAPPER.writeValueAsString(doc);
            boolean failed = false;
            try {
                GraphJson.loads(json);
            } catch (RuntimeException e) {
                failed = true;
            }
            assertTrue("case " + name + " should fail", failed);
        }
    }

    @SuppressWarnings("unchecked")
    private static Map<String,Object> castMap(Object o) { return (Map<String,Object>) o; }
    @SuppressWarnings("unchecked")
    private static List<List<String>> castListList(Object o) { return (List<List<String>>) o; }
    @SuppressWarnings("unchecked")
    private static Map<String,List<String>> castMapList(Object o) { return (Map<String,List<String>>) o; }
}
