import { afterEach, describe, expect, it, vi } from "vitest";
import { createDefaultTransport } from "./fetcher";
import { headerOf, init, json, OK_BODY, STATUS } from "./fetcher.fixtures";

describe("createDefaultTransport: токен из GET /session", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  function stubSession(session: Response) {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(session)
      .mockResolvedValue(json(STATUS.ok, OK_BODY));
    vi.stubGlobal("fetch", fetchMock);
    return fetchMock;
  }

  it("мутация сперва читает сессию (cookie той же страницы, не хранится в кэше) и ставит её токен", async () => {
    // Arrange
    const fetchMock = stubSession(json(STATUS.ok, { authenticated: true, csrf_token: "tok-9" }));
    const transport = createDefaultTransport();

    // Act
    await transport.send("/tokens", { method: "POST" });

    // Assert
    const [sessionUrl, sessionInit] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(sessionUrl).toBe("/api/v1/session");
    expect(sessionInit).toMatchObject({ credentials: "same-origin", cache: "no-store" });
    expect(headerOf(init(fetchMock, 1), "X-CSRF-Token")).toBe("tok-9");
  });

  it("сессии нет (csrf_token null) — unauthenticated, сама мутация не отправляется", async () => {
    // Arrange
    const fetchMock = stubSession(json(STATUS.ok, { authenticated: false, csrf_token: null }));
    const transport = createDefaultTransport();

    // Act
    const error = await transport
      .send("/tokens", { method: "POST" })
      .catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "unauthenticated" });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("ответ сессии без поля csrf_token тоже означает отсутствие сессии", async () => {
    // Arrange
    stubSession(json(STATUS.ok, { authenticated: true }));
    const transport = createDefaultTransport();

    // Act
    const error = await transport
      .send("/tokens", { method: "POST" })
      .catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "unauthenticated" });
  });

  it("сессия ответила ошибкой — ApiProblem с её статусом, мутация не уходит", async () => {
    // Arrange
    const fetchMock = stubSession(json(STATUS.serverError, { title: "Сбой" }));
    const transport = createDefaultTransport();

    // Act
    const error = await transport
      .send("/tokens", { method: "POST" })
      .catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "problem", status: STATUS.serverError });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("GET сессию не читает", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    vi.stubGlobal("fetch", fetchMock);
    const transport = createDefaultTransport();

    // Act
    await transport.send("/status", {});

    // Assert
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["пустое тело", new Response("", { status: STATUS.ok })],
    ["JSON null", new Response("null", { status: STATUS.ok })],
    ["JSON-строка", new Response('"oops"', { status: STATUS.ok })],
    ["токен не строка", new Response('{"csrf_token":123}', { status: STATUS.ok })],
  ])("сессия с телом «%s» — нет токена, а не падение на разборе", async (_name, session) => {
    // Arrange
    stubSession(session);
    const transport = createDefaultTransport();

    // Act
    const error = await transport
      .send("/tokens", { method: "POST" })
      .catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "unauthenticated" });
  });
});
