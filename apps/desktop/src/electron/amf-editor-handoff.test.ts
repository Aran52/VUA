import { describe,expect,it,vi } from "vitest";
import { editorEnvironment, failSdkTasks, launchAmfEditor, waitForLocalTestRequest } from "./amf-editor-handoff.js";
import { spawn, type ChildProcess } from "node:child_process";
import { EventEmitter } from "node:events";
import type { OrchestratorProviderV01 } from "@vua/orchestrator-provider";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
vi.mock("node:child_process", () => ({ spawn: vi.fn() }));
describe("SDK handoff environment",()=>{
  it("opens a queued local test after the earlier assembly succeeds and the worker writes its request", async () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "vua-test-queued-launch-"));
    try {
      const folder = path.join(root, ".vua", "bridge"); fs.mkdirSync(folder, { recursive: true });
      let reads = 0;
      const provider = { async invoke(request: { method: string }) {
        if (request.method === "amf.production.get") return { ok: true, value: { runId: "n3-test", phase: reads++ === 0 ? "assembling" : "succeeded",
          projectRoot: root, editor: { path: path.join(root, "Unity.exe"), version: "2022.3.22f1" } } };
        fs.writeFileSync(path.join(folder, "amf-test.json"), JSON.stringify({ schemaVersion: "0.1", runId: "n3-test", testId: "test-task-queued" }));
        return { ok: true, value: { taskId: "task-queued", state: "running", cancellationRequested: false, recoveryDisposition: "none" } };
      } } as unknown as OrchestratorProviderV01;
      vi.mocked(spawn).mockImplementationOnce(() => {
        const child = Object.assign(new EventEmitter(), { unref: vi.fn() });
        queueMicrotask(() => child.emit("spawn"));
        return child as ChildProcess;
      });
      await launchAmfEditor(provider, "n3-test", "task-queued");
      await vi.waitFor(() => expect(spawn).toHaveBeenCalledOnce());
      expect(spawn).toHaveBeenCalledWith(path.join(root, "Unity.exe"), ["-projectPath", root], expect.objectContaining({ windowsHide: true }));
      expect(reads).toBe(2);
    } finally { fs.rmSync(root, { recursive: true, force: true }); vi.mocked(spawn).mockReset(); }
  });
  it("waits for the queued task's own Bridge request and stops when the task ends", async () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "vua-test-ready-"));
    try {
      const folder = path.join(root, ".vua", "bridge"); fs.mkdirSync(folder, { recursive: true });
      const request = path.join(folder, "amf-test.json");
      fs.writeFileSync(request, JSON.stringify({ schemaVersion: "0.1", runId: "n3-test", testId: "test-old" }));
      let pauses = 0;
      const task = { taskId: "task-1", cancellationRequested: false, recoveryDisposition: "none" };
      const states = ["queued", "preparing", "paused", "waiting_for_input"];
      const ready = await waitForLocalTestRequest(root, "n3-test", "task-1", async () => ({ ...task, state: states[pauses] }), async () => {
        pauses++;
        if (pauses === 2) fs.writeFileSync(request, JSON.stringify({ schemaVersion: "0.1", runId: "n3-test", testId: "test-task-1" }));
      });
      expect(ready).toBe(true); expect(pauses).toBe(3);
      for (const state of ["succeeded", "failed", "cancelled", "waiting_user"]) {
        expect(await waitForLocalTestRequest(root, "n3-test", "task-1", async () => ({ ...task, state }))).toBe(false);
      }
      for (const changes of [{ cancellationRequested: true }, { recoveryDisposition: "inspect_required" }, { taskId: "other-task" }]) {
        expect(await waitForLocalTestRequest(root, "n3-test", "task-1", async () => ({ ...task, state: "running", ...changes }))).toBe(false);
      }
    } finally { fs.rmSync(root, { recursive: true, force: true }); }
  });
  it("reports a local test launch failure only for the matching run and request", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "vua-test-launch-"));
    try {
      const folder = path.join(root, ".vua", "bridge"); fs.mkdirSync(folder, { recursive: true });
      const request = path.join(folder, "amf-test.json"), receipt = path.join(folder, "amf-test.result.json");
      fs.writeFileSync(request, JSON.stringify({ schemaVersion: "0.1", runId: "n3-other", testId: "test-1" }));
      failSdkTasks(root, "n3-test"); expect(fs.existsSync(receipt)).toBe(false);
      fs.writeFileSync(request, JSON.stringify({ schemaVersion: "0.1", runId: "n3-test", testId: "test-1" }));
      failSdkTasks(root, "n3-test");
      expect(JSON.parse(fs.readFileSync(receipt, "utf8"))).toMatchObject({ runId: "n3-test", testId: "test-1", outcome: "failed", reason: "editor_unavailable" });
    } finally { fs.rmSync(root, { recursive: true, force: true }); }
  });
  it("retains OS paths and excludes inherited application credentials",()=>{
    const env=editorEnvironment({PATH:"system",LOCALAPPDATA:"profile",BOOTH_TOKEN:"private",GITHUB_TOKEN:"private",GH_TOKEN:"private",OPENAI_API_KEY:"private",CUSTOM_SECRET:"private",SESSION_COOKIE:"private"});
    expect(env).toEqual({PATH:"system",LOCALAPPDATA:"profile"});
  });
});
