/**
 * SceneCanvas:WebGL 场景统一挂载点(S-VFX-0)。
 *
 * 职责:renderer/场景/相机生命周期、DPR(上限 2)与 resize、rAF 循环、
 * 指针归一化 + 指数平滑、上下文丢失/恢复、降级(见 scene-mode.ts)。
 * 视觉内容全部由调用方以 SceneFactory 注入;本组件不引入任何视觉值。
 *
 * 渐进增强:mode === "off" 不渲染 canvas;上下文创建失败静默回退
 * (CSS 场景兜底),不抛错、不阻塞界面。
 */
import { useEffect, useRef } from "react";
import * as THREE from "three";
import { smoothDamp } from "./scene-mode.ts";
import { useSceneMode } from "./useSceneMode.ts";

export interface ScenePointer {
  /** 归一化 -1..1,已平滑 */
  x: number;
  y: number;
}

export interface SceneHandle {
  update(dt: number, elapsed: number, pointer: ScenePointer): void;
  dispose(): void;
}

export type SceneFactory = (ctx: {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
}) => SceneHandle;

export interface SceneCanvasProps {
  factory: SceneFactory;
  className?: string;
}

export function SceneCanvas({ factory, className }: SceneCanvasProps) {
  const mode = useSceneMode();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const factoryRef = useRef(factory);
  factoryRef.current = factory;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || mode === "off") return;

    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({
        canvas,
        antialias: true,
        alpha: true,
        // 桌面常驻应用:不主动丢上下文,丢了走 lost/restored 重建
        powerPreference: "high-performance",
      });
    } catch {
      return;
    }
    renderer.setClearColor(0x000000, 0);

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 100);
    const handle = factoryRef.current({ scene, camera, renderer });

    const resize = () => {
      const { clientWidth: width, clientHeight: height } = canvas;
      if (width === 0 || height === 0) return;
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    resize();

    const pointer: ScenePointer = { x: 0, y: 0 };
    const pointerTarget: ScenePointer = { x: 0, y: 0 };
    const onPointerMove = (event: PointerEvent) => {
      pointerTarget.x = (event.clientX / window.innerWidth) * 2 - 1;
      pointerTarget.y = -((event.clientY / window.innerHeight) * 2 - 1);
    };
    window.addEventListener("pointermove", onPointerMove, { passive: true });

    let raf = 0;
    let last = performance.now();
    let elapsed = 0;
    const frame = (now: number) => {
      const dt = Math.min((now - last) / 1000, 0.1);
      last = now;
      elapsed += dt;
      pointer.x = smoothDamp(pointer.x, pointerTarget.x, 6, dt);
      pointer.y = smoothDamp(pointer.y, pointerTarget.y, 6, dt);
      handle.update(dt, elapsed, pointer);
      renderer.render(scene, camera);
      raf = requestAnimationFrame(frame);
    };

    const onContextLost = (event: Event) => {
      event.preventDefault();
      cancelAnimationFrame(raf);
    };
    const onContextRestored = () => {
      resize();
      last = performance.now();
      if (mode === "animated") raf = requestAnimationFrame(frame);
    };
    canvas.addEventListener("webglcontextlost", onContextLost);
    canvas.addEventListener("webglcontextrestored", onContextRestored);

    if (mode === "static") {
      // reduced-motion:渲一帧静态画面,不跑循环
      handle.update(0, 0, pointer);
      renderer.render(scene, camera);
    } else {
      raf = requestAnimationFrame(frame);
    }

    return () => {
      cancelAnimationFrame(raf);
      resizeObserver.disconnect();
      window.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("webglcontextlost", onContextLost);
      canvas.removeEventListener("webglcontextrestored", onContextRestored);
      handle.dispose();
      renderer.dispose();
    };
  }, [mode]);

  if (mode === "off") return null;
  return <canvas ref={canvasRef} className={className} aria-hidden="true" data-audit-skip="webgl-scene-art" />;
}
