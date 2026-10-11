import { beforeEach, describe, expect, it } from "vitest";
import type { ApplicationEventV01, DesktopGatewaySuccessValueV1 } from "@vua/contracts";
import { strings } from "../i18n/index.js";
import type { GatewayClient } from "./gateway-client.js";
import { resetTaskIdentities, taskIdentityOf } from "./task-identity.js";
import { createWarehouseCommands } from "./warehouse-commands-live.js";

/** 受理即登记(W25 走查 D1 回归钉):导入/采纳命令的受理回执抵达端口时,
 * 渲染层已知操作类型与来源页必须登记任务身份——通知中心据此呈现人类可读
 * 标题,不再裸 taskId。 */

function clientReplying(value: DesktopGatewaySuccessValueV1): GatewayClient {
  return {
    invoke: () => Promise.resolve({ ok: true, value }),
    subscribe: (_listener: (event: ApplicationEventV01) => void) => () => {},
  };
}

beforeEach(() => {
  resetTaskIdentities();
});

describe("live warehouse command port task identity registration", () => {
  it("routes native file/folder intake through v0.2 and registers the normal import task", async () => {
    let request: unknown;
    const client = clientReplying({ schemaVersion: "0.2", operation: "library.importPaths", taskId: "task-drop-1", correlationId: "corr-drop" });
    const invoke = client.invoke;
    client.invoke = r => { request = r; return invoke(r); };
    const port = createWarehouseCommands(client);
    expect(await port.importPaths(["C:/Avatar.zip", "C:/Clothes"], { commandId: "drop-retained" })).toMatchObject({ ok: true, accepted: { taskId: "task-drop-1" } });
    expect(request).toMatchObject({ method: "library.importPaths", params: { schemaVersion: "0.2", sourcePaths: ["C:/Avatar.zip", "C:/Clothes"], commandId: "drop-retained" } });
    expect(taskIdentityOf("task-drop-1")).toEqual({ title: strings.taskTitles.importBatch, originPage: "warehouse" });
    const oldReceipt = createWarehouseCommands(clientReplying({ schemaVersion: "0.1", operation: "library.importFolders", taskId: "old", correlationId: "old" }));
    expect(await oldReceipt.importPaths(["C:/Avatar.zip"])).toMatchObject({ ok: false });
    expect(taskIdentityOf("old")).toBeUndefined();
  });
  it("registers the import identity when the library intake acceptance arrives", async () => {
    const port = createWarehouseCommands(
      clientReplying({ schemaVersion: "0.1", operation: "library.importFolders", taskId: "task-178984402495255500-0001", correlationId: "corr-task-1" }),
    );
    const outcome = await port.importFolders(["C:/avatars/Meiyun"]);
    expect(outcome.ok).toBe(true);
    expect(taskIdentityOf("task-178984402495255500-0001")).toEqual({
      title: strings.taskTitles.importBatch,
      originPage: "warehouse",
    });
  });

  it("registers the download adoption identity when warehouse.importDownloads acceptance arrives", async () => {
    const port = createWarehouseCommands(
      clientReplying({ taskId: "task-2", correlationId: "corr-task-2" }),
    );
    const outcome = await port.importDownloads(["dl-1"]);
    expect(outcome.ok).toBe(true);
    expect(taskIdentityOf("task-2")).toEqual({
      title: strings.taskTitles.adoptDownload,
      originPage: "warehouse",
    });
  });

  it("leaves no identity behind when the command is not accepted", async () => {
    const port = createWarehouseCommands({
      invoke: () =>
        Promise.resolve({
          ok: false,
          error: { kind: "application", error: {
            code: "vua.warehouse.entry_not_found",
            messageKey: "errors.warehouse.entryNotFound",
            recoverable: true,
            retryable: false,
          } },
        }),
      subscribe: (_listener: (event: ApplicationEventV01) => void) => () => {},
    } as unknown as GatewayClient);
    const outcome = await port.importFolders(["C:/avatars/Meiyun"]);
    expect(outcome.ok).toBe(false);
    expect(taskIdentityOf("task-1")).toBeUndefined();
  });
});
