const en = {
  title: "GLB / VRM & Object-Head check",
  subtitle: "Header-only glTF 2.0 / VRM 1.0 inspection and PH-R Stage 1 MR Object-Head ≥ 5 cm clearance check.",
  formats: {
    vrm_1_0: "VRMC_vrm 1.0",
    vrm_0_x: "VRM 0.x",
    glb_2_0: "glTF 2.0 (.glb)",
    invalid_glb: "Invalid GLB header",
  },
  readiness: {
    ready_vrm1_humanoid: "VRM 1.0 humanoid ready",
    ready_object_head: "Object-Head ≥ 5 cm ready",
    vrm0_needs_upgrade: "VRM 0.x (upgrade to 1.0)",
    clearance_insufficient: "Object-Head < 5 cm margin",
    scale_suspect: "Check meter scale",
    missing_head_bone: "Missing humanoid head bone",
    invalid_glb: "Unreadable GLB",
  },
  clearance: {
    passes: "≥ 5 cm HMD clearance OK",
    insufficient_margin: "< 5 cm HMD margin",
    likely_unscaled: "Suspect non-meter scale",
    unknown: "Bounds unavailable",
  },
  dimensions: "{x} × {y} × {z} m",
  bonesLine: "{bones} bones",
  vrmMetaLine: "{title} (VRM {spec})",
  truncatedNote: "Showing the first inspected 3D binary artifacts.",
};

type GlbInspectionCopy = typeof en;

const zhCN: GlbInspectionCopy = {
  title: "GLB / VRM 与 Object-Head 规格检查",
  subtitle: "基于文件头的 glTF 2.0 / VRM 1.0 结构核验与 PH-R Stage 1 MR Object-Head ≥ 5 cm 头显净空检查。",
  formats: {
    vrm_1_0: "VRMC_vrm 1.0",
    vrm_0_x: "VRM 0.x",
    glb_2_0: "glTF 2.0 (.glb)",
    invalid_glb: "GLB 文件头无效",
  },
  readiness: {
    ready_vrm1_humanoid: "VRM 1.0 人形骨骼就绪",
    ready_object_head: "Object-Head ≥ 5 cm 就绪",
    vrm0_needs_upgrade: "VRM 0.x（建议迁移至 1.0）",
    clearance_insufficient: "Object-Head 净空不足 5 cm",
    scale_suspect: "比例疑似非米制单位",
    missing_head_bone: "缺少人形头部骨骼",
    invalid_glb: "无法解析的 GLB",
  },
  clearance: {
    passes: "符合 ≥ 5 cm 头显净空",
    insufficient_margin: "净空余量不足 5 cm",
    likely_unscaled: "尺寸比例异常",
    unknown: "包围盒未知",
  },
  dimensions: "{x} × {y} × {z} m",
  bonesLine: "{bones} 骨骼",
  vrmMetaLine: "{title}（VRM {spec}）",
  truncatedNote: "仅展示前序已检测的 3D 二进制工件。",
};

const zhTW: GlbInspectionCopy = {
  title: "GLB / VRM 與 Object-Head 規格檢查",
  subtitle: "基於檔頭的 glTF 2.0 / VRM 1.0 結構核驗與 PH-R Stage 1 MR Object-Head ≥ 5 cm 頭顯淨空檢查。",
  formats: {
    vrm_1_0: "VRMC_vrm 1.0",
    vrm_0_x: "VRM 0.x",
    glb_2_0: "glTF 2.0 (.glb)",
    invalid_glb: "GLB 檔頭無效",
  },
  readiness: {
    ready_vrm1_humanoid: "VRM 1.0 人形骨架就緒",
    ready_object_head: "Object-Head ≥ 5 cm 就緒",
    vrm0_needs_upgrade: "VRM 0.x（建議升級至 1.0）",
    clearance_insufficient: "Object-Head 淨空不足 5 cm",
    scale_suspect: "比例疑似非公尺單位",
    missing_head_bone: "缺少人形頭部骨骼",
    invalid_glb: "無法解析的 GLB",
  },
  clearance: {
    passes: "符合 ≥ 5 cm 頭顯淨空",
    insufficient_margin: "淨空餘量不足 5 cm",
    likely_unscaled: "尺寸比例異常",
    unknown: "包圍盒未知",
  },
  dimensions: "{x} × {y} × {z} m",
  bonesLine: "{bones} 骨骼",
  vrmMetaLine: "{title}（VRM {spec}）",
  truncatedNote: "僅顯示前序已檢查的 3D 二進位檔案。",
};

const ja: GlbInspectionCopy = {
  title: "GLB / VRM・Object-Head 仕様チェック",
  subtitle: "ヘッダー解析による glTF 2.0 / VRM 1.0 判定と、PH-R Stage 1 MR Object-Head（≥ 5 cm クリアランス）確認。",
  formats: {
    vrm_1_0: "VRMC_vrm 1.0",
    vrm_0_x: "VRM 0.x",
    glb_2_0: "glTF 2.0 (.glb)",
    invalid_glb: "無効な GLB ヘッダー",
  },
  readiness: {
    ready_vrm1_humanoid: "VRM 1.0 ヒューマノイド対応",
    ready_object_head: "Object-Head ≥ 5 cm 対応",
    vrm0_needs_upgrade: "VRM 0.x（1.0 への変換推奨）",
    clearance_insufficient: "クリアランス 5 cm 未満",
    scale_suspect: "スケール単位要確認",
    missing_head_bone: "Head ボーン未設定",
    invalid_glb: "解析不可 GLB",
  },
  clearance: {
    passes: "HMD クリアランス ≥ 5 cm OK",
    insufficient_margin: "HMD 余白 5 cm 未満",
    likely_unscaled: "非メートル単位の可能性",
    unknown: "境界サイズ不明",
  },
  dimensions: "{x} × {y} × {z} m",
  bonesLine: "{bones} ボーン",
  vrmMetaLine: "{title}（VRM {spec}）",
  truncatedNote: "先頭の 3D バイナリファイルのみ表示しています。",
};

const ko: GlbInspectionCopy = {
  title: "GLB / VRM 및 Object-Head 규격 점검",
  subtitle: "파일 헤더 기반 glTF 2.0 / VRM 1.0 판별 및 PH-R Stage 1 MR Object-Head(≥ 5 cm 여유 공간) 확인.",
  formats: {
    vrm_1_0: "VRMC_vrm 1.0",
    vrm_0_x: "VRM 0.x",
    glb_2_0: "glTF 2.0 (.glb)",
    invalid_glb: "유효하지 않은 GLB 헤더",
  },
  readiness: {
    ready_vrm1_humanoid: "VRM 1.0 휴머노이드 준비됨",
    ready_object_head: "Object-Head ≥ 5 cm 준비됨",
    vrm0_needs_upgrade: "VRM 0.x(1.0 업그레이드 권장)",
    clearance_insufficient: "여유 공간 5 cm 미만",
    scale_suspect: "미터 스케일 확인 필요",
    missing_head_bone: "Head 본 누락",
    invalid_glb: "읽을 수 없는 GLB",
  },
  clearance: {
    passes: "HMD 여유 공간 ≥ 5 cm 충족",
    insufficient_margin: "HMD 여유 공간 5 cm 미만",
    likely_unscaled: "비미터 스케일 의심",
    unknown: "바운딩 크기 미확인",
  },
  dimensions: "{x} × {y} × {z} m",
  bonesLine: "본 {bones}개",
  vrmMetaLine: "{title}(VRM {spec})",
  truncatedNote: "앞부분의 3D 바이너리 파일만 표시하고 있어요.",
};

export const glbInspectionCopy = { en, "zh-CN": zhCN, "zh-TW": zhTW, ja, ko };
