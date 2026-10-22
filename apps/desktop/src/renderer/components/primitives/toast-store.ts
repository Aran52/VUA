/**
 * Tier 2, toast: brief feedback after an action that needs nothing more from the player
 * ("Copied", "Added to downloads"). It leaves by itself; anything the player must act on is an
 * alert, a confirmation or a notification-center entry instead.
 */
export type ToastTone = "neutral" | "success" | "warning" | "error";

export interface ToastItem {
  readonly id: number;
  readonly text: string;
  readonly tone: ToastTone;
}

const MAX_VISIBLE = 3;
let items: readonly ToastItem[] = [];
let nextId = 1;
const listeners = new Set<() => void>();
const emit = () => { for (const listener of listeners) listener(); };

export function toast(text: string, tone: ToastTone = "neutral"): number {
  const id = nextId++;
  // The same message twice in a row replaces the first instead of stacking.
  items = [...items.filter(item => item.text !== text), { id, text, tone }].slice(-MAX_VISIBLE);
  emit();
  return id;
}

export function dismissToast(id: number): void {
  if (!items.some(item => item.id === id)) return;
  items = items.filter(item => item.id !== id);
  emit();
}

/** How long a toast stays: about two seconds, longer for longer text, at most six. */
export function toastDuration(text: string): number {
  return Math.min(6000, 2400 + Math.max(0, text.length - 16) * 60);
}

export function subscribeToasts(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

export function toastSnapshot(): readonly ToastItem[] {
  return items;
}
