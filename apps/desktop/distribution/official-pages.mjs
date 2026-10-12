const escape = value => String(value).replace(/[&<>"']/g, character => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]);
const size = bytes => `${(bytes / 1_000_000).toFixed(1)} MB`;

export const officialStyles = `
.vua-section{padding-block:32px;border-top:1px solid #d5d7d7}.vua-section h2{font-size:clamp(24px,3vw,36px);margin:0 0 20px}.vua-section h3{margin:0 0 10px}.vua-section p,.vua-section li{line-height:1.8}.vua-columns{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:28px}.vua-panel{padding:24px;border:1px solid #d5d7d7;border-radius:16px;min-width:0}.vua-links{display:flex;flex-wrap:wrap;gap:12px;margin:20px 0}.vua-download{display:inline-block;padding:13px 20px;border-radius:8px;background:#23272d;color:#fff;text-decoration:none;font-weight:650}.vua-download-secondary{background:transparent;border:1px solid #555;color:inherit}.vua-meta{font-size:14px;color:#5a6268}.vua-section ol{padding-left:24px}.vua-section code{overflow-wrap:anywhere}.vua-sha{font-size:12px;word-break:break-all}.vua-section details{margin-top:20px}.vua-section summary{cursor:pointer}.vua-section a:focus-visible{outline:3px solid #7556cc;outline-offset:4px}@media(max-width:650px){.vua-columns{grid-template-columns:1fr}.vua-panel{padding:18px}}
`;

function page(title, content) {
  return `<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="description" content="VUA：面向 VRChat 玩家的 Windows 本地情景安装、管理与启动器。下载 Ibis Beta，了解游玩、设备准备、素材管理和更新。"><meta name="theme-color" content="#f3f3ef"><title>${escape(title)} · PH-R</title><link rel="icon" type="image/svg+xml" href="/portal-assets/mark.svg"><link rel="preload" href="/portal-assets/fonts/phr-display.woff2" as="font" type="font/woff2" crossorigin><link rel="stylesheet" href="/portal-assets/skeleton.css"><link rel="stylesheet" href="/vua/downloads/style.css"></head>
<body id="top"><a class="skip" href="#main">跳到正文</a><header class="header"><a class="brand" href="/" aria-label="PH-R 首页"><img class="site-logo" src="/portal-assets/ph-r-light.svg" width="256" height="256" alt="PH-R"></a><span class="brand-caption">Partial · Hybrid — Reality</span><nav aria-label="主导航"><a href="/#projects" aria-current="true">项目</a><a href="/updates/">记录</a><a href="/about/">关于</a></nav></header>
<main id="main">${content}</main><footer class="footer"><a class="brand" href="/" aria-label="PH-R 首页"><img class="site-logo" src="/portal-assets/ph-r-light.svg" width="256" height="256" alt="PH-R"></a><a class="icp" href="https://beian.miit.gov.cn/portal/#/Integrated/index" rel="external">浙ICP备2026001369号</a><a class="back-top" href="#top">回到顶部 ↑</a></footer></body></html>\n`;
}

function downloads(release) {
  const base = `/vua/releases/v${release.version}/`;
  const online = release.assets.find(asset => asset.name.endsWith("-setup.exe"));
  const offline = release.assets.find(asset => asset.name.endsWith("-offline.exe"));
  return `<section class="vua-section wrap" id="download"><h2>下载 VUA ${escape(release.version)} · Ibis Beta</h2><p>适用于 Windows 10 / 11，64 位 x64。AMF 随 VUA 安装，对新用户默认启用，也可以在设置中停用。无需先安装 Node.js、Rust 或开发工具。</p>
<div class="vua-columns"><article class="vua-panel"><h3>在线安装器</h3><p>首次安装时自动选择可达的 Electron 运行时来源，并核验文件。已经装有匹配的 VUA 运行时，升级时会继续复用。</p><a class="vua-download" href="${base}${escape(online.name)}">下载在线安装器 · ${size(online.bytes)}</a></article>
<article class="vua-panel"><h3>离线安装器</h3><p>包含完整运行时。适合网络不稳定，或提前带到另一台电脑安装；游玩和外部软件获取仍需要相应网络。</p><a class="vua-download vua-download-secondary" href="${base}${escape(offline.name)}">下载离线安装器 · ${size(offline.bytes)}</a></article></div>
<p class="vua-meta">当前安装包尚无 Windows 发布者签名，系统可能显示陌生发布者提示。官网与 GitHub 提供相同的原始文件。</p><div class="vua-links"><a href="https://github.com/VUA-Project/VUA/releases/tag/v${escape(release.version)}">GitHub 发布说明与备用下载 ↗</a><a href="${base}SHA256SUMS.txt">查看 SHA-256 校验值</a></div>
<details><summary>版本与文件校验信息</summary><p class="vua-meta">宿主 ${escape(release.version)} · AMF ${escape(release.amfVersion)} · 源码构建 <code>${escape(release.build)}</code></p><p class="vua-sha">在线安装器 SHA-256：<code>${online.sha256}</code></p><p class="vua-sha">离线安装器 SHA-256：<code>${offline.sha256}</code></p></details></section>`;
}

function usage() {
  return `<section class="vua-section wrap"><h2>第一次使用</h2><ol><li>下载并运行安装器，完成后从开始菜单打开 VUA。</li><li>跟随首次功能导览和入门引导，选择这次想做的事；也可以直接进入「游玩」「工具」或「改模」。</li><li>检查所需软件，已有安装可以复用。Steam 登录、游戏库安装、头显配对与账号关联在官方软件中完成。</li><li>需要改模时使用 AMF 素材库，导入本地文件夹，或登录自己的 BOOTH 账号后同步商品、选择下载。</li></ol><p>遇到不熟悉的设备或操作，可以从「帮助」打开知识百科、VRChat 官方 Wiki 和游戏内助手。</p></section>`;
}

function updates(release) {
  return `<section class="vua-section wrap"><h2>安装、更新与数据</h2><div class="vua-columns"><article><h3>更新 VUA</h3><p>完成手头工作并退出 VUA，再运行新版安装器。相同且通过校验的运行时会复用；已有 ZIP 用户也可以改用安装器，保留原来的应用资料。</p></article><article><h3>AMF 可以单独更新</h3><p>「设置 → 模块」或「关于」提供 AMF 的更新检查、下载与重启应用。更新复用相同文件，只获取变化或缺失的部分；服务器不支持分段下载时会提示下载完整模块。</p></article></div><p>素材、项目、账号会话和个人设置保存在本机。停用 AMF 或卸载程序会保留应用资料；外部软件的购买、更新和卸载由对应平台负责。</p>${release.version === "0.6.7" ? '<p class="vua-meta">v0.6.7 内置更新检查仍使用 GitHub。官网已提供同版本下载；官网备用更新源将在后续客户端构建中启用。</p>' : ''}</section>`;
}

export function projectPage(release) {
  return page("VUA", `<section class="detail-hero wrap"><a class="breadcrumb" href="/#projects">← 全部项目</a><div class="detail-heading"><div><p class="eyebrow blue">VRC ULTRA ASSISTANT</p><h1>VUA</h1><p class="project-meta">Windows · 本地优先 · Ibis Beta ${escape(release.version)}</p></div><img class="project-icon vua-logo" src="/portal-assets/vua-level-light.svg" width="248" height="248" alt=""></div><p class="project-intro">围绕你想做的事，准备软件、梳理步骤、管理素材，再把对应的工具启动起来。</p><p>用电脑屏幕游玩、用 PICO 串流、用 AMF 整理素材和改模，都是一套「情景」。VUA 把所需软件与操作顺序放在一起，让下一步更容易找到。</p><div class="vua-links"><a class="vua-download" href="#download">下载 Ibis Beta</a><a class="vua-download vua-download-secondary" href="https://github.com/VUA-Project/VUA" target="_blank" rel="noopener noreferrer">查看源码 ↗</a></div></section>
<section class="vua-section wrap"><h2>现在可以做什么</h2><div class="vua-columns"><article class="vua-panel"><h3>游玩：准备好，再进入游戏</h3><p>检查 Steam、VRChat、SteamVR 与 PICO Connect，复用已有软件，按电脑屏幕或 PICO 情景准备和启动。区域参考延迟帮助判断网络状况；它是近似参考，不是实际 VRChat 房间的 Ping。</p></article><article class="vua-panel"><h3>工具：连接你使用的软件</h3><p>集中查看和启动外部工具。VRCFaceTracking 引导按设备选择上游模块，再完成串流、OSC 与校准；可用能力由设备和上游软件决定。页面会明确标出尚在开发的入口。</p></article><article class="vua-panel"><h3>AMF：管理自己的改模素材</h3><p>使用本地导入与资料库迁移预览，或同步自己 BOOTH 账号的商品列表，选择下载、暂停、重新下载和管理本地文件。素材可按分类筛选，查看详情、原商品页和描述中的适配线索。</p></article><article class="vua-panel"><h3>改模：环境与模块分开管理</h3><p>检测 Unity 编辑器、Hub、VCC / ALCOM 等环境。第一方 AMF 与其素材资料随 VUA 安装，可以单独停用或更新。Recipe、装配、检查和成品交接仍在迭代，复杂真实素材的完整验收尚未完成。</p></article><article class="vua-panel"><h3>帮助：从入门到游戏里</h3><p>首次功能导览介绍界面，入门引导整理目标，知识百科解释设备和准备步骤。游戏内助手可以跟随 VRChat 窗口，保留阅读进度、透明度和拖动位置。</p></article><article class="vua-panel"><h3>内嵌浏览器与四语言界面</h3><p>从素材浏览打开 BOOTH 和多个素材发现站点，支持多标签与保留浏览状态。界面提供中文、English、日本語和한국어，支持深浅色与资源节约模式。</p></article></div></section>
<section class="vua-section wrap"><h2>AMF 正在开发的方向</h2><div class="vua-columns"><article class="vua-panel"><p class="eyebrow">开发中</p><h3>自动改模</h3><p>从你选择的 Avatar 和素材出发，把工程准备、依赖安装、装配与检查串成一条可以查看结果、处理问题和恢复的流程。目标是减少重复操作，让你把时间用在搭配和效果确认上。</p><p>AMF 的复杂真实素材流程仍在开发与验收；自动化不保证最终外观和行为，发布前需要你确认，并在官方 VRChat SDK 中完成登录与上传。</p></article><article class="vua-panel"><p class="eyebrow">开发中</p><h3>Recipe 配方</h3><p>记录素材引用、版本、设置和装配选择，保存自己的搭配，也能把配方分享给其他玩家，让同一套修改更容易再次复现。</p><p>配方分享引用与设置，不打包付费素材。接收者自行取得所需素材；保存、共享、复现及缺失素材处理的完整路径仍在开发。</p></article></div></section>
${downloads(release)}${usage()}${updates(release)}
<section class="vua-section wrap"><h2>发布范围与反馈</h2><p>当前为持续开发的 Beta。已发布的功能与仍在开发的入口分开显示；发布并不代表所有 Windows 环境、PICO 实物线路、VRCFT 设备和真实改模素材都已完成验收。具体已做检查和限制见对应版本的发布说明。</p><p>VUA 是独立第三方工具，与 VRChat Inc. 没有隶属或背书关系。素材由你自行取得，登录、账号关联和最终 Avatar 上传由你在官方流程中完成。</p><p>VUA 源码使用 Apache-2.0 许可；外部软件保留各自的许可和购买条件。</p><div class="vua-links"><a href="https://github.com/VUA-Project/VUA/issues/new/choose">报告问题 / 提出建议 ↗</a><a href="https://github.com/VUA-Project/VUA/blob/main/THIRD_PARTY_NOTICES.md">第三方许可说明 ↗</a></div></section>`);
}

export function downloadPage(release) {
  return page(`VUA ${release.version} 下载`, `<section class="detail-hero wrap"><a class="breadcrumb" href="/projects/vua/">← 了解 VUA</a><p class="eyebrow blue">VRC ULTRA ASSISTANT</p><h1>VUA · Ibis Beta</h1><p class="project-intro">下载、安装，然后选择你想做的事。</p></section>${downloads(release)}${usage()}${updates(release)}`);
}
