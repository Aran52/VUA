import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { DeploymentPurpose } from "@vua/contracts";
import { ScenarioPreparation } from "./ScenarioPreparation.tsx";
const { gateway } = vi.hoisted(() => ({ gateway: { environment: { scenarios: { check: vi.fn(), sop: vi.fn() }, unityLicense: { snapshot: vi.fn(), check: vi.fn() } } } }));
vi.mock("../../gateway/index.ts", () => ({ useGateway: () => gateway }));
function render(purpose: DeploymentPurpose) {
  return renderToStaticMarkup(createElement(ScenarioPreparation, { intent: { purposes: [purpose], editorRoot: "C:\\Editors" }, checked: null, onChecked: vi.fn() }));
}
describe("the visible Avatar scenario license step", () => {
  it.each(["pc_avatar", "quest_avatar"] as const)("shows the step before opening the SOP for %s", purpose => {
    const html = render(purpose);
    expect(html).toContain('class="vua-scenario-license-step"');
    expect(html).toContain('tabindex="-1"');
    expect(html.indexOf('vua-scenario-license-step')).toBeLessThan(html.indexOf('<details'));
    expect(gateway.environment.unityLicense.check).not.toHaveBeenCalled();
    expect(gateway.environment.scenarios.check).not.toHaveBeenCalled();
  });
  it("keeps the desktop play scenario independent of Unity licensing", () => {
    expect(render("desktop_play")).not.toContain('vua-scenario-license-step');
  });
});
