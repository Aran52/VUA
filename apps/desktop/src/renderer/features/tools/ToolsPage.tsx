import { useState } from "react";
import { openExternalUrl } from "../../app/open-external.ts";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { EmptyState } from "../../components/primitives/EmptyState.tsx";
import { strings } from "../../i18n/index.ts";
import {
  useDataSource,
  useToolCatalogView,
  type ToolCard,
  type ToolCategory,
} from "../../gateway/index.ts";
import "./tools.css";

const copy = strings.tools;

export type ToolsPageId =
  | "tools-discover"
  | "tools-devices"
  | "tools-calibration"
  | "tools-installed";

const pageCopyKey = {
  "tools-discover": "discover",
  "tools-devices": "devices",
  "tools-calibration": "calibration",
  "tools-installed": "installed",
} as const satisfies Record<ToolsPageId, keyof typeof copy.pages>;

/** 各页面的用途分组(§8.1:按用途分组);installed 页跨分组只列已装 */
const pageGroups: Record<ToolsPageId, readonly ToolCategory[]> = {
  "tools-discover": ["devices", "calibration", "capture"],
  "tools-devices": ["devices"],
  "tools-calibration": ["calibration"],
  "tools-installed": ["devices", "calibration", "capture"],
};

/**
 * 工具合集(v0.4.0 §8;C-TOOLS):能力目录,不暗示对第三方工具的安全保证。
 * - 卡片四要素常驻:解决什么问题 / 是否已安装 / 数据去向 / 维护者;
 * - 唯一动作是"打开官网"(系统浏览器),影响范围在按钮旁常驻明示;
 *   应用内启动与配置导入未接入,不渲染死按钮;
 * - 加载/失败由 GatewayProvider 全局态承载;未接入为诚实空态;
 * - 目录为空或分组为空时如实说明,不填充占位卡片。
 */

function ToolCardView({
  tool,
  openFailed,
  onOpenHomepage,
}: {
  tool: ToolCard;
  openFailed: boolean;
  onOpenHomepage: (tool: ToolCard) => void;
}) {
  const fields: Array<{ label: string; value: string }> = [
    { label: copy.fields.purpose, value: tool.purpose },
    { label: copy.fields.dataDestination, value: tool.dataDestination },
    { label: copy.fields.maintainer, value: tool.maintainer },
  ];
  return (
    <div className="vua-tool-card">
      <div className="vua-tool-card__header">
        <span className="vua-tool-card__name">{tool.name}</span>
        <Badge tone="neutral">
          {tool.installed ? copy.installedYes : copy.installedNo}
        </Badge>
      </div>
      <dl className="vua-tool-card__fields">
        {fields.map((field) => (
          <div key={field.label} className="vua-tool-card__field">
            <dt className="vua-caption vua-text-secondary">{field.label}</dt>
            <dd>{field.value}</dd>
          </div>
        ))}
      </dl>
      {tool.compatNote ? (
        <p className="vua-caption vua-text-secondary">{tool.compatNote}</p>
      ) : null}
      {tool.homepage ? (
        <div className="vua-tool-card__actions">
          {openFailed ? (
            <p className="vua-caption vua-text-secondary">{copy.openHomepageFailed}</p>
          ) : null}
          <div>
            <Button variant="default" onClick={() => onOpenHomepage(tool)}>
              {copy.openHomepage}
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

export function ToolsPage({ page }: { page: ToolsPageId }) {
  const pageCopy = copy.pages[pageCopyKey[page]];
  const view = useToolCatalogView();
  const isFixture = useDataSource() === "fixture";
  const [openFailedId, setOpenFailedId] = useState<string | null>(null);

  const openHomepage = (tool: ToolCard) => {
    if (!tool.homepage) return;
    setOpenFailedId(null);
    void openExternalUrl(tool.homepage).then((ok) => {
      if (!ok) setOpenFailedId(tool.id);
    });
  };

  const groups = pageGroups[page]
    .map((category) => {
      const tools =
        view.kind === "catalog"
          ? view.tools.filter(
              (tool) =>
                tool.category === category && (page !== "tools-installed" || tool.installed),
            )
          : [];
      return { category, tools };
    })
    .filter((group) => group.tools.length > 0);

  return (
    <div className="vua-page">
      <section className="vua-page__hero">
        <h1 className="vua-title">
          {pageCopy.title}
          {isFixture && view.kind === "catalog" ? (
            <>
              {" "}
              <Badge tone="warning">{strings.common.fixtureBadge}</Badge>
            </>
          ) : null}
        </h1>
      </section>
      {view.kind !== "catalog" ? (
        <EmptyState title={copy.notConnectedTitle} />
      ) : groups.length === 0 ? (
        <Card>
          <EmptyState
            title={pageCopy.title}
            description={
              page === "tools-installed" ? copy.installedEmpty : copy.groupEmpty
            }
          />
        </Card>
      ) : (
        groups.map((group) => (
          <Card key={group.category}>
            <div className="vua-page__stack">
              <h2 className="vua-caption vua-text-secondary">{copy.groups[group.category]}</h2>
              <div className="vua-tool-grid">
                {group.tools.map((tool) => (
                  <ToolCardView
                    key={tool.id}
                    tool={tool}
                    openFailed={openFailedId === tool.id}
                    onOpenHomepage={openHomepage}
                  />
                ))}
              </div>
            </div>
          </Card>
        ))
      )}
    </div>
  );
}
