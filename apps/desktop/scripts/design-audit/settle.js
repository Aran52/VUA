// Serialized into the fixture page. Audit settled theme colours without changing
// camera transforms, page entrances or genuine activity animations.
export function finishThemeTransitions() {
  document.body.getBoundingClientRect();
  let finished = 0;
  for (const animation of document.getAnimations()) {
    if (typeof animation.transitionProperty !== "string") continue;
    if (!/^(?:color|background.*|border.*|box-shadow|fill|stroke)$/.test(animation.transitionProperty)) continue;
    animation.finish();
    finished++;
  }
  return finished;
}
