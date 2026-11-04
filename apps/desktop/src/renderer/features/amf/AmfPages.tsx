/** Optional AMF frontend entry. Loaded only after the module is ready. */
import type { PageId } from "../../app/nav-model.ts";
import { WarehousePage } from "../warehouse/WarehousePage.tsx";
import { ControlRecipesPage } from "./ControlRecipesPage.tsx";
import { AmfProductionPage } from "./AmfProductionPage.tsx";
import { ReleaseAlbumPage } from "./release/ReleaseAlbumPage.tsx";
import { ControlPackagesPage } from "./ControlPackagesPage.tsx";
import { AssetBrowserPage } from "./AssetBrowserPage.tsx";
import { StationList } from "./StationList.tsx";
import { AmfRoom } from "./AmfRoom.tsx";
import { ReleasePage } from "../release/ReleasePage.tsx";
import { strings } from "../../i18n/index.ts";
import { ShapeScope } from "../../components/primitives/ShapeScope.tsx";
export default function AmfPages({ page, navigate, prepareEnv }: {
  page: PageId; creatorReady: boolean; navigate: (page: PageId) => void; prepareEnv: () => void;
}) {
  if (page === "avatar-hub") return <ShapeScope value="amf"><StationList navigate={navigate} /></ShapeScope>;
  // Settings → Debugging information: the factory's production records.
  if (page === "settings-debug") return <ReleasePage onNavigate={navigate} heading={strings.nav.pages.settingsDebug} />;
  return <ShapeScope value="amf"><AmfRoom page={page} navigate={navigate}><RoomPage page={page} navigate={navigate} prepareEnv={prepareEnv} /></AmfRoom></ShapeScope>;
}

function RoomPage({ page, navigate, prepareEnv }: { page: PageId; navigate: (page: PageId) => void; prepareEnv: () => void }) {
  switch (page) {
    case "asset-browser": return <AssetBrowserPage onNavigate={navigate} />;
    case "warehouse": return <WarehousePage onNavigate={navigate} />;
    case "recipe": return <ControlRecipesPage onNavigate={navigate} />;
    case "workshop": return <AmfProductionPage onPrepareEnv={prepareEnv} onNavigate={navigate} />;
    case "release": return <ReleaseAlbumPage onNavigate={navigate} />;
    case "packages": return <ControlPackagesPage onNavigate={navigate} />;
    default: return null;
  }
}
