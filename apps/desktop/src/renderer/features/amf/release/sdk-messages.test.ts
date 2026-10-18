import { describe, expect, it } from "vitest";
import { sdkReviewCopy } from "../../../i18n/sdk-review-copy.ts";
import { SDK_MESSAGE_KEYS, sdkMessageKey } from "./sdk-messages.ts";

describe("plain words for SDK messages", () => {
  it("reads the SDK's messages from a real Meiyun check", () => {
    expect(sdkMessageKey("This avatar contains meshes that were imported with Read/Write disabled. This must be fixed in the mesh import settings before uploading.")).toBe("meshReadWrite");
    expect(sdkMessageKey("This avatar is shorter than average.")).toBe("shorter");
    expect(sdkMessageKey("This avatar uses Unity constraints. Consider using VRChat constraints instead. Click Auto Fix to attempt to replace all constraints and rebind avatar animations automatically.")).toBe("unityConstraints");
    expect(sdkMessageKey("This avatar contains one or more animator states with Write Defaults disabled where the animation clip is either missing or empty. To avoid animation issues, assign animation clips containing at least one property.")).toBe("writeDefaultsEmpty");
    expect(sdkMessageKey("This avatar uses textures with 'Box' mipmap filtering, which blurs distant textures. Switch to 'Kaiser' for improved sharpness.")).toBe("boxMipmaps");
  });
  it("tells similar messages apart and leaves unknown ones in the SDK's words", () => {
    expect(sdkMessageKey("This avatar is too short. The minimum is 20cm shoulder height.")).toBe("tooShort");
    expect(sdkMessageKey("This avatar is too tall. The maximum is 5m shoulder height.")).toBe("tooTall");
    expect(sdkMessageKey("This avatar has no pipeline manager component. Click Auto Fix to add one.")).toBe("noPipeline");
    expect(sdkMessageKey("Something the SDK says in a later release.")).toBeNull();
  });
  it("has plain words for every message in all four languages", () => {
    for (const locale of ["en", "zh-CN", "ja", "ko"] as const) {
      for (const key of SDK_MESSAGE_KEYS) expect(sdkReviewCopy[locale].messages[key], `${locale} ${key}`).toBeTruthy();
    }
  });
});
