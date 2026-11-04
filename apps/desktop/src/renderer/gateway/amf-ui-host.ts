import * as React from "react";
import * as ReactDOM from "react-dom";
import * as JSX from "react/jsx-runtime";
import * as gateway from "./GatewayProvider.tsx";
import * as tasks from "./task-identity.ts";
import * as modals from "../components/primitives/modal-layer.tsx";
import * as browser from "../app/login-browser-store.ts";
import { registerAmfGenerationCleanup } from "../app/amf-generation-cleanup.ts";

/** The first-party AMF entry shares the host's React and stateful shell bindings.
 * No remote URL, arbitrary module name or user-supplied code can select an entry. */
export function installAmfUiHost(): void {
  const current = (globalThis as typeof globalThis & { __VUA_AMF_UI_HOST_V01__?: { abi: string; React: typeof React } }).__VUA_AMF_UI_HOST_V01__;
  if (current) { if (current.abi !== "amf-ui/0.1" || current.React !== React) throw Error("incompatible_amf_ui_host"); return; }
  Object.defineProperty(globalThis, "__VUA_AMF_UI_HOST_V01__", { configurable: false, writable: false,
    value: Object.freeze({ abi: "amf-ui/0.1", React, ReactDOM, JSX, gateway, tasks, modals, browser,
      lifecycle: Object.freeze({ abi: "amf-hot/0.1", registerCleanup: registerAmfGenerationCleanup }) }) });
}
