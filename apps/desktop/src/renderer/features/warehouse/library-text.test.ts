import { describe, expect, it } from "vitest";
import { priceLabel, readableDescription, videoLink } from "./library-text.ts";

describe("priceLabel", () => {
  it("groups the recorded amount and uses the currency's symbol", () => {
    expect(priceLabel({ price: { amount: "9000", currency: "JPY" } })).toMatch(/[¥￥]9,000$/u);
    expect(priceLabel({ price: { amount: "500", currency: "JPY" } })).toMatch(/[¥￥]500$/u);
  });
  it("shows a range only when the high differs", () => {
    expect(priceLabel({ price: { amount: "1000", currency: "JPY" }, priceHigh: "1500" })).toMatch(/1,000.*~.*1,500$/u);
    expect(priceLabel({ price: { amount: "1000", currency: "JPY" }, priceHigh: "1000" })).not.toContain("~");
  });
  it("keeps an amount the formatter cannot read as it was recorded", () => {
    expect(priceLabel({ price: { amount: "1500", currency: "not a currency" } })).toBe("not a currency 1,500");
    expect(priceLabel({ price: { amount: "12.50", currency: "USD" } })).toContain("12.50");
  });
});

describe("videoLink", () => {
  it("opens a YouTube embed as its own page", () => {
    expect(videoLink("https://www.youtube.com/embed/abc-DEF_1")).toEqual({ href: "https://www.youtube.com/watch?v=abc-DEF_1", site: "YouTube" });
    expect(videoLink("https://youtu.be/abc")).toEqual({ href: "https://youtu.be/abc", site: "YouTube" });
  });
  it("names other sites by their host", () => {
    expect(videoLink("https://www.nicovideo.jp/watch/sm1")).toEqual({ href: "https://www.nicovideo.jp/watch/sm1", site: "nicovideo.jp" });
    expect(videoLink("not a url")).toEqual({ href: "not a url", site: "not a url" });
  });
});

describe("readableDescription", () => {
  it("drops bold marks and turns rule lines into paragraph breaks", () => {
    expect(readableDescription("Intro **bold** text ========= Next part")).toBe("Intro bold text\n\nNext part");
    expect(readableDescription("a - b -- c")).toBe("a - b -- c");
  });
});
