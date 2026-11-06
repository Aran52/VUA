import { currentLocale } from "./current-table.ts";
const en = { read: "Read recorded terms",
  reading: "Reading terms…", done: "Reading finished; use decisions have been refreshed.", stopped: "Some terms could not be read. Open a product's details to see why and read again.", failed: "Could not read terms. Please try again." };
const tables: Record<string, typeof en> = {
  en,
  "zh-CN": { read: "读取已记录的条款", reading: "正在读取条款…", done: "读取完成，已刷新用途筛选结果。", stopped: "部分条款未能读取，可打开商品详情查看原因并重新读取。", failed: "条款读取未能开始，请重试。" },
  "zh-TW": { read: "讀取已記錄的條款", reading: "正在讀取條款…", done: "讀取完成，已重新整理用途篩選結果。", stopped: "部分條款未能讀取，可開啟商品詳情查看原因並重新讀取。", failed: "條款讀取未能開始，請重試。" },
  ja: { read: "記録済みの規約を読む", reading: "規約を読み取り中…", done: "読み取りが終わり、用途の判定を更新しました。", stopped: "一部の規約を読めませんでした。商品詳細で理由を確認し、再試行できます。", failed: "規約の読み取りを開始できませんでした。再試行してください。" },
  ko: { read: "기록된 약관 읽기", reading: "약관 읽는 중…", done: "읽기가 끝나 용도 판정을 새로 고쳤어요.", stopped: "일부 약관을 읽지 못했어요. 상품 상세에서 이유를 확인하고 다시 읽을 수 있어요.", failed: "약관 읽기를 시작하지 못했어요. 다시 시도해 주세요." },
};
export const libraryTermsCopy = tables[currentLocale] ?? en;
