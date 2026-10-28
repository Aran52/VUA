import { Icon, useSkinFamily } from "@vua/design-system";
import type { PageId } from "../../app/nav-model.ts";
import { format, strings, TERMS } from "../../i18n/index.ts";
import { directory } from "../home/HomePage.tsx";

/** Theme-only shell parts (author ruling 2026-10-28): Frutiger Aero draws Vista's address bar,
 * command bar and details pane. They mirror existing navigation and actions; no other theme
 * renders them. */

function locate(page: PageId): { group: string | null; title: string } {
  for (const group of ["env", "production"] as const) {
    for (const item of directory[group]) {
      if (item.id === page || ("pages" in item && (item.pages as readonly string[]).includes(page))) {
        return { group: group === "env" ? strings.journey.environment : TERMS.amf, title: item.title };
      }
    }
  }
  return { group: null, title: page.startsWith("settings") ? strings.nav.tabs.settings : page.startsWith("help") ? strings.journey.help : strings.journey.home };
}

/** Vista's breadcrumb address bar and search box, inside the glass header. */
export function ThemeAddress({ page, navigate, openPalette }: { page: PageId; navigate: (page: PageId) => void; openPalette: () => void }) {
  const family = useSkinFamily();
  if (family !== "frutiger-aero") return null;
  const where = locate(page);
  return <div className="vua-theme-address">
    <nav className="vua-theme-address__crumbs" aria-label={strings.journey.home}>
      <button type="button" onClick={() => navigate("home")}>VUA</button>
      {where.group ? <><span aria-hidden="true">▸</span><span>{where.group}</span></> : null}
      <span aria-hidden="true">▸</span><span aria-current="page">{where.title}</span>
    </nav>
    <button type="button" className="vua-theme-address__search" onClick={openPalette} aria-keyshortcuts="Control+P Meta+P">
      <span>{strings.commandPalette.cta}</span>
    </button>
  </div>;
}

/** Vista's dark command bar: the header's help and settings, and the module list. */
export function ThemeCommandBar({ page, navigate, openSettings }: { page: PageId; navigate: (page: PageId) => void; openSettings: () => void }) {
  const family = useSkinFamily();
  if (family !== "frutiger-aero") return null;
  return <div className="vua-theme-commandbar" role="toolbar" aria-label={strings.app.sidebarAria}>
    <button type="button" aria-pressed={page.startsWith("help")} onClick={() => navigate("help")}>{strings.journey.help}</button>
    <button type="button" aria-pressed={page.startsWith("settings")} onClick={openSettings}>{strings.nav.tabs.settings}</button>
    <button type="button" onClick={() => navigate("settings-modules")}>{strings.amfModule.title}</button>
  </div>;
}

/** Vista's details pane: where the player is and which VUA this is. */
export function ThemeDetails({ page }: { page: PageId }) {
  const family = useSkinFamily();
  if (family !== "frutiger-aero") return null;
  const where = locate(page);
  return <footer className="vua-theme-details">
    <Icon name="computer" size={24} />
    <div><strong>{where.title}</strong><span>{strings.settings.version.heading} · {format(strings.settings.version.versionLine, { version: __VUA_BUILD_INFO__.version })}</span></div>
  </footer>;
}
