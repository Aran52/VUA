import type { PlayRoute } from "@vua/contracts";

/** Which route's details the Play page opens with, when something else sent the player there. */
let requested: PlayRoute | null = null;
export function requestPlayRoute(route: PlayRoute): void { requested = route; }
export function takeRequestedPlayRoute(): PlayRoute | null {
  const route = requested; requested = null; return route;
}
