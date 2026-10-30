// Common setup for focused native smokes; the dedicated startup smoke covers both choices.
export async function enterHomeFromFirstUse(js, waitFor) {
  await waitFor(() => js('!!document.querySelector("[data-first-use-notice]")'), "first-use notice");
  await js('document.querySelector("[data-nav-id=first-use-beginner]").click()');
  await waitFor(() => js('!!document.querySelector("[data-wizard-step=vrchat]")'), "beginner route");
  await js('document.querySelector("[data-nav-id=logo-home]").click()');
  await waitFor(() => js('!!document.querySelector(".vua-home") && !document.querySelector(".vua-onboarding") && !document.querySelector(".vua-tour")'), "Home after the beginner choice");
}
