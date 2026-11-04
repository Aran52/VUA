// Serialized into the real component fixture; no Gateway or filesystem writes.
export async function verifyGridFactoryState(view) {
  const until = async predicate => {
    const deadline = performance.now() + 7000;
    while (!predicate()) {
      if (performance.now() > deadline) throw new Error("Grid review surface did not settle: " + view);
      await new Promise(resolve => setTimeout(resolve, 30));
    }
  };
  const check = (ok, message) => { if (!ok) throw new Error(message); };
  const roundedContainer = radius => element => {
    const style = getComputedStyle(element);
    return style.getPropertyValue("corner-top-left-shape") === "superellipse(1)"
      && style.borderTopLeftRadius === radius && style.borderTopRightRadius === radius
      && style.borderBottomRightRadius === radius && style.borderBottomLeftRadius === radius;
  };
  const family = document.documentElement.dataset.themeFamily ?? "missing";
  const container = {
    grid: roundedContainer("8px"),
    "frutiger-aero": roundedContainer("4px"),
    tenshi: roundedContainer("14px"),
    jirai: roundedContainer("10px"),
  }[family];
  check(container, `Workshop review has a known theme family: ${family}`);
  if (view.startsWith("workshop-")) {
    await until(() => document.querySelectorAll(".vua-line__station").length >= 6);
    const steps = [...document.querySelectorAll(".vua-line__station")];
    check(steps.every(container), `Every workshop step uses the ${family} container shape`);
    if (family === "grid") {
      check(!document.querySelector(".vua-line__skin-motif"), "Grid keeps the workshop free of skin motifs");
    } else {
      await until(() => {
        const motif = document.querySelector(".vua-line__skin-motif");
        return motif?.complete && motif.naturalWidth > 0;
      });
      check(document.querySelector(".vua-line__skin-motif")?.dataset.motifFamily === family,
        `Workshop motif loads from the ${family} preset`);
    }
    const active = steps.find(step => step.getAttribute("aria-pressed") === "true");
    // Viewing and execution never change a step card's shadow; Grid's step cards have none.
    const shadows = new Set(steps.map(step => getComputedStyle(step).boxShadow));
    check(active && shadows.size === 1 && (family !== "grid" || shadows.has("none")), "Viewing and execution never grant step cards cast elevation");
    if (view === "workshop-running") {
      const selectedBackground = getComputedStyle(active).backgroundColor;
      const running = steps.find(step => step.dataset.state === "running");
      const future = steps.find(step => step.dataset.state === "todo");
      check(running && future, "Running fixture has current and future steps");
      await until(() => document.querySelector(".vua-line__station-indicator"));
      const indicator = document.querySelector(".vua-line__station-indicator");
      check(document.querySelectorAll(".vua-line__station-indicator").length === 1, "One shared indicator marks the viewed station");
      const motion = getComputedStyle(indicator);
      check(motion.transitionProperty.includes("transform") && motion.transitionTimingFunction.includes("0.55") && parseFloat(motion.transitionDuration) > 0,
        "Station indicator uses an accelerated transform transition");
      const indicatorAt = step => Math.abs(indicator.getBoundingClientRect().left - (step.getBoundingClientRect().left + 12)) <= 1;
      future.click();
      await until(() => future.getAttribute("aria-pressed") === "true");
      await until(() => getComputedStyle(future).backgroundColor === selectedBackground);
      await until(() => indicatorAt(future));
      check(indicator === document.querySelector(".vua-line__station-indicator"), "The same indicator moves instead of being recreated");
      check(running.getAttribute("aria-current") === "step" && future.getAttribute("aria-current") === null, "Viewing a future step cannot move execution");
      check(getComputedStyle(future).backgroundColor === selectedBackground, "Future selection uses the same neutral viewing treatment as the current step");
      check(document.querySelector(".vua-line__context").textContent.includes(future.querySelector("strong").textContent)
        && document.querySelector(".vua-line__context").textContent.includes(running.querySelector("strong").textContent), "Detail names both viewed and executing steps");
      running.click();
      await until(() => running.getAttribute("aria-pressed") === "true");
      await until(() => getComputedStyle(running).backgroundColor === selectedBackground);
      await until(() => indicatorAt(running));
    }
  } else {
    await until(() => document.querySelectorAll(".vua-control__repo").length >= 2);
    const rows = [...document.querySelectorAll(".vua-control__repo")];
    const refresh = rows.map(row => row.querySelector(".vua-control__repo-actions > button"));
    check(refresh.every(button => button && button.getClientRects().length > 0), "Refresh remains visible in every repository row");
    check(rows.every(row => row.querySelector(".vua-control__repo-actions").children.length === 2), "Every row reserves both action positions");
    const aligned = values => values.every(value => Math.abs(value - values[0]) <= 1);
    check(aligned(refresh.map(button => button.getBoundingClientRect().left)), "Refresh positions align across available and unavailable rows");
    check(aligned(rows.map(row => row.children[3].getBoundingClientRect().right)), "Repository package counts keep their common right edge");
    const countTextEdges = rows.map(row => row.querySelector(".vua-control__repo-count")).filter(element => element.textContent.trim()).map(element => {
      const range = document.createRange(); range.selectNodeContents(element); return range.getBoundingClientRect().right;
    });
    check(aligned(countTextEdges), "Rendered package-count text, including the longer unavailable label, is right-aligned");
    if (view.endsWith("-disabled")) check(refresh.every(button => button.disabled), "Unavailable lifecycle keeps visible disabled refresh controls");
    const local = rows.findIndex(row => row.textContent.includes("Local preview"));
    check(local >= 0 && refresh[local].disabled, "A missing repository identifier keeps the refresh slot disabled");
    refresh[local].click();
    check(rows[local].isConnected && refresh[local].disabled, "Clicking a disabled refresh leaves the repository row intact");
    check(document.querySelectorAll(".vua-control__repo-columns > span").length === 5, "Repository list labels its five columns");
    const add = document.querySelector(".vua-control__head .vua-button--primary");
    if (add) {
      check(document.querySelectorAll(".vua-control .vua-button--primary").length === 1, "Adding is the only repository primary action");
      add.click();
      await until(() => document.querySelector(".vua-control__add"));
      const form = document.querySelector(".vua-control__add");
      check(form.querySelector('button[type="submit"]').disabled, "Empty repository form cannot advance");
      form.querySelector('button[type="button"]').click();
      await until(() => !document.querySelector(".vua-control__add"));
    }
  }
}
