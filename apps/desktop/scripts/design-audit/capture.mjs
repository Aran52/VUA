const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

export function hasAnnotationLegend(image) {
  const pixels = image.toBitmap();
  let count = 0;
  for (let offset = 0; offset < pixels.length; offset += 4) {
    if (pixels[offset] >= 40 && pixels[offset] <= 50 && pixels[offset + 1] >= 40 && pixels[offset + 1] <= 50
      && pixels[offset + 2] >= 250 && pixels[offset + 3] >= 250 && ++count >= 20) return true;
  }
  return false;
}

export async function captureAuditPage(win, annotated = false, retryDelayMs = 200) {
  for (let attempt = 0; attempt < 2; attempt++) {
    win.webContents.invalidate();
    let timer;
    try {
      await Promise.race([
        win.webContents.executeJavaScript("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))", true),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("screenshot paint timed out")), 5000); }),
      ]);
    } finally { clearTimeout(timer); }
    const image = await win.webContents.capturePage({}, { stayHidden: true, stayAwake: true });
    if (image.isEmpty()) throw new Error("The screenshot is empty.");
    if (!annotated || hasAnnotationLegend(image)) return image;
    if (attempt === 0) await pause(retryDelayMs);
  }
  throw new Error("The annotation legend was not painted in the screenshot.");
}
