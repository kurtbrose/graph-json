require 'json'

module GraphJSON
  MAX_REF_ID = 2**31

  module_function

  def primitive?(value)
    value.nil? || value.is_a?(String) || value.is_a?(Numeric) || value == true || value == false
  end

  def deflate(value)
    seen = {}
    counter = 0
    stack = [[value, nil, nil]]
    result = nil

    until stack.empty?
      node, parent, key = stack.pop
      out = nil

      if primitive?(node)
        out = node
      else
        entry = seen[node.object_id]
        if entry
          if entry[:ref_id].nil?
            counter += 1
            raise 'ref id overflow' if counter > MAX_REF_ID
            entry[:ref_id] = counter
            proxy = entry[:proxy]
            if proxy.is_a?(Array)
              proxy.unshift({ '#' => counter })
            else
              proxy['#'] = counter
            end
          end
          out = { '#' => entry[:ref_id] }
        elsif node.is_a?(Hash)
          proxy = {}
          seen[node.object_id] = { ref_id: nil, proxy: proxy }
          out = proxy
          node.to_a.reverse_each do |k, v|
            key2 = k
            key2 = "#" + key2 if key2.is_a?(String) && key2.match?(/^#+$/)
            stack << [v, proxy, key2]
          end
        elsif node.is_a?(Array)
          proxy = []
          seen[node.object_id] = { ref_id: nil, proxy: proxy }
          out = proxy
          node.reverse_each do |item|
            stack << [item, proxy, nil]
          end
        else
          raise TypeError, "Unsupported type: #{node.class}"
        end
      end

      if parent.nil?
        result = out
      elsif parent.is_a?(Array)
        if key.nil?
          parent << out
        else
          parent[key] = out
        end
      else
        parent[key] = out
      end
    end

    result
  end

  def inflate(value)
    ref_map = {}
    owners = {}
    stack = [[value, nil, nil]]
    result = nil

    until stack.empty?
      node, parent, key = stack.pop
      out = nil

      if primitive?(node)
        out = node
      elsif node.is_a?(Array)
        items = node
        if !items.empty? && items[0].is_a?(Hash) && items[0].keys == ['#']
          n = items[0]['#']
          raise 'invalid ref id' unless n.is_a?(Integer) && n > 0 && n <= MAX_REF_ID
          raise 'ref object with extra members' if owners[n]
          if ref_map.key?(n)
            out = ref_map[n]
            raise 'ref object with extra members' if items.length > 1
          else
            out = []
            ref_map[n] = out
            owners[n] = true
            items[1..].reverse_each do |item|
              stack << [item, out, nil]
            end
          end
        else
          out = []
          items.reverse_each do |item|
            stack << [item, out, nil]
          end
        end
      elsif node.is_a?(Hash)
        if node.keys == ['#']
          n = node['#']
          raise 'invalid ref id' unless n.is_a?(Integer) && n > 0 && n <= MAX_REF_ID
          out = ref_map[n] ||= {}
        else
          n = nil
          obj = node
          if obj.key?('#')
            n = obj['#']
            raise 'invalid ref id' unless n.is_a?(Integer) && n > 0 && n <= MAX_REF_ID
            obj = obj.reject { |k, _| k == '#' }
            raise 'ref object with extra members' if owners[n]
          end
          out = {}
          if n
            out = ref_map[n] ||= {}
            owners[n] = true
          end
          obj.to_a.reverse_each do |k, v|
            key2 = k
            key2 = key2[1..] if key2.is_a?(String) && key2.match?(/^#+$/)
            stack << [v, out, key2]
          end
        end
      else
        raise TypeError, "Unsupported type: #{node.class}"
      end

      if parent.nil?
        result = out
      elsif parent.is_a?(Array)
        if key.nil?
          parent << out
        else
          parent[key] = out
        end
      else
        parent[key] = out
      end
    end

    unresolved = ref_map.keys.reject { |n| owners[n] }
    raise 'unknown ref id' unless unresolved.empty?
    result
  end

  def dumps(value, *args)
    JSON.generate(deflate(value), *args)
  end

  def loads(str, **kwargs)
    inflate(JSON.parse(str, **kwargs))
  end
end
