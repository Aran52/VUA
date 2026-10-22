import { describe, expect, it } from "vitest";
import { dismissToast, toast, toastDuration, toastSnapshot } from "./toast-store.ts";

describe("toast store", () => {
  it("keeps the three newest toasts and replaces a repeated message", () => {
    for (const item of toastSnapshot()) dismissToast(item.id);
    toast("one"); toast("two"); toast("three"); toast("four");
    expect(toastSnapshot().map(item => item.text)).toEqual(["two", "three", "four"]);
    toast("two", "success");
    expect(toastSnapshot().map(item => item.text)).toEqual(["three", "four", "two"]);
    expect(toastSnapshot().at(-1)?.tone).toBe("success");
  });

  it("stays about two seconds, longer for longer text, at most six", () => {
    expect(toastDuration("已复制")).toBe(2400);
    expect(toastDuration("x".repeat(36))).toBe(3600);
    expect(toastDuration("x".repeat(400))).toBe(6000);
  });
});
