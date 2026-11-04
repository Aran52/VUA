/**
 * 桌面教程伴随窗口表面(G4 P0)。
 * 经 ?surface=tutorial 在应用初始化最早阶段分流渲染:不初始化主壳
 * Gateway、DEV scenario、路由与业务 store;只挂教程端口。
 *
 * 语义(契约冻结):窗口关闭按钮 = dismiss(终止整份共享教程),
 * 由 Rust 端在状态迁移后统一关窗;open_on_desktop 不在此表面出现
 * (桌面窗口自身即落点,该动作仅 VR 表面有意义)。
 */
import { useCallback, useEffect, useState } from "react";
import { tutorialPort } from "../../app/tutorial-port-instance.ts";
import type { TutorialSnapshot } from "../../app/tutorial-port.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { Icon } from "@vua/design-system";
import { format, strings } from "../../i18n/index.ts";
import { useSurfaceAppearance } from "../../app/surface-appearance.ts";
import "./tutorial-surface.css";

const copy = strings.tutorial;

/** 首帧快照超时:超过即判定后端不可达,进入失败态而非永久空白 */
const SNAPSHOT_TIMEOUT_MS = 4000;
/** 壳层窗口动作(preload 注入);浏览器预览无 preload */
const nativeWindow = window.vua?.window;
/** 置顶(ADR-0002 optionally topmost)需要独立窗口动作,随 Electron
 * 教程切片(M5)进入 preload;此前置顶按钮不渲染 */
const topmostAvailable = false;
const DISMISS_TIMEOUT_MS = 2000;

type StepId = keyof typeof copy.steps;

function stepCopy(id: string | null) {
  return id !== null && id in copy.steps ? copy.steps[id as StepId] : null;
}

export function TutorialSurface() {
  useSurfaceAppearance();
  const [snapshot, setSnapshot] = useState<TutorialSnapshot | null>(null);
  const [topmost, setTopmost] = useState(false);
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    let alive = true;
    // 首帧快照必须带超时与失败态:invoke 永不返回(应用层冻结/后端不可达)
    // 时窗口不得停留在无内容的空白态——那会让无边框窗口既无信息也
    // 没有可用关闭路径
    const timeout = new Promise<null>((resolve) => {
      setTimeout(() => resolve(null), SNAPSHOT_TIMEOUT_MS);
    });
    void Promise.race([tutorialPort.snapshot(), timeout])
      .then((s) => {
        if (!alive) return;
        if (s === null) setLoadFailed(true);
        else setSnapshot(s);
      })
      .catch(() => {
        if (alive) setLoadFailed(true);
      });
    const unsubscribe = tutorialPort.subscribe(setSnapshot);
    return () => {
      alive = false;
      unsubscribe();
    };
  }, []);

  const dispatch = useCallback((action: "next" | "back" | "dismiss") => {
    void tutorialPort.dispatch(action).then((result) => {
      // stale / rejected 都携带最新快照,以其为准重新同步
      setSnapshot(result.snapshot);
    });
  }, []);

  /** 关闭 = dismiss(终止共享教程);后端不可达时退化为原生关窗,
      保证窗口永远关得掉 */
  const closeSurface = useCallback(() => {
    const nativeClose = () => void nativeWindow?.close();
    if (!nativeWindow) return;
    const timeout = new Promise<null>((resolve) => {
      setTimeout(() => resolve(null), DISMISS_TIMEOUT_MS);
    });
    void Promise.race([tutorialPort.dispatch("dismiss"), timeout])
      .then((result) => {
        if (result === null) nativeClose();
      })
      .catch(nativeClose);
  }, []);

  const retrySnapshot = useCallback(() => {
    setLoadFailed(false);
    setSnapshot(null);
    const timeout = new Promise<null>((resolve) => {
      setTimeout(() => resolve(null), SNAPSHOT_TIMEOUT_MS);
    });
    void Promise.race([tutorialPort.snapshot(), timeout])
      .then((s) => {
        if (s === null) setLoadFailed(true);
        else setSnapshot(s);
      })
      .catch(() => setLoadFailed(true));
  }, []);

  const toggleTopmost = useCallback(() => {
    void topmost;
  }, [topmost]);

  const step = stepCopy(snapshot?.currentStepId ?? null);

  return (
    <div className="vua-tutorial">
      <header className="vua-tutorial__titlebar vua-drag-region">
        <span className="vua-tutorial__title vua-drag-region">{copy.surfaceTitle}</span>
        {topmostAvailable ? (
          <button
            type="button"
            className="vua-tutorial__chrome-button"
            aria-pressed={topmost}
            aria-label={topmost ? copy.topmostOn : copy.topmostOff}
            title={topmost ? copy.topmostOn : copy.topmostOff}
            onClick={toggleTopmost}
          >
            <Icon name="pin" size={16} />
          </button>
        ) : null}
        <button
          type="button"
          className="vua-tutorial__chrome-button vua-tutorial__chrome-button--close"
          aria-label={copy.dismiss}
          onClick={closeSurface}
        >
          <Icon name="close" size={16} />
        </button>
      </header>

      <main className="vua-tutorial__body">
        {loadFailed ? (
          <>
            <h1 className="vua-title">{copy.loadErrorTitle}</h1>
            <p className="vua-tutorial__step-body">{copy.loadErrorBody}</p>
            <div className="vua-tutorial__actions">
              <Button variant="default" onClick={retrySnapshot}>
                {copy.retry}
              </Button>
              <Button variant="primary" onClick={() => void nativeWindow?.close()}>
                {copy.closeWindow}
              </Button>
            </div>
          </>
        ) : snapshot === null ? null : snapshot.status === "active" && step ? (
          <>
            <p className="vua-caption vua-text-secondary">
              {format(copy.progress, {
                index: snapshot.stepIndex,
                total: snapshot.stepTotal,
              })}
            </p>
            <h1 className="vua-title">{step.title}</h1>
            <p className="vua-tutorial__step-body">{step.body}</p>
            <div className="vua-tutorial__actions">
              {snapshot.allowedActions.includes("back") ? (
                <Button variant="default" onClick={() => dispatch("back")}>
                  {copy.back}
                </Button>
              ) : null}
              <Button variant="primary" onClick={() => dispatch("next")}>
                {copy.next}
              </Button>
            </div>
          </>
        ) : snapshot.status === "completed" ? (
          <>
            <h1 className="vua-title">{copy.completedTitle}</h1>
            <p className="vua-tutorial__step-body">{copy.completedBody}</p>
            <div className="vua-tutorial__actions">
              <Button variant="primary" onClick={closeSurface}>
                {copy.dismiss}
              </Button>
            </div>
          </>
        ) : (
          <>
            <h1 className="vua-title">{copy.inactiveTitle}</h1>
            <p className="vua-tutorial__step-body">{copy.inactiveBody}</p>
          </>
        )}
      </main>
    </div>
  );
}
