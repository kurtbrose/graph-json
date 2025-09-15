const MAX_REF_ID = 2 ** 31;

function isPrimitive(value: any): boolean {
  return value === null || typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean';
}

export function deflate(value: any): any {
  const seen = new Map<any, { refId: number | null; proxy: any }>();
  let counter = 0;
  const stack: Array<{ node: any; parent: any; key: any }> = [{ node: value, parent: null, key: null }];
  let result: any = undefined;

  while (stack.length) {
    const { node, parent, key } = stack.pop()!;
    let out: any;

    if (isPrimitive(node)) {
      out = node;
    } else {
      const entry = seen.get(node);
      if (entry) {
        if (entry.refId === null) {
          counter += 1;
          if (counter > MAX_REF_ID) {
            throw new Error('ref id overflow');
          }
          entry.refId = counter;
          const proxy = entry.proxy;
          if (Array.isArray(proxy)) {
            proxy.unshift({ '#': counter });
          } else {
            proxy['#'] = counter;
          }
        }
        out = { '#': entry.refId };
      } else if (Array.isArray(node)) {
        const proxy: any[] = [];
        seen.set(node, { refId: null, proxy });
        out = proxy;
        for (let i = node.length - 1; i >= 0; i--) {
          stack.push({ node: node[i], parent: proxy, key: null });
        }
      } else if (typeof node === 'object') {
        const proxy: Record<string, any> = {};
        seen.set(node, { refId: null, proxy });
        out = proxy;
        const entries = Object.entries(node);
        for (let i = entries.length - 1; i >= 0; i--) {
          let [k, v] = entries[i];
          let key2 = k;
          if (/^#+$/.test(key2)) {
            key2 = '#' + key2;
          }
          stack.push({ node: v, parent: proxy, key: key2 });
        }
      } else {
        throw new TypeError(`Unsupported type: ${typeof node}`);
      }
    }

    if (parent === null) {
      result = out;
    } else if (Array.isArray(parent)) {
      parent.push(out);
    } else {
      parent[key] = out;
    }
  }

  return result;
}

export function inflate(value: any): any {
  const refMap = new Map<number, any>();
  const owners = new Set<number>();

  function walk(node: any): any {
    if (isPrimitive(node)) {
      return node;
    }
    if (Array.isArray(node)) {
      const items = node;
      if (
        items.length &&
        typeof items[0] === 'object' &&
        items[0] !== null &&
        Object.keys(items[0]).length === 1 &&
        '#' in items[0]
      ) {
        const n = (items[0] as any)['#'];
        if (!Number.isInteger(n) || n <= 0 || n > MAX_REF_ID) {
          throw new Error('invalid ref id');
        }
        if (owners.has(n)) {
          throw new Error('ref object with extra members');
        }
        let out: any;
        if (refMap.has(n)) {
          out = refMap.get(n);
          if (items.length > 1) {
            throw new Error('ref object with extra members');
          }
        } else {
          out = [];
          refMap.set(n, out);
          owners.add(n);
          for (let i = 1; i < items.length; i++) {
            out.push(walk(items[i]));
          }
        }
        return out;
      }
      return items.map(walk);
    }
    if (typeof node === 'object' && node !== null) {
      const keys = Object.keys(node);
      if (keys.length === 1 && keys[0] === '#') {
        const n = (node as any)['#'];
        if (!Number.isInteger(n) || n <= 0 || n > MAX_REF_ID) {
          throw new Error('invalid ref id');
        }
        const existing = refMap.get(n);
        if (existing) {
          return existing;
        }
        const placeholder: any = {};
        refMap.set(n, placeholder);
        return placeholder;
      }
      let n: number | undefined;
      let obj: Record<string, any> = node as any;
      if ('#' in obj) {
        const value = obj['#'];
        if (!Number.isInteger(value) || value <= 0 || value > MAX_REF_ID) {
          throw new Error('invalid ref id');
        }
        n = value as number;
        obj = { ...obj };
        delete obj['#'];
        if (owners.has(n)) {
          throw new Error('ref object with extra members');
        }
      }
      let out: any = {};
      if (n !== undefined) {
        if (refMap.has(n)) {
          out = refMap.get(n);
        } else {
          refMap.set(n, out);
        }
        owners.add(n);
      }
      for (const [k, v] of Object.entries(obj)) {
        let key = k;
        if (/^#+$/.test(key)) {
          key = key.slice(1);
        }
        out[key] = walk(v);
      }
      return out;
    }
    throw new TypeError(`Unsupported type: ${typeof node}`);
  }

  const result = walk(value);
  for (const n of refMap.keys()) {
    if (!owners.has(n)) {
      throw new Error('unknown ref id');
    }
  }
  return result;
}

export function dumps(value: any, replacer?: any, space?: string | number): string {
  return JSON.stringify(deflate(value), replacer, space);
}

export function loads(text: string): any {
  return inflate(JSON.parse(text));
}
