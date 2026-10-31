import { PlayPage } from "./PlayPage.tsx";
import { CreatorPage } from "./CreatorPage.tsx";
/** Fixed feature routes are independent of wizard choices. */
export function RouteEnvironmentPage({ zone, onAccounts, onOpenAmf, onWorkshop }: { zone: "play" | "create"; onAccounts: () => void; onOpenAmf: () => void; onWorkshop?: () => void }) {
  return zone === "play" ? <PlayPage onAccounts={onAccounts} /> : <CreatorPage onOpenAmf={onOpenAmf} onWorkshop={onWorkshop} />;
}
