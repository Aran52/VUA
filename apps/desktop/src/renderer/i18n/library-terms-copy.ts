import { currentLocale } from "./current-table.ts";
const en = { read: "Read recorded terms", hint: "Use decisions need recorded terms. Unknown does not mean allowed. Read a product's details and terms to fill gaps.",
  reading: "Reading terms…", done: "Reading finished; use decisions have been refreshed.", stopped: "Some terms could not be read. Open a product's details to see why and read again.", failed: "Could not read terms. Please try again." };
const tables: Record<string, typeof en> = {
  en,
  "zh-CN": { read: "读取已记录的条款", hint: "用途筛选依据已读取的条款，未知不代表允许。缺少资料时，可打开商品详情读取商品信息和条款。", reading: "正在读取条款…", done: "读取完成，已刷新用途筛选结果。", stopped: "部分条款未能读取，可打开商品详情查看原因并重新读取。", failed: "条款读取未能开始，请重试。" },
  "zh-TW": { read: "讀取已記錄的條款", hint: "用途篩選依據已讀取的條款，未知不代表允許。缺少資料時，可開啟商品詳情讀取商品資訊和條款。", reading: "正在讀取條款…", done: "讀取完成，已重新整理用途篩選結果。", stopped: "部分條款未能讀取，可開啟商品詳情查看原因並重新讀取。", failed: "條款讀取未能開始，請重試。" },
  ja: { read: "記録済みの規約を読む", hint: "用途の判定には読み取った規約が必要です。不明は許可を意味しません。情報が足りない商品は詳細から商品情報と規約を読み取れます。", reading: "規約を読み取り中…", done: "読み取りが終わり、用途の判定を更新しました。", stopped: "一部の規約を読めませんでした。商品詳細で理由を確認し、再試行できます。", failed: "規約の読み取りを開始できませんでした。再試行してください。" },
  ko: { read: "기록된 약관 읽기", hint: "용도 판정에는 읽어 둔 약관이 필요해요. 알 수 없음은 허용을 뜻하지 않아요. 정보가 부족한 상품은 상세에서 상품 정보와 약관을 읽을 수 있어요.", reading: "약관 읽는 중…", done: "읽기가 끝나 용도 판정을 새로 고쳤어요.", stopped: "일부 약관을 읽지 못했어요. 상품 상세에서 이유를 확인하고 다시 읽을 수 있어요.", failed: "약관 읽기를 시작하지 못했어요. 다시 시도해 주세요." },
};
export const libraryTermsCopy = tables[currentLocale] ?? en;
