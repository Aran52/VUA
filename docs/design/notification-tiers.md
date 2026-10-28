# Notification tiers

> Updated: 2026-10-21
> Scope: every message, confirmation and side panel the desktop app shows the player

VUA tells the player things at six levels of interruption. A message takes the lowest tier that
still gets it read, and each tier has one component in `apps/desktop/src/renderer/components/primitives/`.
The [design standard](design-standard.md) keeps the visual language; this document owns which tier
a message uses.

## The six tiers

| Tier | Component | Interrupts | Leaves | Use for |
| --- | --- | --- | --- | --- |
| 1 Tooltip | `Tooltip` | nothing | when the pointer or focus leaves | what an icon or control does, the consequence of a button (Stop keeps the project), why a card is in development |
| 2 Toast | `toast()` with `ToastHost` | nothing | by itself after about two seconds, longer for longer text (at most six); hovering pauses it | feedback after an action that needs nothing more: copied, added to downloads, sync started, repository added |
| 3 Alert | `Alert` | nothing; it speaks up | when the condition ends or the player dismisses it | a failure with its retry, a blocked step, a queued task, a condition to know before acting |
| 4 Popconfirm | `Popconfirm` | the one control | after the answer, Escape or a click elsewhere | one question before an everyday action the player can undo or redo: disable a module, remove a repository, change a line's materials, restore the previous version |
| 5 Drawer | `Drawer` | nothing; the page stays usable | when closed | longer content read next to the page: package details, terms, the pinned notification list |
| 6 Modal | `ConfirmDialog`, `ContentDialog` | everything; the background dims | only by a choice | decisions that cannot be undone or carry real risk (delete a line, delete originals, full access, sign-in risk, leave with unsaved work) and page-sized tasks (upload, import, recipe apply) |

Choose from the top: if a tooltip is enough, nothing else is used. A message the player must act on
is never a toast. A confirmation the player meets several times a session is a popconfirm, not a
modal. A modal asks one decision; content that only informs goes in a drawer or the page.

Long-running work reports in the notification center (the bell). Its redesign, with the pinned
drawer and Windows notifications for work that ends in the background, is slice 40 of the
[current plan](../development/current-plan.md); the bell follows this document's tiers.

## Behavior

- **Tooltip.** Shows 350 ms after the pointer arrives, at once on keyboard focus; Escape, scrolling
  or a press hides it. It works on a disabled control. Text only, at most two short lines.
- **Toast.** Bottom center, at most three stacked; the same text twice replaces the first. The stack
  is a polite live region. No buttons: anything with an action is an alert.
- **Alert.** Sits at the top of the page, panel or dialog it concerns. Tones: info, success, warning,
  error. Error alerts use `role="alert"`; the rest are polite. It may hold one or two buttons that
  resolve the condition and a dismiss button.
- **Popconfirm.** Opens below its button (above when there is no room), focuses Cancel, and returns
  focus to the button on Escape or Cancel. It opens above dialogs, so it works inside one. The danger
  variant colors the title and the confirm button.
- **Drawer.** Slides in from the right under the title bar, without a backdrop; Escape inside it
  closes it. Width 420 px by default.
- **Modal.** The shared modal stack keeps focus inside, closes on Escape and returns focus. Only a
  dangerous confirmation has a red title.

## Layers and motion

Layers are tokens in `packages/design-system/src/tokens.css`: browser frame 500, drawer 600, dialog 800, popover 900,
toast 1000, tooltip 1100. Anything that can open from inside a dialog sits above it.

The global notification backdrop/panel sit at 700/701, above browser chrome. While notifications
are present, including their closing animation, `data-vua-native-occluder` makes the native
webpage yield its viewport; closing restores the retained browser view without navigation or
reload. Browser-backed import modals allow these host popovers through their focus/isolation
boundary. Notification surfaces retain the host's rectangular/rounded shape while AMF is open.

Every tier moves with scripted motion from the motion tokens (`--vua-motion-fast`,
`--vua-motion-base`, `--vua-motion-ease`) through `motion.ts`, so Windows' animation setting does
not flatten it and resource-saving mode stills it.

## Writing

Each tier says its thing once, in the player's words, following the
[writing rules](../meta/documentation-governance.md#6-writing-style): a popconfirm is a question
title plus at most one sentence of consequence; a toast is a short fact; an alert names the
condition and what resolves it.

## Where VUA uses each tier

| Message | Tier | State |
| --- | --- | --- |
| Icon buttons in the title bar (home, notifications, window controls), the room's map button | Tooltip | done |
| In-development headsets and Unity 6 | Tooltip | done |
| Factory room names (click to look inside, double-click to enter) | Tooltip | done |
| Workshop Stop (Unity ends, the project stays); play card Stop (only this card's software closes) | Tooltip | done |
| Package repository checkbox | Tooltip | done |
| Copied (AI connection text, Avatar ID) | Toast | done |
| Library sync started, already running, finished; downloads added | Toast | done |
| Repository added, removed, refreshed; opening a project in Unity | Toast | done |
| Package changes applied or nothing to change (development data) | Toast | done |
| Library sync warnings and failures, download progress notes | Alert | done |
| Workshop task error with retry, Unity license not active, queued work | Alert | done |
| Release action errors, SDK must-fix count and AI-requested upload in the upload dialog | Alert | done |
| Package page errors | Alert | done |
| Materials start dialog load and start failures, missing editor | Alert | done |
| Disable AMF | Popconfirm | done |
| Restore the previous AMF version | Popconfirm | done |
| Remove a package repository | Popconfirm | done |
| Change a line's materials | Popconfirm | done |
| Show pre-release packages (inline confirmation panel) | Popconfirm | follow-up |
| Package details (development data) | Drawer | follow-up: move to `Drawer` |
| Notification center, pinned | Drawer | slice 40 |
| Delete a line, delete originals after generating, full access, VRChat sign-in risk, leave with unsaved work, quit during a task | Modal | in place |
| Upload, import, recipe apply, share and import, materials start | Modal (content) | in place |
| Long-running work and AI requests | Notification center | slice 40 |
