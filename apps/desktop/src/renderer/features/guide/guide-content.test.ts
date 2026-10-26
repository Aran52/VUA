/**
 * Content checks: the encyclopedia keeps its topic catalog and source-backed
 * media, while VUA instructions use a separate presentation. Read-only topics
 * never invent entries in the inactive tutorial content pack.
 */
import assert from "node:assert/strict";
import { test } from "vitest";
import { contentPack } from "../../app/tutorial-content-pack.ts";
import { strings } from "../../i18n/index.ts";
import {
  ENCYCLOPEDIA_TOPIC_IDS,
  GUIDE_MEDIA,
  GUIDE_PAGE_TUTORIAL,
  GUIDE_TOPIC_COPY_KEY,
  GUIDE_TOPIC_IDS,
  GUIDE_TUTORIAL_TOPIC_IDS,
  resolveGuideMedia,
  validateGuideContent,
} from "./guide-content.ts";

test("百科与独立 VUA 指南的内容结构校验通过(真实内容包)", () => {
  assert.doesNotThrow(() => validateGuideContent(contentPack));
});

test("教程支撑主题 id 与内容包教程同名且在内容包内;阅读型主题无教程支撑", () => {
  const pack = contentPack;
  const ids = new Set(pack.tutorials.map((t) => t.id));
  for (const topic of GUIDE_TUTORIAL_TOPIC_IDS) {
    assert.equal(GUIDE_PAGE_TUTORIAL[topic], topic);
    assert.ok(ids.has(topic), `内容包缺教程 ${topic}`);
  }
  for (const topic of ["guide-vua", "guide-room", "guide-hardware", "guide-creator"]) {
    assert.ok(!ids.has(topic));
  }
  assert.equal(GUIDE_TOPIC_IDS.length, 9);
  assert.equal(GUIDE_TOPIC_IDS[0], "guide-hardware");
  assert.equal(GUIDE_TOPIC_IDS.at(-1), "guide-vua");
  assert.equal(ENCYCLOPEDIA_TOPIC_IDS.length, 7);
  assert.ok(!ENCYCLOPEDIA_TOPIC_IDS.includes("guide-vua"));
  assert.ok(!ENCYCLOPEDIA_TOPIC_IDS.includes("guide-tutorials"));
});

test("每个主题 id 都映射到 strings.guide.pages 的合法键", () => {
  for (const topic of GUIDE_TOPIC_IDS) {
    const key = GUIDE_TOPIC_COPY_KEY[topic];
    assert.ok(key in strings.guide.pages, `${topic} copy key ${key}`);
  }
  assert.equal(GUIDE_TOPIC_COPY_KEY["guide-vua"], "vua");
  assert.equal(GUIDE_TOPIC_COPY_KEY["guide-creator"], "creator");
});

test("媒体解析:登记 id 有资产,未登记 id 为 null", () => {
  for (const id of Object.keys(GUIDE_MEDIA)) {
    const media = resolveGuideMedia(id);
    assert.ok(media && media.src.length > 0 && media.alt.length > 0, `${id} 资产缺省`);
  }
  assert.equal(resolveGuideMedia("no-such-media"), null);
});

test("媒体 src 一律相对路径(file:// 打包形态下绝对路径必破图,B 切片真机实证)", () => {
  for (const [id, media] of Object.entries(GUIDE_MEDIA)) {
    assert.ok(media.src.startsWith("./guide/"), `${id} src 必须是 ./guide/ 相对路径`);
  }
});
