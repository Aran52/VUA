/** Plain words for the VRChat SDK's most common Avatar check messages (SDK 3.10.5). The SDK's own
 * sentence stays one click away; a message without plain words shows as the SDK wrote it. */
export const SDK_MESSAGE_KEYS = [
  "noPipeline", "multiplePipelines", "meshReadWrite", "tooShort", "shorter", "tooTall", "taller",
  "unityConstraints", "constraintsNotAllowed", "writeDefaultsEmpty", "boxMipmaps", "streamingMipmaps",
  "texturesTooBig", "visemeFaceMesh", "avatarDisabled", "feetMissing", "upperArmsMissing",
  "tooManyParameters", "parametersMissing", "menuMissing", "menuIconsLarge", "dynamicBones",
  "particleLights", "gestureMask", "unsupportedShader", "mayBeBlocked", "blockedByDefault",
  "canBeFallback", "notFallbackRating", "notFallbackHumanoid", "fallbackAnimators", "stations",
] as const;
export type SdkMessageKey = (typeof SDK_MESSAGE_KEYS)[number];

// Each pattern matches the SDK's fixed wording; counts and names inside a message do not matter.
const PATTERNS: readonly (readonly [SdkMessageKey, RegExp])[] = [
  ["noPipeline", /^This avatar has no pipeline manager component/u],
  ["multiplePipelines", /^There are multiple pipeline manager components/u],
  ["meshReadWrite", /imported with Read\/Write disabled/u],
  ["tooShort", /^This avatar is too short/u],
  ["shorter", /^This avatar is shorter than average/u],
  ["tooTall", /^This avatar is too tall/u],
  ["taller", /^This avatar is taller than average/u],
  ["unityConstraints", /^This avatar uses Unity constraints/u],
  ["constraintsNotAllowed", /^Unity constraints are not allowed on this platform/u],
  ["writeDefaultsEmpty", /Write Defaults disabled where the animation clip is either missing or empty/u],
  ["boxMipmaps", /'Box' mipmap filtering/u],
  ["streamingMipmaps", /without 'Streaming Mip Maps' enabled/u],
  ["texturesTooBig", /^This avatar has textures bigger than/u],
  ["visemeFaceMesh", /^This avatar uses Visemes but the Face Mesh is not specified/u],
  ["avatarDisabled", /^Your avatar is disabled in the scene hierarchy/u],
  ["feetMissing", /feet aren't specified/u],
  ["upperArmsMissing", /upper arms aren't specified/u],
  ["tooManyParameters", /^VRCExpressionParameters has too many parameters/u],
  ["parametersMissing", /^VRCExpressionParameters object reference is missing/u],
  ["menuMissing", /^VRCExpressionsMenu object reference is missing/u],
  ["menuIconsLarge", /^Images used for Actions & Moods are too large/u],
  ["dynamicBones", /DynamicBone components/u],
  ["particleLights", /particle systems configured to emit dynamic lights/u],
  ["gestureMask", /^Gesture Layer needs valid mask/u],
  ["unsupportedShader", /^Avatar uses unsupported shader/u],
  ["mayBeBlocked", /^Other users may choose to block this avatar due to performance/u],
  ["blockedByDefault", /^This avatar will be blocked by default due to performance/u],
  ["canBeFallback", /^This avatar can be used as a custom fallback/u],
  ["notFallbackRating", /overall rating of Good or better, so it can not be used as a custom fallback/u],
  ["notFallbackHumanoid", /does not have a humanoid rig, so it can not be used as a custom fallback/u],
  ["fallbackAnimators", /uses additional animators, they will be disabled when used as a fallback/u],
  ["stations", /^Stations found on Avatar/u],
];

/** The plain-word message for one SDK sentence, or null when VUA shows the SDK's own words. */
export function sdkMessageKey(text: string): SdkMessageKey | null {
  return PATTERNS.find(([, pattern]) => pattern.test(text))?.[0] ?? null;
}
