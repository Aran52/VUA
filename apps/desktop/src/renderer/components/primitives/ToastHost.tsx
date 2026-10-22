import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { createPortal } from "react-dom";
import { Icon, type IconName } from "@vua/design-system";
import { canAnimate, easing, fastMs, motionOff } from "./motion.ts";
import { dismissToast, subscribeToasts, toastDuration, toastSnapshot, type ToastItem, type ToastTone } from "./toast-store.ts";
import "./toast.css";

const ICONS: Record<ToastTone, IconName | null> = { neutral: null, success: "check", warning: "warning", error: "error-circle" };

/** The toast stack, mounted once per window. The live region stays in the DOM so readers announce each toast. */
export function ToastHost() {
  const items = useSyncExternalStore(subscribeToasts, toastSnapshot, toastSnapshot);
  return createPortal(<div className="vua-toast-stack" role="status" aria-live="polite">
    {items.map(item => <Toast key={item.id} item={item} />)}
  </div>, document.body);
}

function Toast({ item }: { item: ToastItem }) {
  const ref = useRef<HTMLDivElement>(null);
  const [paused, setPaused] = useState(false);
  const [leaving, setLeaving] = useState(false);

  useLayoutEffect(() => {
    if (motionOff() || !canAnimate(ref.current)) return;
    const animation = ref.current.animate([{ opacity: 0, transform: "translateY(12px) scale(0.98)" }, { opacity: 1, transform: "none" }], { duration: fastMs() * 1.6, easing: easing() });
    return () => animation.cancel();
  }, []);

  useEffect(() => {
    if (paused || leaving) return;
    const timer = window.setTimeout(() => setLeaving(true), toastDuration(item.text));
    return () => window.clearTimeout(timer);
  }, [paused, leaving, item.text]);

  useEffect(() => {
    if (!leaving) return;
    if (motionOff() || !canAnimate(ref.current)) { dismissToast(item.id); return; }
    const animation = ref.current.animate([{ opacity: 1, transform: "none" }, { opacity: 0, transform: "translateY(6px)" }], { duration: fastMs(), easing: "ease-in", fill: "forwards" });
    let active = true;
    const done = () => { if (active) dismissToast(item.id); };
    animation.finished.then(done, done);
    return () => { active = false; animation.cancel(); };
  }, [leaving, item.id]);

  const icon = ICONS[item.tone];
  return <div ref={ref} className="vua-toast" data-tone={item.tone}
    onPointerEnter={() => setPaused(true)} onPointerLeave={() => setPaused(false)}>
    {icon ? <Icon name={icon} size={16} /> : null}
    <span>{item.text}</span>
  </div>;
}
