import { readFileSync } from "node:fs";
import { Ajv2020 } from "ajv/dist/2020.js";
import { isAmfSharedRecipe, type AmfSharedRecipe } from "@vua/contracts";
import type { Json } from "./json.js";

export interface Product { productId: string; shopName?: string; variantName?: string }
export interface SelectionShare { kind: "vua.selection"; formatVersion: "0.1"; title: string; products: Product[] }
export type Share = AmfSharedRecipe | SelectionShare;

// Existing package maps require named entries governed by additionalProperties.
const ajv = new Ajv2020({ strict: true, strictRequired: false, allErrors: false, ownProperties: true });
const schemas = new Map([
  ["recipe:0.4", "recipe-v04.json"],
  ["recipe:0.5", "recipe-v05.json"],
  ["selection:0.1", "selection-v01.json"],
].map(([version, filename]) => [version!, ajv.compile(JSON.parse(readFileSync(new URL(`../schemas/${filename}`, import.meta.url), "utf8")))]));

/** Version dispatch stays explicit; schemas are bundled locally and never fetched. */
export function validateShare(value: Json): Share | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const selection = value.kind === "vua.selection";
  const validator = schemas.get(`${selection ? "selection" : "recipe"}:${String(value.formatVersion)}`);
  if (!validator?.(value)) return null;
  if (selection) return value as unknown as SelectionShare;
  return isAmfSharedRecipe(value) ? value : null;
}

export function productsOf(share: Share): Product[] {
  if ("kind" in share) return share.products;
  const products = new Map<string, Product>();
  for (const source of Object.values(share.sources)) {
    const product = typeof source === "string" ? { productId: source } : {
      productId: source.boothProductId,
      ...(source.shopName ? { shopName: source.shopName } : {}),
    };
    const previous = products.get(product.productId);
    if (!previous || (!previous.shopName && product.shopName)) products.set(product.productId, product);
  }
  return [...products.values()];
}
