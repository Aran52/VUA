import { useEffect, useState } from "react";
import type { ConfirmedEditorV1 } from "@vua/contracts";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { format, strings } from "../../i18n/index.ts";
import { saveUnityMirrors, useUnityMirrors } from "../../app/unity-download-preference.ts";
import {
  narrowEditorFindings,
  narrowVerifyEditorResult,
  refusalCodeKey,
  type EditorFindingNarrowed,
  type VerifyEditorNarrowed,
} from "./environment-settings-model.ts";

/**
 * U10「环境与路径」设置面(021 桌面半边;ADR path-configuration 验收标准
 * 第 5 条:逐行含来源):
 * - 预填候选 = project.environmentManagers 读面(021 收敛点 1 单一面),
 *   来源恒「已探测」(检测侧唯一真实来源,渲染层常量呈现);
 * - 手选 = 单一「浏览」入口双态(exe/目录),路径三形态原样透传
 *   environment.verifyEditor(桌面零本地归一化,归一化是原语职责);
 * - 验证结果就地形呈现:Verified 六字段 / 拒绝码 i18n 映射 + detail 原文
 *   (钉子二:零加工);拒绝 = 正常发现,不隐藏不猜测;
 * - 门③(021 仲裁):信任呈现 + 首次确认一次 + 留痕 = 壳编辑器设置
 *   (机器级 settings);选择变更 = 新选择,确认重新起算。注入生效时机 =
 *   provider 进程启动,如实标注。
 * 诚实纪律:所有呈现值来自 Gateway 实际返回;探测空 = 诚实空态;
 * 词表外码照原词呈现,不猜测映射。
 */

type EditorsState =
  | { readonly kind: "loading" }
  | { readonly kind: "unavailable" }
  | { readonly kind: "loaded"; readonly editors: readonly EditorFindingNarrowed[] | null };

type VerifyState =
  | { readonly kind: "idle" }
  | { readonly kind: "verifying"; readonly path: string }
  | { readonly kind: "failed"; readonly path: string }
  | { readonly kind: "unavailable"; readonly path: string }
  /** narrowed = null 表示返回形状不可解释(不猜测,按不可解读呈现) */
  | { readonly kind: "done"; readonly path: string; readonly narrowed: VerifyEditorNarrowed | null };

function classificationText(classification: string, known: boolean): string {
  if (!known) return classification;
  const copy = strings.settings.environment;
  switch (classification) {
    case "production_target": return copy.classProductionTarget;
    case "migration_source": return copy.classMigrationSource;
    case "other_unity_version": return copy.classOtherVersion;
    case "tuanjie_family": return copy.classTuanjie;
    default: return classification;
  }
}

export function EnvironmentSettingsPage() {
  const copy = strings.settings.environment;
  const useMirrors = useUnityMirrors();
  const [editorsState, setEditorsState] = useState<EditorsState>({ kind: "loading" });
  const [reloadKey, setReloadKey] = useState(0);
  const [verify, setVerify] = useState<VerifyState>({ kind: "idle" });
  const [confirmed, setConfirmed] = useState<ConfirmedEditorV1 | null>(null);
  const [saveFailed, setSaveFailed] = useState(false);

  useEffect(() => {
    let active = true;
    setEditorsState({ kind: "loading" });
    void window.vua?.gateway
      .invoke({
        schemaVersion: 1,
        requestId: crypto.randomUUID(),
        method: "project.environmentManagers",
        params: {},
      })
      .then((result) => {
        if (!active) return;
        if (!result.ok) {
          setEditorsState({ kind: "unavailable" });
          return;
        }
        // wire 值 = project-inspection 信封,快照本体在内层 result(021
        // 收敛点 1 解包纪律,与 ProjectCompatPage 修正同构)
        const envelope = result.value as { result?: unknown };
        const record = (envelope.result ?? null) as Record<string, unknown> | null;
        setEditorsState({
          kind: "loaded",
          editors: record === null ? null : narrowEditorFindings(record.editors),
        });
      });
    void window.vua?.editorSettings.read().then((settings) => {
      if (!active) return;
      setConfirmed(settings.confirmedEditor);
    });
    return () => {
      active = false;
    };
  }, [reloadKey]);

  const verifyPath = (path: string) => {
    setVerify({ kind: "verifying", path });
    void window.vua?.gateway
      .invoke({
        schemaVersion: 1,
        requestId: crypto.randomUUID(),
        method: "environment.verifyEditor",
        params: { path },
      })
      .then((result) => {
        if (!result.ok) {
          // 缺席码 = 路由未接线/原语不可达(裁决⑤),绝不冒充验证拒绝
          const code = result.error.code === "application"
            ? result.error.application.code
            : null;
          setVerify(
            code === "vua.environment.verify_unavailable"
              ? { kind: "unavailable", path }
              : { kind: "failed", path },
          );
          return;
        }
        setVerify({ kind: "done", path, narrowed: narrowVerifyEditorResult(result.value) });
      });
  };

  const confirmEditor = (narrowed: Extract<VerifyEditorNarrowed, { kind: "verified" }>) => {
    const next: ConfirmedEditorV1 = {
      path: narrowed.exePath,
      version: narrowed.version,
      confirmedAt: new Date().toISOString(),
    };
    void window.vua?.editorSettings
      .save({ schemaVersion: 1, confirmedEditor: next })
      .then((saved) => {
        setSaveFailed(false);
        setConfirmed(saved.confirmedEditor);
      })
      .catch(() => setSaveFailed(true));
  };

  const clearConfirmed = () => {
    void window.vua?.editorSettings
      .save({ schemaVersion: 1, confirmedEditor: null })
      .then((saved) => {
        setSaveFailed(false);
        setConfirmed(saved.confirmedEditor);
      })
      .catch(() => setSaveFailed(true));
  };

  const verifiedNow = verify.kind === "done" && verify.narrowed?.kind === "verified"
    ? verify.narrowed
    : null;
  const refusedNow = verify.kind === "done" && verify.narrowed?.kind === "refused"
    ? verify.narrowed
    : null;
  const invalidNow = verify.kind === "done" && verify.narrowed === null;

  return (
    <div className="vua-page">
      <section className="vua-page__hero">
        <h1 className="vua-title">{strings.nav.pages.settingsEnvironment}</h1>
      </section>

      <Card>
        <div className="vua-page__stack">
          <h2 className="vua-title">{strings.deployment.mirrorHeading}</h2>
          <label className="vua-settings-row">
            <input type="checkbox" checked={useMirrors} onChange={event => saveUnityMirrors(event.target.checked)} />
            {strings.deployment.mirrorLabel}
          </label>
          <p className="vua-caption vua-text-secondary">{strings.deployment.mirrorHint}</p>
        </div>
      </Card>

      <Card>
        <div className="vua-page__stack">
          <h2 className="vua-title">{copy.probedHeading}</h2>
          {editorsState.kind === "loading" ? (
            <p className="vua-caption vua-text-secondary">{copy.loading}</p>
          ) : null}
          {editorsState.kind === "unavailable" ? (
            <p className="vua-caption vua-text-secondary" role="alert">{copy.unavailable}</p>
          ) : null}
          {editorsState.kind === "loaded" ? (
            editorsState.editors === null ? (
              <p className="vua-caption vua-text-secondary">{copy.uninterpretable}</p>
            ) : editorsState.editors.length === 0 ? (
              <p className="vua-caption vua-text-secondary">{copy.probedEmpty}</p>
            ) : (
              <ul className="vua-project-compat__specs">
                {editorsState.editors.map((editor) => (
                  <li key={`${editor.path}#${editor.version}`}>
                    {copy.sourceProbed} · {editor.version}
                    {" · "}
                    {classificationText(editor.classification, editor.classificationKnown)}
                    {editor.chinaDistribution ? ` · ${copy.chinaMark}` : null}
                    <br />
                    <span className="vua-caption vua-text-secondary">{editor.path}</span>
                  </li>
                ))}
              </ul>
            )
          ) : null}
        </div>
      </Card>

      <Card>
        <div className="vua-page__stack">
          <h2 className="vua-title">{copy.pickHeading}</h2>
          <div className="vua-settings-row">
            <Button
              variant="default"
              onClick={() => {
                void window.vua?.dialog.pickEditorExecutable().then((picked) => {
                  if (picked !== null) verifyPath(picked);
                });
              }}
            >
              {copy.pickFileCta}
            </Button>
            <Button
              variant="default"
              onClick={() => {
                void window.vua?.dialog.pickEditorDirectory().then((picked) => {
                  if (picked !== null) verifyPath(picked);
                });
              }}
            >
              {copy.pickDirCta}
            </Button>
          </div>

          {verify.kind === "verifying" ? (
            <p className="vua-caption vua-text-secondary">{format(copy.verifying, { path: verify.path })}</p>
          ) : null}
          {verify.kind === "unavailable" ? (
            <p className="vua-caption vua-text-secondary" role="alert">{copy.verifyUnavailable}</p>
          ) : null}
          {verify.kind === "failed" ? (
            <p className="vua-caption vua-text-secondary" role="alert">{copy.verifyFailed}</p>
          ) : null}
          {invalidNow ? (
            <p className="vua-caption vua-text-secondary" role="alert">{copy.uninterpretable}</p>
          ) : null}

          {refusedNow ? (
            <div role="status" className="vua-page__stack">
              <p className="vua-caption">{copy.refusedHeading}</p>
              <p className="vua-caption vua-text-secondary">
                {refusalText(refusedNow.code, refusedNow.codeKnown)}
              </p>
              {/* 钉子二:detail 为原语资源原文,逐字呈现零加工 */}
              <p className="vua-caption vua-text-secondary">{refusedNow.detail}</p>
            </div>
          ) : null}

          {verifiedNow ? (
            <div role="status" className="vua-page__stack">
              <p className="vua-caption">{copy.verifiedHeading}</p>
              <p>
                {verifiedNow.version}
                {" · "}
                {classificationText(verifiedNow.classification, verifiedNow.classificationKnown)}
                {verifiedNow.chinaDistribution ? ` · ${copy.chinaMark}` : null}
                <br />
                <span className="vua-caption vua-text-secondary">{verifiedNow.exePath}</span>
                <br />
                <span className="vua-caption vua-text-secondary">{verifiedNow.editorRoot}</span>
              </p>
              <div>
                <Button variant="primary" onClick={() => confirmEditor(verifiedNow)}>
                  {copy.confirmCta}
                </Button>
              </div>
            </div>
          ) : null}
        </div>
      </Card>

      <Card>
        <div className="vua-page__stack">
          <h2 className="vua-title">{copy.confirmedHeading}</h2>
          {confirmed === null ? (
            <p className="vua-caption vua-text-secondary">{copy.confirmedEmpty}</p>
          ) : (
            <>
              <p>
                {copy.sourcePicked} · {confirmed.version}
                <br />
                <span className="vua-caption vua-text-secondary">{confirmed.path}</span>
              </p>
              <p className="vua-caption vua-text-secondary">
                {format(copy.confirmedAt, { time: confirmed.confirmedAt })}
              </p>
              <p className="vua-caption vua-text-secondary">{copy.effectiveNote}</p>
              <div>
                <Button variant="default" onClick={clearConfirmed}>{copy.clearCta}</Button>
              </div>
            </>
          )}
          {saveFailed ? (
            <p className="vua-caption vua-text-secondary" role="alert">{copy.saveFailed}</p>
          ) : null}
        </div>
      </Card>
    </div>
  );
}

function refusalText(code: string, known: boolean): string {
  const copy = strings.settings.environment;
  if (!known) return code;
  const key = refusalCodeKey(code);
  switch (key) {
    case "target_missing": return copy.refusalTargetMissing;
    case "exe_missing": return copy.refusalExeMissing;
    case "identity_unreadable": return copy.refusalIdentityUnreadable;
    case "not_an_editor": return copy.refusalNotAnEditor;
    case "unsupported_platform": return copy.refusalUnsupportedPlatform;
    default: return code;
  }
}
