export type ErrorCode = "too_large" | "invalid_declaration" | "rate_limited" | "not_found" | "forbidden" | "method_not_allowed" | "code_collision" | "capacity_exceeded" | "unavailable";
export function errorResponse(status: number, error: ErrorCode, message: string, extra: HeadersInit = {}): Response {
  return new Response(JSON.stringify({ error, message }), {
    status,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "no-store",
      "X-Content-Type-Options": "nosniff",
      ...extra,
    },
  });
}
