import { buildSplashRays } from "./components/splash/boot-splash-model.ts";
import "./components/splash/boot-splash.css";

function flattenStartupMotion(): boolean {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return true;
  try {
    return localStorage.getItem("vua-effects") === "off";
  } catch {
    return false;
  }
}

const surface = document.querySelector<HTMLElement>(".vua-boot-splash__surface");
const raysHost = document.querySelector<HTMLElement>(".vua-boot-splash__rays");

function renderRays(width: number, height: number): void {
  if (!raysHost) return;
  const fragment = document.createDocumentFragment();
  for (const ray of buildSplashRays(width, height)) {
    const track = document.createElement("div");
    track.className = "vua-boot-splash__ray-track";
    track.style.left = `${ray.left}px`;
    track.style.top = `${ray.top}px`;
    const bar = document.createElement("div");
    bar.className = "vua-boot-splash__ray";
    bar.style.top = `${-ray.thickness / 2}px`;
    bar.style.width = `${ray.length}px`;
    bar.style.height = `${ray.thickness}px`;
    bar.style.animationDelay = `${ray.delayMs}ms`;
    track.appendChild(bar);
    fragment.appendChild(track);
  }
  raysHost.replaceChildren(fragment);
}

if (surface) {
  if (flattenStartupMotion()) surface.setAttribute("data-static", "true");
  renderRays(surface.clientWidth || 480, surface.clientHeight || 320);
}
