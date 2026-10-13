"use strict";

document.addEventListener("click", async event => {
  const button = event.target instanceof Element ? event.target.closest("button[data-copy]") : null;
  if (!button || button.disabled) return;
  const value = button.dataset.copy;
  if (!value || !(/^[a-f0-9]{64}$/.test(value) || /^https:\/\/ph-r\.com\/vua\/releases\/v\d+\.\d+\.\d+\/VUA-\d+\.\d+\.\d+-windows-x64-(setup|offline)\.exe$/.test(value))) return;
  const status = document.getElementById("vua-copy-status");
  button.disabled = true;
  try {
    await navigator.clipboard.writeText(value);
    if (status) status.textContent = button.dataset.copyKind === "checksum" ? "已复制 SHA-256 校验值。" : "已复制安装包链接。";
  } catch {
    if (status) status.textContent = "浏览器未允许复制。可以右键复制下载链接，或展开校验信息后选择文本。";
  } finally {
    button.disabled = false;
  }
});
