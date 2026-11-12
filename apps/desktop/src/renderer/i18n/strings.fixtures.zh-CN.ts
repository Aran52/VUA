/**
 * 演示数据负载文案(zh-CN)—— DEV 构建专用。
 *
 * 原则①硬防线的结构保证:本模块的应用内引用来自 gateway/fixture-*.ts(fixture-gateway
 * 及其拆分子模块)与 features/overlay/demo/(切片五 Overlay 演示端口)引用,
 * 两条引用链都仅在 import.meta.env.DEV 分支可达(gateway/create.ts 静态分支、
 * overlay-port-instance.ts 动态 import),
 * 生产构建中整条引用链被 Rollup 剔除,演示文案不会进入生产包。
 * scripts/fixtures 下的独立 Chromium 检查也可引用本模块,其入口不属于应用生产入口;
 * 检查专用文案同样参与生产包泄漏检查。
 * 真实数据源接入后,本文件随 fixture-gateway.ts 一并移除。
 *
 * 插值与术语纪律与主字符串表一致(见 strings.en.ts 文件头)。
 */
export const fixtureStrings = {
  /** Standalone Chromium smoke payload; its marker participates in the production leak check. */
  libraryMaintenanceProbeTitle: "Synthetic N5 library interaction test",
  checks: {
    vrchat: { title: "VRChat 客户端", okDescription: "已安装，版本验证通过。" },
    unity: { title: "Unity 版本", okDescription: "Unity 2022.3 LTS 已就绪。" },
    vpm: {
      title: "VPM 环境",
      okDescription: "VPM 环境已就绪，可管理角色项目依赖。",
      errorDescription: "未检测到 VPM 环境，无法管理角色项目依赖。",
      fixLabel: "安装 VPM 环境",
    },
    vrRuntime: {
      title: "VR 运行时与串流",
      okDescription: "SteamVR 正在运行，头显已连接。",
      warningDescription: "未检测到头显连接，仍可使用桌面模式游玩。",
      fixLabel: "查看连接指南",
    },
    disk: { title: "磁盘空间", okDescription: "剩余 128 GB，满足备份快照所需空间。" },
    network: { title: "网络连接", okDescription: "连接正常，可正常访问素材库与下载源。" },
  },
  fixPlans: {
    vpm: {
      title: "安装 VPM 环境",
      impact:
        "将在默认浏览器中打开 VCC 官方下载页面；下载与安装均由官方安装包完成，VUA 不会修改系统设置或代装软件。",
      identify: "未能自动识别 VPM 的安装状态，请确认你的实际情况：",
      identifyCandidates: ["尚未安装 VPM", "已经安装，但未被识别"],
      openDownload: "打开 VCC 官方下载页",
      openDownloadDescription: "在默认浏览器中打开 VRChat 官方创作者工具下载页。",
      runInstaller: "运行官方安装程序完成安装",
      runInstallerDescription: "下载完成后请按官方安装向导完成安装，VUA 不会自动代跑安装程序。",
      recheck: "重新检测改模环境",
      recheckDescription: "安装完成后重新检测，刷新当前环境状态。",
    },
    vrRuntime: {
      title: "排查 VR 运行时与串流连接",
      impact: "将在默认浏览器中打开官方连接排查指南；排查操作在你的设备上进行，VUA 不会改动任何系统设置。",
      openGuide: "打开 SteamVR 官方连接指南",
      openGuideDescription: "在默认浏览器中打开官方排查指南页面。",
      followGuide: "按指南检查头显连接与串流状态",
      followGuideDescription: "确认 SteamVR 已启动、头显连接稳固且串流画面正常。",
      recheck: "重新检测开玩环境",
      recheckDescription: "排查完成后重新检测，刷新当前环境状态。",
    },
  },
  recipe: {
    conflictAvatarBase: "当前包含两个基础素体（avatar_base），系统不会自动替你取舍，请手动确认保留哪一个。",
  },
  release: {
    projects: {
      summer: { title: "夏日制服角色模型", recipeTitle: "夏日制服穿搭方案" },
      casual: { title: "日常便服角色模型", recipeTitle: "日常便服穿搭方案" },
      legacy: { title: "旧版演出角色模型", recipeTitle: "舞台装穿搭方案" },
    },
  },
  workshop: {
    headline: "正在执行：{stage} · 骨骼绑定与换装菜单生成",
    logs: [
      "已创建并校验备份快照 r7",
      "素材“Summer_Uniform”导入完成",
      "骨骼绑定完成，共执行 42 项自动操作",
      "正在生成换装开关菜单与动画参数…",
    ],
    tapes: {
      success: {
        headline: "演示回放：一次完整的组装流程",
        snapshot: "已创建并校验备份快照 r7",
        imported: "素材“Summer_Uniform”导入完成",
        rigged: "骨骼绑定完成，共执行 42 项自动操作",
        menu: "换装开关菜单与参数生成完成",
        inspected: "检查通过：引用、功能与性能指标均正常",
      },
      warning: {
        headline: "演示回放：流程在确认节点暂停等待",
        imported: "素材“Summer_Uniform”导入完成",
        rigged: "骨骼绑定完成，共执行 18 项自动操作",
        checkpoint: "换装菜单合并方式需要你确认后才能继续",
      },
      blocked: {
        headline: "演示回放：缺少前置依赖导致流程中止",
        imported: "素材“Stage_Set”导入完成",
        missing: "缺少依赖：未在 VPM 中找到所需的着色器包，流程已中止",
      },
      recover: {
        headline: "演示回放：执行失败后从最近备份恢复",
        failed: "骨骼绑定失败：骨骼权重数据异常",
        restored: "已恢复备份快照 r6，回退到绑定前状态",
        retried: "重新执行骨骼绑定完成，共执行 42 项自动操作",
        inspected: "检查通过：引用、功能与性能指标均正常",
      },
    },
  },
  tools: {
    devices: [
      {
        id: "demo-tracker-adapter",
        name: "示例头显追踪桥接器",
        purpose: "将头显与手柄追踪数据转发给捕捉软件使用。",
        installed: true,
        dataDestination: "仅在本机处理，不上传网络。",
        maintainer: "VUA 演示目录",
        homepage: "https://example.invalid/tools/tracker-adapter",
        compatNote: "兼容情况未经验证，使用前请自行确认。",
      },
      {
        id: "demo-controller-map",
        name: "示例手柄按键映射工具",
        purpose: "自定义手柄按键触发角色表情的方式。",
        installed: false,
        dataDestination: "配置文件保存在本机。",
        maintainer: "社区示例",
        homepage: "https://example.invalid/tools/controller-map",
      },
    ],
    calibration: [
      {
        id: "demo-space-calibrator",
        name: "示例游玩空间校准工具",
        purpose: "校准游玩安全边界与全身追踪坐标系原点。",
        installed: true,
        dataDestination: "仅在本机处理，不上传网络。",
        maintainer: "VUA 演示目录",
        homepage: "https://example.invalid/tools/space-calibrator",
      },
    ],
    capture: [
      {
        id: "demo-motion-capture",
        name: "示例摄像头全身动捕工具",
        purpose: "将普通摄像头画面转换为全身追踪输入信号。",
        installed: false,
        dataDestination: "画面仅在本机实时计算，不上传任何视频。",
        maintainer: "社区示例",
        compatNote: "兼容情况未经验证，使用前请自行确认。",
      },
      {
        id: "demo-face-capture",
        name: "示例面捕参数映射器",
        purpose: "调整面部捕捉灵敏度并映射到角色表情。",
        installed: false,
        dataDestination: "配置文件保存在本机。",
        maintainer: "社区示例",
      },
    ],
  },
  versions: {
    vrchat: "VRChat 客户端",
    steamvr: "SteamVR 运行时",
    unity: "Unity 编辑器",
    vrcsdk: "VRChat SDK",
  },
  acquire: {
    entries: {
      summerUniform: {
        displayName: "夏日制服 ver1.2",
        folderName: "Summer_Uniform_v1.2",
        original: {
          relativePath: "Summer_Uniform_v1.2.unitypackage",
          suggestedFileName: "Summer_Uniform_v1.2.unitypackage",
        },
      },
      mikoDress: {
        displayName: "巫女服套装",
        folderName: "Miko_Dress_set",
        original: {
          relativePath: "orig/Miko_Dress_set_v1.0.unitypackage",
          suggestedFileName: "Miko_Dress_set_v1.0.unitypackage",
        },
        generatedVpm: {
          relativePath: "generated/Miko_Dress_set_v1.0.vpm",
          suggestedFileName: "Miko_Dress_set_v1.0.vpm",
        },
      },
      stageSet: {
        displayName: "舞台道具包",
        folderName: "Stage_Set_pack",
        original: {
          relativePath: "Stage_Set_pack_v0.3.unitypackage",
          suggestedFileName: "Stage_Set_pack_v0.3.unitypackage",
        },
      },
      suspicious: {
        displayName: "免费模型合集包",
        folderName: "free_avatar_bundle",
        original: {
          relativePath: "pack/free_avatar_bundle.zip",
          rejectionReason: "检测到可执行程序：setup.exe、scripts/install.bat",
        },
      },
    },
  },
  packages: {
    projects: {
      summer: { name: "夏日制服角色项目", path: "~/Unity/Projects/SummerAvatar" },
      stage: { name: "舞台演出角色项目", path: "~/Unity/Projects/StageLive" },
      lost: { name: "旧存档项目", path: "~/Unity/Projects/OldArchive" },
      imported: { name: "手动添加的项目", path: "~/Unity/Projects/ImportedProject" },
    },
    rows: {
      avatarsSdk: {
        displayName: "VRChat SDK - Avatars",
        description: "VRChat 官方角色模型构建与上传核心包。",
      },
      modularCloset: {
        displayName: "示例模块化衣柜",
        description: "示例非破坏式穿衣组件与开关菜单生成工具。",
      },
      facefx: {
        displayName: "示例表情扩展包",
        description: "示例面捕参数与表情动画映射预设集。",
      },
      toonShader: {
        displayName: "示例卡通着色器",
        description: "示例二次元风格卡通渲染着色器包。",
      },
      legacyProps: {
        displayName: "示例旧版手持道具集",
        description: "示例采用旧版 Assets 目录结构的手持道具包。",
      },
      physbonePlus: {
        displayName: "示例动骨参数预设包",
        description: "示例常用头发与裙摆动骨（PhysBone）手感预设。",
      },
      localTail: {
        displayName: "本地导入：示例尾巴动骨包",
        description: "从本地文件夹直接导入的示例包，无在线软件源。",
      },
      stageFx: {
        displayName: "示例舞台粒子特效包",
        description: "示例演出粒子合集；旧版本与当前项目不兼容。",
      },
      localImport: {
        displayName: "本地导入：示例发饰动骨包",
        description: "演示手动从本地导入的扩展包。",
      },
    },
    repos: {
      official: { name: "VRChat 官方软件源" },
      curated: { name: "VUA 精选软件源" },
      communityA: { name: "示例社区软件源 A", url: "https://example.invalid/repos/community-a" },
      communityB: { name: "示例社区软件源 B", url: "https://example.invalid/repos/community-b" },
    },
    legacyDirs: {
      modularCloset: "Assets/ModularCloset (legacy)",
    },
  },
  production: {
    materials: {
      unitypackageDirect: "Summer_Uniform_v1.2.unitypackage",
      localVpm: "summer-uniform-local-1.0.0（本地 VPM 包）",
    },
    headlines: {
      active: "正在执行：{stage} · 服装导入与菜单生成",
      completed: "组装已完成：{stage} · 所有步骤均已通过",
      cancelled: "已取消：{stage} · 已在安全节点停止",
      failed: "执行未完成：{stage} · 可从备份快照还原",
      expired: "确认已过期：{stage} · 未对项目做任何改动",
    },
    inspection: {
      findings: {
        compat: "服装“Summer_Uniform”标注支持当前素体（演示数据）。",
        missing: "未在本地找到可选配饰贴图“Summer_Hat”（不影响继续构建）。",
        conflict: "换装开关与项目内已有参数“OutfitToggle”重名。",
      },
    },
    plan: {
      risks: [
        {
          kind: "conflict",
          summary: "合并换装菜单时会调整同名参数，执行前会自动创建还原快照。",
          recoverable: true,
          retryable: true,
        },
        {
          kind: "missing",
          summary: "预计新增占用磁盘空间约 300 MB。",
          recoverable: true,
          retryable: true,
        },
      ],
      diffs: {
        added: "为缺失的可选配饰贴图创建占位引用，避免打断构建。",
        resolved: "将重名参数自动改名为“OutfitToggle_v2”以避免冲突。",
      },
    },
    recordEvidence: {
      completed: "Unity 桥接任务 bridge-job-demo-0142 已完成（演示）。",
      rolledBack: "Unity 桥接任务 bridge-job-demo-0142 已回退还原（演示）。",
      rollbackFailed: "Unity 桥接任务 bridge-job-demo-0142 回退中断（演示）。",
    },
    tasks: {
      inspect: "检查素材与目标项目",
      plan: "生成组装计划",
      execute: "执行模型组装",
      recover: "还原项目状态",
    },
    transitionLog: {
      inspectStarted: "开始检查素材与目标项目。",
      inspectDone: "检查完成：1 项素体匹配说明、1 项可选贴图缺失、1 项参数重名。",
      planStarted: "正在生成组装计划…",
      planDone: "组装计划已生成，等待确认。",
      snapshotStarted: "正在创建执行前备份快照…",
      executeStarted: "正在导入素材并生成换装菜单…",
      validateStarted: "正在核对资源引用与动画参数…",
      completed: "全部步骤已完成，已生成构建记录。",
      recoverStarted: "正在启动还原任务。",
      continueRerun: "从最近的安全节点继续完成剩余步骤。",
      expiredBack: "已返回计划确认页，请核对最新内容。",
      rollbackDone: "已还原至执行前的备份快照。",
      rollbackFailed: "还原失败：备份快照校验值不一致，请手动检查项目。",
      drifted: "检测到执行结果与预期不一致，可还原至执行前状态。",
      expired: "计划内容已更新，此前的确认已失效，项目未被改动。",
      cancelled: "已取消：已在安全节点停止，未留下半成品。",
    },
  },
} as const;

export type FixtureStrings = typeof fixtureStrings;
