import { storageKeys } from "../../app/storage-keys.ts";
import { strings } from "../../i18n/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { GuideOverlayView, type GuideRequest } from "../overlay/GuideOverlayView.tsx";
import { loadGuideReading, type GuideTarget } from "../guide/guide-target.ts";
import "../overlay/overlay.css";

export function VuaGuide({ onBack, onReplay, guideRequest, acknowledgeGuide }: {
  onBack: () => void;
  onReplay: () => void;
  guideRequest: GuideRequest | null;
  acknowledgeGuide: (nonce: number) => void;
}) {
  const legacy = loadGuideReading();
  const initialPosition: GuideTarget = legacy?.topic === "guide-vua" ? legacy : { topic: "guide-vua" };
  return <div className="vua-page vua-knowledge vua-vua-guide" data-focus-scope>
    <header className="vua-page__hero vua-environment-heading">
      <h1 className="vua-title">{strings.journey.wizard}</h1>
      <Button variant="subtle" onClick={onReplay}>{strings.contextGuide.replay}</Button>
      <Button variant="subtle" data-back data-nav-id="help-child-back" onClick={onBack}>{strings.journey.backToHelp}</Button>
    </header>
    <div className="vua-overlay__body vua-knowledge__body">
      <GuideOverlayView
        embedded
        guideRequest={guideRequest}
        onGuideRequestApplied={acknowledgeGuide}
        topicIds={["guide-vua"]}
        initialTopic="guide-vua"
        initialPosition={initialPosition}
        readingKey={storageKeys.vuaGuideReading}
        historyEnabled={false}
      />
    </div>
  </div>;
}
