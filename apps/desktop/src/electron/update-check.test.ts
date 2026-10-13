import assert from "node:assert/strict";
import { describe, it } from "vitest";
import { checkLatestRelease, compareVersions, isDistributionReleaseUrl, LATEST_RELEASE_URL, OFFICIAL_RELEASE_URL } from "./update-check.js";

/**
 * 版本检测纯逻辑测试:三段数字比较(去 v 前缀、不等长补零、不可解析 = null
 * 不猜测);检测全路径——远端更高 = newer-available,相等/更低 = up-to-date,
 * 网络异常/HTTP 失败语义(注入 fetcher 抛错)/负载不可解析/缺 tag_name/
 * 版本串不可比较 一律 check-failed(诚实失败,绝不猜态)。
 */

describe("compareVersions (strict numeric dotted compare)", () => {
  it("compares dotted numeric versions with v-prefix tolerance", () => {
    assert.ok(compareVersions("0.7.0", "0.6.0")! > 0);
    assert.ok(compareVersions("v0.7.0", "0.6.0")! > 0);
    assert.ok(compareVersions("0.6.0", "0.7.0")! < 0);
    assert.equal(compareVersions("0.6.0", "0.6.0"), 0);
    assert.equal(compareVersions("v0.6.0", "0.6.0"), 0);
    assert.equal(compareVersions("0.6", "0.6.0"), 0);
    assert.ok(compareVersions("0.6.1", "0.6")! > 0);
    assert.ok(compareVersions("1.0.0", "0.99.99")! > 0);
    assert.ok(compareVersions("0.10.0", "0.9.9")! > 0);
  });

  it("refuses unparseable versions instead of guessing", () => {
    assert.equal(compareVersions("dev", "0.6.0"), null);
    assert.equal(compareVersions("0.6.0", "nightly-2026"), null);
    assert.equal(compareVersions("0.6.0-beta", "0.6.0"), null);
    assert.equal(compareVersions("", "0.6.0"), null);
    assert.equal(compareVersions("0..1", "0.6.0"), null);
  });
});

describe("checkLatestRelease (honest three-state detection)", () => {
  const fetcher = (payload: unknown) => async () => payload;

  it("reports newer-available when the remote release is higher", async () => {
    const result = await checkLatestRelease(
      "0.6.0",
      fetcher({ tag_name: "v0.7.0", html_url: "https://example.test/release" }),
      "2026-09-19T00:00:00.000Z",
    );
    assert.equal(result.state, "newer-available");
    assert.equal(result.currentVersion, "0.6.0");
    assert.equal(result.latestVersion, "v0.7.0");
    assert.equal(result.releaseUrl, "https://example.test/release");
    assert.equal(result.checkedAt, "2026-09-19T00:00:00.000Z");
  });

  it("reports up-to-date for equal or lower remote versions", async () => {
    for (const tag of ["v0.6.0", "0.6.0", "v0.5.9"]) {
      const result = await checkLatestRelease("0.6.0", fetcher({ tag_name: tag, html_url: "https://example.test/r" }));
      assert.equal(result.state, "up-to-date", `tag ${tag}`);
      assert.equal(result.latestVersion, tag);
    }
  });

  it("tolerates a missing html_url as honest absence", async () => {
    const result = await checkLatestRelease("0.6.0", fetcher({ tag_name: "v0.7.0" }));
    assert.equal(result.state, "newer-available");
    assert.equal(result.releaseUrl, null);
  });

  it("falls to check-failed on fetch failure, never throwing", async () => {
    const result = await checkLatestRelease("0.6.0", async () => {
      throw new Error("offline");
    });
    assert.equal(result.state, "check-failed");
    assert.equal(result.latestVersion, null);
    assert.equal(result.releaseUrl, null);
    const synchronous = await checkLatestRelease("0.6.0", () => { throw Error("offline before returning a Promise"); });
    assert.equal(synchronous.state, "check-failed");
  });

  it("falls to check-failed on malformed payloads without guessing", async () => {
    for (const payload of [null, "not-an-object", [], {}, { tag_name: "" }, { tag_name: 42 }, { tag_name: "not-a-version" }]) {
      const result = await checkLatestRelease("0.6.0", fetcher(payload));
      assert.equal(result.state, "check-failed", JSON.stringify(payload));
    }
  });

  it("queries the repository releases endpoint kept in sync with app-meta", () => {
    assert.match(LATEST_RELEASE_URL, /^https:\/\/api\.github\.com\/repos\/VUA-Project\/VUA\/releases\/latest$/);
  });

  it("uses the official source when GitHub is unreachable and rejects malformed mirrors", async () => {
    const result = await checkLatestRelease("0.6.7", async url => {
      if (url === LATEST_RELEASE_URL) throw Error("unreachable");
      return { tag_name: "v0.6.8", html_url: "https://ph-r.com/vua/releases/v0.6.8/" };
    });
    assert.equal(result.state, "newer-available");
    assert.equal(result.releaseUrl, "https://ph-r.com/vua/releases/v0.6.8/");
    const fallback = await checkLatestRelease("0.6.7", async url => url === OFFICIAL_RELEASE_URL ? "html instead of JSON" : { tag_name: "v0.6.8" });
    assert.equal(fallback.latestVersion, "v0.6.8");
  });

  it("does not let a reachable stale source hide a newer version", async () => {
    for (const newerSource of [LATEST_RELEASE_URL, OFFICIAL_RELEASE_URL]) {
      const result = await checkLatestRelease("0.6.7", async url => ({ tag_name: url === newerSource ? "v0.6.9" : "v0.6.7", html_url: url }));
      assert.equal(result.latestVersion, "v0.6.9");
      assert.equal(result.state, "newer-available");
    }
  });

  it("limits authenticated release navigation to the two release-page roots", () => {
    assert.equal(isDistributionReleaseUrl("https://ph-r.com/vua/releases/v0.6.8/"), true);
    assert.equal(isDistributionReleaseUrl("https://github.com/VUA-Project/VUA/releases/tag/v0.6.8"), true);
    for (const url of ["http://ph-r.com/vua/releases/v0.6.8/", "https://ph-r.com.evil.test/vua/releases/v0.6.8/", "https://ph-r.com/vua/releases/v0.6.8/?next=evil", "https://github.com/other/repo/releases/tag/v0.6.8"]) assert.equal(isDistributionReleaseUrl(url), false);
  });
});
