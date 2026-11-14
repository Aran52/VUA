import { useEffect, useState } from "react";
import type { GlbInspectionResultV01, PhrReadinessV01 } from "@vua/contracts";
import { Icon } from "@vua/design-system";
import { Badge } from "../../components/primitives/Badge.tsx";
import { useGateway } from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";

const copy = strings.glbInspection;

function formatMeters(value: number): string {
  return value.toFixed(2);
}

function readinessTone(readiness: PhrReadinessV01): "success" | "warning" | "neutral" {
  if (readiness === "ready_vrm1_humanoid" || readiness === "ready_object_head") return "success";
  if (readiness === "invalid_glb") return "warning";
  return "neutral";
}

export function GlbInspectionSection({
  target,
  revision = 0,
}: {
  readonly target: { readonly productId: string } | { readonly entryId: string };
  readonly revision?: number;
}) {
  const port = useGateway().libraryBrowser;
  const [result, setResult] = useState<GlbInspectionResultV01 | null>(null);
  const key = "productId" in target ? `p:${target.productId}` : `e:${target.entryId}`;

  useEffect(() => {
    setResult(null);
    if (!port?.inspectGlb) return;
    let active = true;
    void port.inspectGlb(target).then(
      (next) => {
        if (active) setResult(next);
      },
      () => {
        if (active) setResult(null);
      },
    );
    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- target is keyed by stable productId/entryId
  }, [port, key, revision]);

  if (!port?.inspectGlb || result === null || result.items.length === 0) {
    return null;
  }

  return (
    <section className="vua-warehouse-detail__section vua-glb-inspection" aria-label={copy.title}>
      <div className="vua-warehouse-detail__section-head">
        <h3>{copy.title}</h3>
      </div>
      <p className="vua-caption vua-text-secondary">{copy.subtitle}</p>
      <ul className="vua-glb-inspection__list">
        {result.items.map((item) => (
          <li key={`${item.copyId}:${item.fileName}`} className="vua-glb-inspection__item">
            <div className="vua-glb-inspection__row">
              <span className="vua-glb-inspection__name" title={item.fileName}>
                <Icon name="file" size={16} />
                <span>{item.fileName}</span>
              </span>
              <span className="vua-glb-inspection__badges">
                <Badge tone={item.format === "vrm_1_0" ? "success" : item.format === "vrm_0_x" ? "warning" : "neutral"}>
                  {copy.formats[item.format]}
                </Badge>
                <Badge tone={readinessTone(item.phrReadiness)}>
                  {copy.readiness[item.phrReadiness]}
                </Badge>
              </span>
            </div>
            {item.format !== "invalid_glb" ? (
              <div className="vua-glb-inspection__meta">
                {item.vrm?.modelName ? (
                  <span>
                    {format(copy.vrmMetaLine, {
                      title: item.vrm.modelName,
                      spec: item.vrm.specVersion ?? "1.0",
                    })}
                  </span>
                ) : null}
                {item.boundsMeters ? (
                  <span>
                    {format(copy.dimensions, {
                      x: formatMeters(item.boundsMeters.size[0]),
                      y: formatMeters(item.boundsMeters.size[1]),
                      z: formatMeters(item.boundsMeters.size[2]),
                    })}
                  </span>
                ) : null}
                {item.objectHeadClearance.status !== "unknown" ? (
                  <span>{copy.clearance[item.objectHeadClearance.status]}</span>
                ) : null}
                {item.vrm ? (
                  <span>{format(copy.bonesLine, { bones: item.vrm.boneCount })}</span>
                ) : null}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
      {result.truncated ? <p className="vua-caption vua-text-secondary">{copy.truncatedNote}</p> : null}
    </section>
  );
}
