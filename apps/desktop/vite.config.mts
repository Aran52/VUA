import path from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { buildInfo } from "./scripts/build-info.mjs";

const desktopRoot = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react(), {
    name: "vua-build-info",
    apply: "build",
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "build-info.json", source: JSON.stringify(buildInfo, null, 2) });
    },
  }],
  base: "./",
  // 构建期事实注入(Phase A):开屏/设置页显示真实版本与提交号,
  // 生产构建同路径(check:leak 的临时生产构建也走这里)
  define: { __VUA_BUILD_INFO__: JSON.stringify(buildInfo), __VUA_SPLIT_AMF__: "false" },
  // @vua/contracts 以 CJS dist 经 pnpm link 进入 workspace:vite 默认不预构建链接包,
  // 浏览器按 ESM 解析 CJS 时命名导出全部丢失(renderer 首帧崩溃、黑屏)。
  // 强制预构建做一次 CJS→ESM 互操作。production build 由 rolldown 处理,不受影响。
  // Workspace dist changes do not update the lockfile hash. Rebuild the dev cache
  // on startup so newly added contract exports cannot resolve to stale code.
  optimizeDeps: { entries: ["index.html", "splash.html"], include: ["@vua/contracts"], force: true },
  build: {
    outDir: "dist/renderer",
    emptyOutDir: true,
    chunkSizeWarningLimit: 550,
    rolldownOptions: {
      input: {
        main: path.join(desktopRoot, "index.html"),
        splash: path.join(desktopRoot, "splash.html"),
      },
      output: {
        codeSplitting: {
          groups: [
            { name: "three-core", test: /node_modules[\\/]three[\\/]build[\\/]/, priority: 30 },
            { name: "three-addons", test: /node_modules[\\/]three[\\/]examples[\\/]/, priority: 30 },
            { name: "react-vendor", test: /node_modules[\\/](?:react|react-dom|scheduler)[\\/]/, priority: 30 },
            { name: "vua-contracts", test: /packages[\\/]contracts[\\/]/, priority: 25 },
            { name: "vua-design-system", test: /packages[\\/]design-system[\\/]/, priority: 25 },
            { name: "i18n-copy-amf", test: /src[\\/]renderer[\\/]i18n[\\/](?:amf-|factory-|control-center-|release-room-|sdk-review-|library-|booth-|recipe-|avatar-optimizer-|texture-memory-|outfit-removal-|glb-inspection-|use-terms-)/, priority: 22, minSize: 0, maxSize: 350_000 },
            { name: "i18n-copy-diag", test: /src[\\/]renderer[\\/]i18n[\\/](?:local-diagnostics-|pose-graph-|help-|context-guide-|guide-channel-|environment-|scenario-|hardware-)/, priority: 22, minSize: 0, maxSize: 350_000 },
            { name: "i18n-copy-host", test: /src[\\/]renderer[\\/]i18n[\\/].*-copy\.ts$/, priority: 21, minSize: 0, maxSize: 350_000 },
            { name: "strings-en", test: /src[\\/]renderer[\\/]i18n[\\/]strings\.en\.ts$/, priority: 20, minSize: 0, maxSize: 350_000 },
            { name: "guide-and-help", test: /src[\\/]renderer[\\/]features[\\/](?:guide|help)[\\/]/, priority: 15 },
            { name: "warehouse-surface", test: /src[\\/]renderer[\\/]features[\\/]warehouse[\\/]/, priority: 15 },
            { name: "amf-workshop", test: /src[\\/]renderer[\\/]features[\\/]amf[\\/](?:AmfProduction|AvatarOptimizer|MaterialsStart|workshop)/, priority: 16 },
            { name: "amf-surface", test: /src[\\/]renderer[\\/]features[\\/](?:amf|packages|release)[\\/]/, priority: 15 },
          ],
        },
      },
    },
  },
  server: { host: "127.0.0.1", port: 5173, strictPort: true, watch: { ignored: ["**/out/**"] } },
});
