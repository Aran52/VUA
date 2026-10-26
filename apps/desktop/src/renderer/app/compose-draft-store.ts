import { useEffect, useState } from "react";
import { createSignal } from "../gateway/index.ts";

/**
 * 搭配草稿共享状态(019 批 B,UI-03;桌面域共享容器层):
 * - 项目无关:条目直接来自素材库(warehouseItemId 身份,UI-02 统一对象
 *   身份),无须先创建 Unity 项目;
 * - 会话概念:未提交草稿是客户端会话状态(UI-02 归共享草稿状态=桌面域),
 *   **不进 localStorage**(核心 UI-03 评估:持久化由版本化应用接口
 *   recipe.save 管理,禁止散落 localStorage 充当生产文档库);保存后的
 *   权威事实在 recipe 文档库(recipe.list 读面);
 * - 撤销栈:撤销只回退本地未提交编辑,不反向执行已提交命令(UI-03);
 * - 保存入口(批 B 保存链,core 路由裁定):挂载选择器按 recipe v0.3
 *   entrypointSelector anyOf 由用户输入(catalogEntryId 引用目录条目或
 *   nameHint 用户命名提示)——零词表扩展,recipe.save 原样承载;共享容器
 *   层 signal:跨 UI 根切换保留(UI 根切换不触碰本 store)。
 */

export interface ComposeDraftItem {
  /** 素材身份(UI-02):warehouse 条目 id */
  readonly warehouseItemId: string;
  /** 呈现名(素材事实) */
  readonly title: string;
  /** 素材角色(素材事实;无则 null) */
  readonly role: string | null;
  /** 挂载选择器名称提示(recipe v0.3 anyOf 用户输入;null = 未指定)。
   *  D3 自动派生(用户裁定 2026-09-20):加入时未指定即派生为条目
   *  displayName——用户无须填写即可保存;清空 = 显式未命名(守卫如实阻止) */
  readonly nameHint: string | null;
  readonly addedAt: string;
}

export interface ComposeDraftState {
  /** 当前草稿条目(加入顺序) */
  readonly items: readonly ComposeDraftItem[];
  /** 撤销栈(本地未提交编辑的历史;不含已提交状态) */
  readonly undoStack: readonly (readonly ComposeDraftItem[])[];
  /** 与最近一次保存的差异标记(会话内;保存接口接入前恒 true——有内容即未保存) */
  readonly dirty: boolean;
  /** 最近一次成功保存的 recipe 身份与修订(UI-02 对象身份;跨 UI 根保留) */
  readonly saved: { readonly recipeId: string; readonly revision: number } | null;
}

export const emptyComposeDraft: ComposeDraftState = {
  items: [],
  undoStack: [],
  dirty: false,
  saved: null,
};

/** 共享容器层草稿 signal(UI 根切换不触碰) */
const draftSignal = createSignal<ComposeDraftState>(emptyComposeDraft);
export function readComposeDraft(): ComposeDraftState { return draftSignal.get(); }

/** 纯函数语义的状态转换(共享 signal 落地) */
function apply(next: ComposeDraftState): void {
  draftSignal.set(next);
}

export function useComposeDraft(): ComposeDraftState {
  const [state, setState] = useState(draftSignal.get());
  useEffect(() => {
    const unsubscribe = draftSignal.subscribe((value) => setState(value));
    return () => {
      unsubscribe();
    };
  }, []);
  return state;
}

export function composeAddItemAction(
  item: Omit<ComposeDraftItem, "addedAt">,
): void {
  // 时钟注入(BG-18):真实时钟只在命令边界取用,纯函数保持确定
  apply(composeAddItem(draftSignal.get(), item, new Date().toISOString()));
}

export function composeRemoveItemAction(warehouseItemId: string): void {
  apply(composeRemoveItem(draftSignal.get(), warehouseItemId));
}

/** 挂载选择器名称提示编辑(纯函数;不入撤销栈——文本输入粒度,结构变更
 *  才压栈;dirty 置真) */
export function composeSetNameHint(
  state: ComposeDraftState,
  warehouseItemId: string,
  nameHint: string,
): ComposeDraftState {
  if (!state.items.some((item) => item.warehouseItemId === warehouseItemId)) return state;
  const hint = nameHint.length > 0 ? nameHint : null;
  const items = state.items.map((item) =>
    item.warehouseItemId === warehouseItemId ? { ...item, nameHint: hint } : item,
  );
  return { items, undoStack: state.undoStack, dirty: true, saved: state.saved };
}

/** 挂载选择器名称提示编辑(用户输入;action 层经纯函数落共享 signal) */
export function composeSetNameHintAction(warehouseItemId: string, nameHint: string): void {
  apply(composeSetNameHint(draftSignal.get(), warehouseItemId, nameHint));
}

export function composeUndoAction(): void {
  apply(composeUndo(draftSignal.get()));
}

/** 保存成功对齐(回执驱动;saved 身份/修订入容器层状态——跨 UI 根保留,
 *  请求解析入口据此启用) */
export function composeSavedAction(
  recipeId: string, revision: number, submittedItems?: ComposeDraftState["items"],
): boolean {
  const current = draftSignal.get();
  if (current.saved?.recipeId === recipeId && current.saved.revision > revision) return false;
  const next = composeSaved(current, recipeId, revision);
  // Edits made during an in-flight save are still unsaved.
  apply(submittedItems && current.items !== submittedItems ? { ...next, dirty: true } : next);
  return true;
}

/* ---- 批 B 保存链:草稿 → recipe v0.3 文档映射(core 路由裁定零词表
 * 扩展;recipe.save 原样承载)。entrypointSelector anyOf:nameHint 用户
 * 命名提示(无目录条目事实时不虚构 catalogEntryId) ---- */

export interface ComposeSaveDocument {
  readonly formatVersion: "0.3";
  readonly recipeId: string;
  readonly baseRevision: number;
  readonly title: string;
  readonly createdAt: string;
  readonly updatedAt: string;
  readonly assets: readonly {
    readonly id: string;
    readonly role: string;
    readonly label: string;
    readonly sourceRef: { readonly warehouseItemId: string; readonly role: "original" };
  }[];
  readonly instances: readonly {
    readonly id: string;
    readonly assetId: string;
    readonly entrypoint: {
      readonly selectorId: string;
      readonly kind: "user_named_entrypoint";
      readonly nameHint: string;
    };
    readonly enabled: true;
  }[];
  readonly relations: readonly unknown[];
}

export interface ComposeSaveInput {
  /** 已保存文档的 recipeId(再次保存沿用;首存 = null 由调用方生成) */
  readonly savedRecipeId: string | null;
  /** 已保存文档的 baseRevision(首存 = 0) */
  readonly savedRevision: number;
  readonly items: readonly ComposeDraftItem[];
  readonly now: string;
}

/** 草稿 → recipe v0.3 保存文档(整文档提交;entrypoint=nameHint 用户
 *  命名提示——无 entrypoint 事实时用户命名,系统不虚构);items 空 = null */
export function composeDraftToSaveDocument(
  input: ComposeSaveInput,
): ComposeSaveDocument | null {
  if (input.items.length === 0) return null;
  const now = input.now;
  const recipeId =
    input.savedRecipeId ??
    `0190${(now.replaceAll(/[-:TZ.]/g, "") + "000000").slice(0, 20)}f7`;
  return {
    formatVersion: "0.3",
    recipeId,
    baseRevision: input.savedRevision,
    title: input.items.map((item) => item.title).join(" + "),
    createdAt: now,
    updatedAt: now,
    assets: input.items.map((item) => ({
      id: item.warehouseItemId,
      role: item.role ?? "other",
      label: item.title,
      sourceRef: { warehouseItemId: item.warehouseItemId, role: "original" },
    })),
    instances: input.items.map((item, index) => ({
      id: `${item.warehouseItemId}-instance-${index + 1}`,
      assetId: item.warehouseItemId,
      entrypoint: {
        selectorId: `${item.warehouseItemId}-entrypoint`,
        kind: "user_named_entrypoint",
        nameHint: item.nameHint ?? item.title,
      },
      enabled: true,
    })),
    relations: [],
  };
}

/** 加入素材(身份幂等:同素材重复加入为无操作)。时钟注入(BG-18):now
 *  必填——纯函数确定化,真实时钟由 action 层(命令边界)取用。
 *  挂载名称自动派生(D3,用户裁定 2026-09-20「不该让用户填写」):加入时
 *  未指定 nameHint(null/空白)即自动派生为条目 displayName(title,仓库
 *  素材的稳定呈现标识,满足 recipe 词面校验)——两套 UI 同一规则;用户仍可
 *  在输入框显式覆盖或清空(清空 = 显式未命名,保存守卫如实阻止) */
export function composeAddItem(
  state: ComposeDraftState,
  item: Omit<ComposeDraftItem, "addedAt">,
  now: string,
): ComposeDraftState {
  if (state.items.some((existing) => existing.warehouseItemId === item.warehouseItemId)) {
    return state;
  }
  const nameHint =
    item.nameHint !== null && item.nameHint.trim() !== "" ? item.nameHint : item.title;
  return {
    items: [...state.items, { ...item, nameHint, addedAt: now }],
    undoStack: [...state.undoStack.slice(-49), state.items],
    dirty: true,
    saved: state.saved,
  };
}

/** 移除素材(按身份) */
export function composeRemoveItem(state: ComposeDraftState, warehouseItemId: string): ComposeDraftState {
  if (!state.items.some((existing) => existing.warehouseItemId === warehouseItemId)) {
    return state;
  }
  return {
    items: state.items.filter((existing) => existing.warehouseItemId !== warehouseItemId),
    undoStack: [...state.undoStack.slice(-49), state.items],
    dirty: true,
    saved: state.saved,
  };
}

/** 撤销:回退到上一本地编辑前状态;空栈 = 无操作。dirty 语义(BG-18):
 *  回到空草稿且从未保存 = 无未保存差异(dirty:false);已保存后回空 =
 *  内容偏离已保存文档(dirty:true) */
export function composeUndo(state: ComposeDraftState): ComposeDraftState {
  const previous = state.undoStack[state.undoStack.length - 1];
  if (previous === undefined) return state;
  return {
    items: previous,
    undoStack: state.undoStack.slice(0, -1),
    dirty: previous.length > 0 || state.saved !== null,
    saved: state.saved,
  };
}

/** 保存成功后的状态对齐(修订号来自服务端回执;脏标记清除;saved 身份
 *  入状态供请求解析入口使用) */
export function composeSaved(
  state: ComposeDraftState,
  recipeId: string,
  revision: number,
): ComposeDraftState {
  return { ...state, saved: { recipeId, revision }, dirty: false };
}
