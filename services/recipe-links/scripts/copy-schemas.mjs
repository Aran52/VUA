import { copyFile, mkdir } from "node:fs/promises";

const destination = new URL("../dist/schemas/", import.meta.url);
await mkdir(destination, { recursive: true });
for (const [source, name] of [
  ["amf-recipe/v0.4/share.schema.json", "recipe-v04.json"],
  ["amf-recipe/v0.5/share.schema.json", "recipe-v05.json"],
  ["selection-share/v0.1/share.schema.json", "selection-v01.json"],
]) {
  await copyFile(new URL(`../../../schemas/${source}`, import.meta.url), new URL(name, destination));
}
