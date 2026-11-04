import type { PageId } from "../../app/nav-model.ts";
import { strings, presentationCopy, formatDateTime, TERMS } from "../../i18n/index.ts";
import { Icon } from "@vua/design-system";
import { Button } from "../../components/primitives/Button.tsx";
import { useEnvironmentView, useTaskCenter } from "../../gateway/index.ts";
import { isActiveStatus, taskRowOpenTarget } from "../task-center/notification-model.ts";
import { RouteTile } from "../../components/RouteTile.tsx";
const copy = strings.journey;
export const directory = {
  env: [{ id: "env-play", title: copy.play, icon: "anim" }, { id: "tools-discover", title: copy.tools, icon: "tools" }, { id: "env-create", title: copy.create, icon: "flask" }],
  // The factory rooms (2026-10-17 author ruling): each room holds the pages listed in pages. The control
  // center comes first, right under AMF (author ruling 2026-10-18).
  production: [{ id: "recipe", title: strings.factory.rooms.control.name, icon: "edit", pages: ["recipe", "packages"] }, { id: "warehouse", title: strings.factory.rooms.materials.name, icon: "folder", pages: ["warehouse", "asset-browser"] }, { id: "workshop", title: strings.factory.rooms.workshop.name, icon: "avatar", pages: ["workshop"] }, { id: "release", title: strings.factory.rooms.release.name, icon: "anim", pages: ["release"] }],
} as const;
export function HomePage({ page, navigate, amfInstalled = false }: {
  page: "home" | "environment-hub" | "avatar-hub";
  navigate: (page: PageId) => void; amfInstalled?: boolean;
}) {
  const environment = useEnvironmentView();
  const task = useTaskCenter().tasks.find(item => isActiveStatus(item.status));
  const observed = (["play", "create"] as const).flatMap(zone => {
    const phase = environment.deployer.zones[zone];
    return phase.kind === "results" && phase.items.length > 0 ? [{ zone, phase }] : [];
  });
  const global = <nav className="vua-home__utilities" aria-label={presentationCopy.home.utilities}>
    <Button variant="subtle" onClick={() => navigate("settings-modules")} data-nav-id="home-modules"><Icon name="modules" size={20}/>{strings.amfModule.title}</Button>
    <Button variant="subtle" onClick={() => navigate("help")} data-nav-id="home-help"><Icon name="question" size={20}/>{copy.help}</Button>
    <Button variant="subtle" onClick={() => navigate("settings-theme")} data-nav-id="home-settings"><Icon name="settings" size={20}/>{strings.nav.tabs.settings}</Button>
  </nav>;
  return <div className="vua-page vua-home" data-focus-scope>
    <header className="vua-page__hero"><h1 className="vua-display">{page === "home" ? "VUA" : page === "environment-hub" ? copy.environment : copy.avatar}</h1></header>
    {page === "home" && task && <section className="vua-home__activity"><h2>{presentationCopy.home.continue}</h2><button type="button" onClick={() => navigate(taskRowOpenTarget(task))}><Icon name="arrow-right" size={20}/><strong>{task.title}</strong><span>{strings.taskStatus[task.status]}</span></button></section>}
    {page === "home" && observed.length > 0 && <section className="vua-home__activity"><h2>{presentationCopy.home.environment}</h2>{observed.map(({ zone, phase }) => <button type="button" key={zone} onClick={() => navigate(zone === "play" ? "env-play" : "env-create")}><Icon name="computer" size={20}/><strong>{zone === "play" ? copy.play : copy.create}</strong><time dateTime={phase.checkedAt}>{formatDateTime(phase.checkedAt)}</time><small>{phase.items.map(item => `${item.title}: ${item.description}`).join(" · ")}</small></button>)}</section>}
    <>
      {(["env", "production"] as const).filter(group => (group !== "production" || amfInstalled) && (page === "home" || (page === "environment-hub" ? group === "env" : group === "production"))).map(group => <section key={group} className="vua-home__section" data-module={group}>
        <h2>{group === "env" ? copy.environment : TERMS.amf}</h2><div className="vua-route-grid">
          {directory[group].map(item => <RouteTile key={item.id} title={item.title} icon={item.icon} onClick={() => navigate(item.id)} id={`home-${item.id}`} />)}
        </div>
      </section>)}
      {page === "home" ? global : null}
    </>
  </div>;
}
