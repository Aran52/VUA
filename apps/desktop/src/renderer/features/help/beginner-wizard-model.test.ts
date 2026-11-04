import assert from "node:assert/strict";
import { test } from "vitest";
import { isPageId } from "../../app/nav-model.ts";
import { guidePage } from "../guide/guide-content.ts";
import { BEGINNER_CONCEPTS, parseBeginnerBookmark, serializeBeginnerBookmark } from "./beginner-wizard-model.ts";

test("the beginner path starts with VRChat and links real explanations to setup destinations", () => {
  assert.equal(BEGINNER_CONCEPTS[0]?.target.section, "what-is-vrchat");
  for (const concept of BEGINNER_CONCEPTS) {
    const section = guidePage(concept.target.topic).sections.find(section => section.id === concept.target.section);
    assert.ok(section && section.paragraphs.length > 0, concept.id);
    assert.ok(isPageId(concept.setup), concept.id);
  }
  assert.equal(new Set(BEGINNER_CONCEPTS.map(concept => concept.id)).size, BEGINNER_CONCEPTS.length);
});

test("a concept bookmark cannot restore an unknown version, retired concept or task state", () => {
  for (const raw of [null, "broken", "null", '{"version":2,"concept":"upload"}',
    '{"version":1,"concept":"retired"}', '{"version":1,"taskId":"t","status":"succeeded"}']) {
    assert.equal(parseBeginnerBookmark(raw), 0);
  }
  assert.equal(parseBeginnerBookmark(serializeBeginnerBookmark(3)), 3);
  assert.equal(parseBeginnerBookmark(serializeBeginnerBookmark(-1)), 0);
});
