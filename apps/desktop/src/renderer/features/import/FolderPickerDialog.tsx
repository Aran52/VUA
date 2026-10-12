import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Icon } from "@vua/design-system";
import type { DesktopFsErrorV1, DesktopFsListV1, DesktopFsNavigationV1, DesktopFsEntryV1 } from "@vua/contracts";
import { Button } from "../../components/primitives/Button.tsx";
import { ModalOwnerContext, useModalLayer } from "../../components/primitives/modal-layer.tsx";
import { strings } from "../../i18n/index.ts";
import {
  createErrorCopyKey,
  isValidNewFolderName,
  listErrorCopyKey,
  loadLastFolder,
  parentDirOf,
  saveLastFolder,
} from "./folder-picker-model.ts";
import "./folder-picker.css";

/**
 * 应用内文件夹选择器(2026-09-25 用户裁决,本地导入段;ALCOM 的
 * 「选择工作区目录」形态):目录浏览(仅子目录)+ 多选 + 新建 + 记忆。
 * - 数据面:DesktopFsApiV1 窄面(listDirectory/createDirectory),失败
 *   收信如实呈现(列表错误/内联新建错误/路径编辑内联错误三处);
 * - 记忆:打开起于上次浏览目录(loadLastFolder),失效回落用户主目录;
 *   确认成功记当前浏览目录(非选中子项);Windows 原生拾取路径记
 *   首个选取的父目录;
 * - 打开语义:有选中 = 确认所选;空选 = 确认当前浏览目录;
 * - 模态栈:useModalLayer 与 ConfirmDialog 同构(Esc/背板 = 取消);
 *   路径输入的 Esc = 取消编辑经 [data-vua-esc-local] 局部接管
 *   (modal-layer opt-out),焦点随后移回编辑钮,后续 Esc 照常关弹窗;
 * - 无壳降级:window.vua.fs 缺席(浏览器 dev)时列表如实失败态,不悬挂。
 */

type ListingState =
  | { readonly kind: "loading" }
  | { readonly kind: "error"; readonly error: DesktopFsErrorV1; readonly target: string | null }
  | { readonly kind: "loaded"; readonly list: DesktopFsListV1 }
  | { readonly kind: "computer"; readonly drives: readonly DesktopFsEntryV1[] };

export interface FolderPickerDialogProps {
  readonly open: boolean;
  /** 取消(Esc/背板/取消钮):关闭且不选取 */
  readonly onClose: () => void;
  /** 确认所选文件夹路径(有序、去重由调用方合流) */
  readonly onConfirm: (folders: readonly string[]) => void;
  /** Windows 原生文件夹拾取(取消返回 null/空);非 null 时经同一
   *  onConfirm 收口,并记首个选取的父目录为上次浏览目录 */
  readonly onUseWindowsPicker: () => Promise<readonly string[] | null>;
}

export function FolderPickerDialog({
  open,
  onClose,
  onConfirm,
  onUseWindowsPicker,
}: FolderPickerDialogProps) {
  const copy = strings.folderPicker;
  const modal = useModalLayer(open, onClose);
  const [listing, setListing] = useState<ListingState>({ kind: "loading" });
  const [navigation, setNavigation] = useState<DesktopFsNavigationV1 | null>(null);
  const [selection, setSelection] = useState<ReadonlySet<string>>(new Set());
  const [showHidden, setShowHidden] = useState(false);
  const [editingPath, setEditingPath] = useState(false);
  const [pathDraft, setPathDraft] = useState("");
  const [pathError, setPathError] = useState<string | null>(null);
  const [newFolderOpen, setNewFolderOpen] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
  const [newFolderError, setNewFolderError] = useState<string | null>(null);
  const [newFolderBusy, setNewFolderBusy] = useState(false);
  const [windowsBusy, setWindowsBusy] = useState(false);
  // 竞态护栏:快速连点导航时仅最新响应落定(过期响应丢弃,不猜序)
  const requestSeqRef = useRef(0);
  // showHidden 经 ref 供 navigateTo 闭包读取(toggle  handler 先写 ref
  // 再触发重列,避免以旧值发请求)
  const showHiddenRef = useRef(false);
  const editButtonRef = useRef<HTMLButtonElement>(null);

  const navigateTo = useCallback(
    async (target: string | null): Promise<{ readonly ok: true } | { readonly ok: false; readonly error: DesktopFsErrorV1 }> => {
      const api = window.vua?.fs;
      const seq = (requestSeqRef.current += 1);
      setListing({ kind: "loading" });
      if (api === undefined) {
        setListing({ kind: "error", error: "failed", target });
        return { ok: false, error: "failed" };
      }
      const result = await api.listDirectory(target, { showHidden: showHiddenRef.current });
      if (seq !== requestSeqRef.current) return { ok: false, error: "failed" };
      if (result.ok) {
        setListing({ kind: "loaded", list: result.value });
        setPathDraft(result.value.path);
        setEditingPath(false);
        setSelection((current) => (current.size === 0 ? current : new Set()));
        return { ok: true };
      }
      setListing({ kind: "error", error: result.error, target });
      return { ok: false, error: result.error };
    },
    [],
  );

  // 打开即初始化:起于记忆目录,失效(记忆目录被移走/不可读)诚实回落
  // 用户主目录;选择/编辑/新建态复位(重开 = 诚实起点)
  useEffect(() => {
    if (!open) return undefined;
    requestSeqRef.current += 1;
    setSelection(new Set());
    setShowHidden(false);
    showHiddenRef.current = false;
    setEditingPath(false);
    setPathDraft("");
    setPathError(null);
    setNewFolderOpen(false);
    setNewFolderName("");
    setNewFolderError(null);
    let active = true;
    void (async () => {
      void window.vua?.fs.navigation?.().then(result => { if (active && result.ok) setNavigation(result.value); }).catch(() => {});
      const remembered = loadLastFolder();
      const first = await navigateTo(remembered);
      // 记忆目录失败回落主目录仅对「初始记忆」适用;手动导航错误就地呈现
      if (!first.ok && active && remembered !== null) await navigateTo(null);
    })();
    return () => {
      active = false;
    };
  }, [open, navigateTo]);

  const currentPath = listing.kind === "loaded" ? listing.list.path : null;
  const currentParent = listing.kind === "loaded" ? listing.list.parent : null;
  const listedEntries = listing.kind === "loaded" ? listing.list.entries : listing.kind === "computer" ? listing.drives : [];

  const showComputer = async () => {
    const seq = ++requestSeqRef.current;
    setListing({ kind: "loading" }); setSelection(new Set()); setNewFolderOpen(false); setPathError(null);
    try {
      const result = await window.vua?.fs.navigation?.();
      if (seq !== requestSeqRef.current) return;
      if (result?.ok) { setNavigation(result.value); setListing({ kind: "computer", drives: result.value.drives }); setPathDraft(""); setEditingPath(false); }
      else setListing({ kind: "error", error: result?.error ?? "failed", target: null });
    } catch { if (seq === requestSeqRef.current) setListing({ kind: "error", error: "failed", target: null }); }
  };

  const toggleSelect = (path: string) => {
    setSelection((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const enterFolder = (path: string) => {
    setPathError(null);
    setEditingPath(false);
    void navigateTo(path);
  };

  const submitPathDraft = async () => {
    const draft = pathDraft.trim();
    if (draft === "") {
      setPathError(copy.errorNotFound);
      return;
    }
    // 失败恢复旧列表:路径编辑的错误内联呈现,不霸占列表区
    const previous = listing;
    const seq = requestSeqRef.current + 1;
    const result = await navigateTo(draft);
    if (seq !== requestSeqRef.current) return;
    if (!result.ok) {
      setListing(previous);
      setPathError(copy[listErrorCopyKey(result.error)]);
      setEditingPath(true);
      return;
    }
    setPathError(null);
    setEditingPath(false);
    editButtonRef.current?.focus();
  };

  const cancelPathEdit = () => {
    setPathError(null);
    setPathDraft(currentPath ?? "");
    setEditingPath(false);
    editButtonRef.current?.focus();
  };

  const toggleShowHidden = () => {
    const next = !showHidden;
    showHiddenRef.current = next;
    setShowHidden(next);
    if (currentPath !== null) void navigateTo(currentPath);
  };

  const submitNewFolder = async () => {
    const api = window.vua?.fs;
    if (api === undefined || listing.kind !== "loaded") return;
    if (!isValidNewFolderName(newFolderName)) {
      setNewFolderError(copy.newFolderInvalid);
      return;
    }
    setNewFolderBusy(true);
    const result = await api.createDirectory(listing.list.path, newFolderName);
    setNewFolderBusy(false);
    if (!result.ok) {
      setNewFolderError(copy[createErrorCopyKey(result.error)]);
      return;
    }
    const createdPath = result.value.path;
    setNewFolderName("");
    setNewFolderError(null);
    setNewFolderOpen(false);
    // 成功:刷新并选中新建目录,停留当前目录(新建不进新目录)
    const refresh = await navigateTo(listing.list.path);
    if (refresh.ok) setSelection(new Set([createdPath]));
  };

  /** 确认收口:记忆当前浏览目录(非选中子项),交选取,关闭 */
  const confirmWith = (folders: readonly string[]) => {
    if (currentPath !== null) saveLastFolder(currentPath);
    onConfirm(folders);
    onClose();
  };

  const handleOpen = () => {
    if (selection.size > 0) {
      confirmWith([...selection]);
      return;
    }
    if (currentPath !== null) confirmWith([currentPath]);
  };

  const handleWindowsPicker = async () => {
    setWindowsBusy(true);
    try {
      const picked = await onUseWindowsPicker();
      if (picked === null || picked.length === 0) return;
      const first = picked[0]!;
      const parent = parentDirOf(first);
      if (parent !== null) saveLastFolder(parent);
      onConfirm(picked);
      onClose();
    } finally {
      setWindowsBusy(false);
    }
  };

  if (!open) return null;
  // portal 到 body:内联渲染会被祖先 .vua-card 的 backdrop-filter 劫持
  // (backdrop-filter 使元素成为 fixed 后代的包含块,遮罩只剩卡片大小);
  // modal-layer 的隔离/焦点循环本就从 document.body 起步,portal 完全兼容
  return createPortal(
    <ModalOwnerContext value={modal.id}>
      <div
        ref={modal.overlayRef}
        className="vua-folder-picker__overlay"
        onClick={modal.closeTop}
      >
        <div
          ref={modal.panelRef}
          tabIndex={-1}
          role="dialog"
          aria-modal="true"
          aria-label={copy.title}
          className="vua-folder-picker"
          onClick={(event) => event.stopPropagation()}
        >
          <div className="vua-folder-picker__header">
            <span className="vua-folder-picker__title">{copy.title}</span>
            <button
              type="button"
              className="vua-folder-picker__close"
              aria-label={strings.common.dialogClose}
              onClick={modal.closeTop}
            >
              ×
            </button>
          </div>

          <div className="vua-folder-picker__path-row">
              <div className="vua-folder-picker__path-edit" data-vua-esc-local={editingPath ? "true" : undefined}>
                <input
                  type="text"
                  value={pathDraft}
                  aria-label={copy.pathInputAria}
                  placeholder={listing.kind === "computer" ? copy.computer : copy.pathLabel}
                  onChange={(event) => {
                    setPathDraft(event.target.value);
                    setEditingPath(true);
                    setPathError(null);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      void submitPathDraft();
                    } else if (event.key === "Escape" && editingPath) {
                      event.preventDefault();
                      event.stopPropagation();
                      cancelPathEdit();
                    }
                  }}
                />
                {pathError !== null ? (
                  <p className="vua-caption vua-folder-picker__inline-error" role="alert">
                    {pathError}
                  </p>
                ) : null}
              </div>
                <button
                  type="button"
                  ref={editButtonRef}
                  className="vua-folder-picker__icon-button"
                  aria-label={copy.editPath}
                  title={copy.editPath}
                  disabled={pathDraft.trim() === ""}
                  onClick={() => void submitPathDraft()}
                >
                  <Icon name="arrow-right" size={16} />
                </button>
          </div>

          <div className="vua-folder-picker__toolbar">
            <Button
              variant="subtle"
              disabled={listing.kind !== "loaded"}
              onClick={() => currentParent !== null ? enterFolder(currentParent) : void showComputer()}
            >
              <Icon name="arrow-up" size={16} />
              {copy.up}
            </Button>
            <Button
              variant="subtle"
              aria-label={copy.refresh}
              title={copy.refresh}
              disabled={listing.kind === "loading"}
              onClick={() => listing.kind === "computer" ? void showComputer() : void navigateTo(currentPath)}
            >
              <Icon name="refresh" size={16} />
            </Button>
            {(["desktop", "documents", "downloads"] as const).map(key => <Button key={key} variant="subtle" disabled={navigation === null} onClick={() => { if (navigation) enterFolder(navigation[key]); }}>{copy[key]}</Button>)}
            <Button variant="subtle" onClick={() => void showComputer()}>{copy.computer}</Button>
          </div>

          {newFolderOpen ? (
            <div className="vua-folder-picker__new-folder" data-vua-esc-local="true">
              <input
                type="text"
                value={newFolderName}
                aria-label={copy.newFolderNameAria}
                autoFocus
                disabled={newFolderBusy}
                onChange={(event) => {
                  setNewFolderName(event.target.value);
                  setNewFolderError(null);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    void submitNewFolder();
                  } else if (event.key === "Escape") {
                    event.preventDefault();
                    event.stopPropagation();
                    setNewFolderOpen(false);
                    setNewFolderName("");
                    setNewFolderError(null);
                  }
                }}
              />
              <Button variant="default" disabled={newFolderBusy} onClick={() => void submitNewFolder()}>
                {copy.newFolderSubmit}
              </Button>
              {newFolderError !== null ? (
                <p className="vua-caption vua-folder-picker__inline-error" role="alert">
                  {newFolderError}
                </p>
              ) : null}
            </div>
          ) : null}

          <div className="vua-folder-picker__list" role="list" aria-label={copy.pathLabel}>
            {listing.kind === "loading" ? (
              <p className="vua-caption vua-text-secondary">{copy.loading}</p>
            ) : null}
            {listing.kind === "error" ? (
              <div className="vua-folder-picker__error" role="alert">
                <p className="vua-caption vua-text-secondary">{copy[listErrorCopyKey(listing.error)]}</p>
                <Button
                  variant="default"
                  onClick={() => void navigateTo(listing.target ?? currentPath)}
                >
                  {copy.retry}
                </Button>
              </div>
            ) : null}
            {(listing.kind === "loaded" || listing.kind === "computer") && listedEntries.length === 0 ? (
              <p className="vua-caption vua-text-secondary">{copy.emptyFolder}</p>
            ) : null}
            {(listing.kind === "loaded" || listing.kind === "computer")
              ? listedEntries.map((entry) => (
                  <div
                    key={entry.path}
                    role="listitem"
                    className="vua-folder-picker__row"
                    data-selected={selection.has(entry.path) || undefined}
                    onClick={() => toggleSelect(entry.path)}
                    onDoubleClick={() => enterFolder(entry.path)}
                  >
                    <Icon name="folder" size={16} />
                    <span className="vua-folder-picker__row-name" title={entry.name}>
                      {entry.name}
                    </span>
                    {selection.has(entry.path) ? <Icon name="check" size={16} /> : null}
                    <button
                      type="button"
                      className="vua-folder-picker__icon-button"
                      aria-label={entry.name}
                      title={entry.name}
                      onClick={(event) => {
                        event.stopPropagation();
                        enterFolder(entry.path);
                      }}
                    >
                      <Icon name="arrow-right" size={16} />
                    </button>
                  </div>
                ))
              : null}
          </div>

          <div className="vua-folder-picker__footer">
            <div className="vua-folder-picker__footer-left">
              <Button
                variant="subtle"
                disabled={currentPath === null}
                onClick={() => {
                  setNewFolderOpen((value) => !value);
                  setNewFolderError(null);
                }}
              >
                {"+ "}
                {copy.newFolder}
              </Button>
              <Button variant="subtle" disabled={windowsBusy} onClick={() => void handleWindowsPicker()}>
                {copy.windowsPicker}
              </Button>
              <button
                type="button"
                className="vua-folder-picker__toggle"
                aria-pressed={showHidden}
                onClick={toggleShowHidden}
              >
                {copy.showHidden}
              </button>
            </div>
            <div className="vua-folder-picker__footer-right">
              <Button variant="default" onClick={modal.closeTop}>
                {copy.cancel}
              </Button>
              <Button variant="primary" disabled={listing.kind !== "loaded" && listing.kind !== "computer" || currentPath === null && selection.size === 0} onClick={handleOpen}>
                {copy.openCta}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </ModalOwnerContext>,
    document.body,
  );
}
