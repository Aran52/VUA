/** A factory room keeps its accent colour; a room with several views shows their tabs. The sidebar
 * and the page title already name the room, so nothing else repeats it here. */
import type { ReactNode } from "react";
import type { PageId } from "../../app/nav-model.ts";
import { strings } from "../../i18n/index.ts";
import "./factory.css";

const copy = strings.factory;
export type RoomId = "materials" | "workshop" | "control" | "release";
type TabKey = keyof typeof copy.tabs;

const ROOM_OF_PAGE: Partial<Record<PageId, RoomId>> = {
  warehouse: "materials", "asset-browser": "materials", workshop: "workshop", recipe: "control", packages: "control", release: "release",
};
const MATERIAL_TABS: readonly { page: PageId; label: TabKey }[] = [{ page: "warehouse", label: "library" }, { page: "asset-browser", label: "browse" }];

export function roomOf(page: PageId): RoomId | null {
  return ROOM_OF_PAGE[page] ?? null;
}

export function AmfRoom({ page, navigate, children }: { page: PageId; navigate: (page: PageId) => void; children: ReactNode }) {
  const room = roomOf(page);
  if (room === null) return <>{children}</>;
  return <div className="vua-room" data-room={room}>
    {room === "materials" ? <nav className="vua-room__bar" aria-label={copy.rooms.materials.name}>
      <span className="vua-room__tabs" role="tablist">{MATERIAL_TABS.map(tab => <button type="button" role="tab" key={tab.page} aria-selected={tab.page === page}
        data-nav-id={`tab-${tab.page}`} onClick={() => navigate(tab.page)}>{copy.tabs[tab.label]}</button>)}</span>
    </nav> : null}
    <div className="vua-room__content">{children}</div>
  </div>;
}
