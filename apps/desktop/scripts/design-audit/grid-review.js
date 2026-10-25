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
  const bevel = element => {
    const style = getComputedStyle(element);
    const [x, y] = style.borderTopLeftRadius.split(/\s+/).map(parseFloat);
    return style.getPropertyValue("corner-top-left-shape") === "superellipse(0)"
      && Math.abs(y / x - Math.sqrt(3)) < 0.01 && style.borderTopRightRadius === "0px"
      && style.borderBottomRightRadius === "0px" && style.borderBottomLeftRadius === "0px";
  };
  if (view.startsWith("workshop-")) {
    await until(() => document.querySelectorAll(".vua-line__station").length >= 6);
    const steps = [...document.querySelectorAll(".vua-line__station")];
    check(steps.every(bevel), "Every workshop step uses the upper-left 120-degree bevel");
    const active = steps.find(step => step.getAttribute("aria-pressed") === "true");
    check(active && getComputedStyle(active).boxShadow !== "none", "The selected workshop step keeps its heavier elevation");
    check(steps.filter(step => step !== active).every(step => getComputedStyle(step).boxShadow === "none"), "Resting workshop steps stay flat");
  } else {
    await until(() => document.querySelectorAll(".vua-control__repo").length >= 2);
    const rows = [...document.querySelectorAll(".vua-control__repo")];
    const refresh = rows.map(row => row.querySelector(".vua-control__repo-actions > button"));
    check(refresh.every(button => button && button.getClientRects().length > 0), "Refresh remains visible in every repository row");
    check(rows.every(row => row.querySelector(".vua-control__repo-actions").children.length === 2), "Every row reserves both action positions");
    const aligned = values => values.every(value => Math.abs(value - values[0]) <= 1);
    check(aligned(refresh.map(button => button.getBoundingClientRect().left)), "Refresh positions align across available and unavailable rows");
    check(aligned(rows.map(row => row.children[3].getBoundingClientRect().right)), "Repository package counts keep their common right edge");
    if (view.endsWith("-disabled")) check(refresh.every(button => button.disabled), "Unavailable lifecycle keeps visible disabled refresh controls");
    const local = rows.findIndex(row => row.textContent.includes("Local preview"));
    check(local >= 0 && refresh[local].disabled, "A missing repository identifier keeps the refresh slot disabled");
  }
}
