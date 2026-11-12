import {
  isPlayScenarioSopV02,
  isPlaySetupCatalogV02,
  isScenarioEnvironmentCheck,
  isScenarioSop,
  type DeploymentIntent,
  type PlayScenarioSopV02,
  type PlaySetupCatalogV02,
  type PlaySetupSelectionV02,
  type ScenarioEnvironmentCheck,
  type ScenarioSelection,
  type ScenarioSop,
} from "@vua/contracts";
import type { GatewayClient } from "./gateway-client.ts";
export interface ScenarioEnvironmentPort {
  playSetups(): Promise<PlaySetupCatalogV02>;
  playSop(selection: PlaySetupSelectionV02): Promise<PlayScenarioSopV02>;
  sop(selection: ScenarioSelection): Promise<ScenarioSop>;
  check(intent: DeploymentIntent): Promise<ScenarioEnvironmentCheck>;
}
/** The SOP query never calls a check; the check query never calls a plan. */
export function createScenarioEnvironmentPort(client: GatewayClient): ScenarioEnvironmentPort {
  return {
    async playSetups() {
      const response = await client.invoke({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "environment.listPlaySetups", params: { schemaVersion: "0.2" } });
      if (!response.ok || !isPlaySetupCatalogV02(response.value)) throw new Error("play_setups_unavailable");
      return response.value;
    },
    async playSop(selection) {
      const response = await client.invoke({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "environment.getScenarioSop", params: { schemaVersion: "0.2", selection } });
      if (!response.ok || !isPlayScenarioSopV02(response.value)) throw new Error("scenario_sop_unavailable");
      return response.value;
    },
    async sop(scenario) {
      const response = await client.invoke({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "environment.getScenarioSop", params: { schemaVersion: "0.1", scenario } });
      if (!response.ok || !isScenarioSop(response.value)) throw new Error("scenario_sop_unavailable");
      return response.value;
    },
    async check(intent) {
      const response = await client.invoke({ schemaVersion: 1, requestId: crypto.randomUUID(), method: "environment.checkScenarioEnvironment", params: { schemaVersion: "0.1", intent } });
      if (!response.ok || !isScenarioEnvironmentCheck(response.value)) throw new Error("scenario_check_unavailable");
      return response.value;
    },
  };
}
