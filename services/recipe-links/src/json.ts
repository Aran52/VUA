import { createHash } from "node:crypto";

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export const MAX_BODY_BYTES = 64 * 1024;
export const MAX_DEPTH = 32;
export const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

function unicode(text: string): string {
  if (/[\uD800-\uDFFF]/u.test(text)) throw new Error("Invalid Unicode scalar value");
  return text;
}

/** JCS serializes keys directly: JSON.stringify of a sorted object reorders integer keys. */
export function canonicalize(value: Json): string {
  if (value === null || typeof value === "boolean") return JSON.stringify(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("Non-finite number");
    return JSON.stringify(value);
  }
  if (typeof value === "string") return JSON.stringify(unicode(value));
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(",")}]`;
  return `{${Object.keys(value).sort().map(key => `${JSON.stringify(unicode(key))}:${canonicalize(value[key]!)}`).join(",")}}`;
}

export function shareCode(canonical: string): string {
  const digest = createHash("sha256").update(canonical, "utf8").digest("hex");
  const prefix = BigInt(`0x${digest.slice(0, 15)}`);
  let code = "";
  for (let shift = 55n; shift >= 0n; shift -= 5n) code += ALPHABET[Number((prefix >> shift) & 31n)];
  return code;
}

export function normalizeCode(code: string): string | null {
  if (!/^[A-Za-z0-9]{12}$/u.test(code)) return null;
  const upper = code.toUpperCase();
  return new RegExp(`^[${ALPHABET}]{12}$`, "u").test(upper) ? upper : null;
}

/** Bounded JSON parsing also rejects duplicate decoded keys before they can be lost. */
export function parseJson(text: string, maxDepth = MAX_DEPTH): Json {
  let index = 0;
  const whitespace = () => { while (/[ \t\r\n]/u.test(text[index] ?? "x")) index++; };
  const fail = (): never => { throw new Error("Invalid JSON"); };
  function string(): string {
    const start = index++;
    while (index < text.length) {
      const char = text[index++];
      if (char === "\\") index++;
      else if (char === '"') return unicode(JSON.parse(text.slice(start, index)) as string);
    }
    return fail();
  }
  function value(depth: number): Json {
    if (depth > maxDepth) return fail();
    whitespace();
    const char = text[index];
    if (char === '"') return string();
    if (char === "{" || char === "[") {
      index++;
      const object: { [key: string]: Json } = Object.create(null) as { [key: string]: Json };
      const array: Json[] = [];
      const end = char === "{" ? "}" : "]";
      whitespace();
      if (text[index] === end) { index++; return char === "{" ? object : array; }
      while (true) {
        whitespace();
        if (char === "{") {
          if (text[index] !== '"') return fail();
          const key = string();
          if (Object.hasOwn(object, key)) return fail();
          whitespace();
          if (text[index++] !== ":") return fail();
          object[key] = value(depth + 1);
        } else array.push(value(depth + 1));
        whitespace();
        if (text[index] === end) { index++; return char === "{" ? object : array; }
        if (text[index++] !== ",") return fail();
      }
    }
    const token = /^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/u.exec(text.slice(index))?.[0];
    if (!token) return fail();
    index += token.length;
    const parsed = JSON.parse(token) as Json;
    if (typeof parsed === "number" && !Number.isFinite(parsed)) return fail();
    return parsed;
  }
  const result = value(1);
  whitespace();
  if (index !== text.length) return fail();
  return result;
}
