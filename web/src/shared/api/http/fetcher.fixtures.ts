export const STATUS = {
  serverError: 500,
  ok: 200,
  noContent: 204,
  forbidden: 403,
  notFound: 404,
} as const;

export const OK_BODY = { version: "0.3.0" };

export function json(
  status: number,
  body: unknown,
  headers: Record<string, string> = {},
): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...headers },
  });
}

export function csrfProblem(): Response {
  return json(STATUS.forbidden, {
    type: "https://crane.example/problems/csrf",
    title: "CSRF",
    status: STATUS.forbidden,
  });
}

export function init(fetchMock: { mock: { calls: unknown[][] } }, call = 0): RequestInit {
  return fetchMock.mock.calls[call]?.[1] as RequestInit;
}

export function headerOf(request: RequestInit, name: string): string | null {
  return new Headers(request.headers).get(name);
}
