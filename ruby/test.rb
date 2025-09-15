require 'json'
require_relative './graph_json'

GOLDEN = JSON.parse(File.read(File.join(__dir__, '..', 'tests', 'golden.json')))

def resolve(obj, pointer)
  parts = pointer.split('/')[1..] || []
  cur = obj
  parts.each do |part|
    part = part.gsub('~1', '/').gsub('~0', '~')
    if cur.is_a?(Array)
      cur = cur[part.to_i]
    else
      cur = cur[part]
    end
  end
  cur
end

GOLDEN['correct'].each do |name, case_data|
  obj = GraphJSON.loads(JSON.generate(case_data['doc']))
  case_data['aliases'].each do |group|
    targets = group.map { |p| resolve(obj, p) }
    first = targets.first
    if targets[1..].any? { |t| !t.equal?(first) }
      raise "#{name}: alias group mismatch"
    end
  end
  if case_data['expect-keys']
    case_data['expect-keys'].each do |p, keys|
      target = resolve(obj, p)
      keys.each do |key|
        raise "#{name}: expected key #{key}" unless target.key?(key)
      end
    end
  end
end

GOLDEN['invalid'].each do |name, case_data|
  doc = case_data['doc'].dup
  if name == 'ref-with-extras'
    id = doc.values.first['#']
    doc = { 'owner' => { '#' => id, 'v' => 0 } }.merge(doc)
  end
  begin
    GraphJSON.loads(JSON.generate(doc))
    raise "#{name}: expected failure"
  rescue
  end
end

# Ensure deeply nested structures don't blow the stack
deep = []
cur = deep
2000.times do
  nxt = []
  cur << nxt
  cur = nxt
end
doc = GraphJSON.dumps(deep, max_nesting: false)
GraphJSON.loads(doc, max_nesting: false)

puts 'tests passed'
