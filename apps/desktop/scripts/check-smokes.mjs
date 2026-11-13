// Every smoke either runs in CI or says why it stays on a developer's machine, so none goes stale unnoticed.
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scripts = path.dirname(fileURLToPath(import.meta.url));
const workflows = path.resolve(scripts, "../../../.github/workflows");
const workflowText = readdirSync(workflows).map(name => readFileSync(path.join(workflows, name), "utf8")).join("\n");
const packageScripts = JSON.parse(readFileSync(path.resolve(scripts, "../package.json"), "utf8")).scripts;
const local = JSON.parse(readFileSync(path.join(scripts, "smokes-local.json"), "utf8"));

const smokes = readdirSync(scripts).filter(name => /^smoke-.*\.mjs$/u.test(name)).map(name => name.slice(6, -4));
const inCi = name => workflowText.includes(`smoke:${name}`) || workflowText.includes(`scripts/smoke-${name}.mjs`);
const problems = [];
for (const name of smokes) {
  if (inCi(name) && Object.hasOwn(local, name)) problems.push(`${name} runs in CI and is also listed as local`);
  if (!inCi(name) && !local[name]?.trim()) problems.push(`${name} neither runs in CI nor says why it stays local (scripts/smokes-local.json)`);
  if (inCi(name) && workflowText.includes(`smoke:${name}`) && !packageScripts[`smoke:${name}`]) problems.push(`${name} runs in CI without a package script`);
}
for (const name of Object.keys(local)) if (!smokes.includes(name)) problems.push(`${name} is listed as local but has no script`);

if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`smokes: ${smokes.length - Object.keys(local).length} in CI, ${Object.keys(local).length} local`);
