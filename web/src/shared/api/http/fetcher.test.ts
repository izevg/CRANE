import { afterEach, describe, expect, it, vi } from "vitest";
import { createTransport, fetcher } from "./fetcher";
import { csrfProblem, headerOf, init, json, OK_BODY, STATUS } from "./fetcher.fixtures";
import { ApiProblem } from "./problem";

describe("createTransport: запрос", () => {
  it("GET не ставит X-CSRF-Token, не читает сессию и ревалидирует кэш (no-cache)", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const readCsrfToken = vi.fn().mockResolvedValue("tok");
    const transport = createTransport({ fetch: fetchMock, readCsrfToken });

    // Act
    await transport.send("/status", { method: "GET" });

    // Assert
    const request = init(fetchMock);
    expect(headerOf(request, "X-CSRF-Token")).toBeNull();
    expect(request.cache).toBe("no-cache");
    expect(request.credentials).toBe("same-origin");
    expect(readCsrfToken).not.toHaveBeenCalled();
  });

  it("запрос идёт на префикс /api/v1", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    await transport.send("/status", {});

    // Assert
    expect(fetchMock.mock.calls[0]?.[0]).toBe("/api/v1/status");
  });

  it("мутация ставит X-CSRF-Token и cache no-store", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const transport = createTransport({
      fetch: fetchMock,
      readCsrfToken: () => Promise.resolve("tok-7"),
    });

    // Act
    await transport.send("/tokens", { method: "POST", body: "{}" });

    // Assert
    const request = init(fetchMock);
    expect(headerOf(request, "X-CSRF-Token")).toBe("tok-7");
    expect(request.cache).toBe("no-store");
  });

  it("мутация с Authorization (Bearer) не читает сессию и уходит без X-CSRF-Token", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const readCsrfToken = vi.fn().mockResolvedValue(null);
    const transport = createTransport({ fetch: fetchMock, readCsrfToken });

    // Act
    await transport.send("/onboarding/steps", {
      method: "POST",
      headers: { Authorization: "Bearer crb_demo" },
    });

    // Assert
    const request = init(fetchMock);
    expect(readCsrfToken).not.toHaveBeenCalled();
    expect(headerOf(request, "X-CSRF-Token")).toBeNull();
    expect(headerOf(request, "Authorization")).toBe("Bearer crb_demo");
  });

  it("запрос с Bearer уходит без cookie: credentials omit", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    await transport.send("/onboarding/steps", {
      method: "POST",
      headers: { Authorization: "Bearer crb_demo" },
    });

    // Assert
    expect(init(fetchMock).credentials).toBe("omit");
  });

  it("пустой Authorization — не Bearer: обычный запрос с cookie и X-CSRF-Token", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const transport = createTransport({
      fetch: fetchMock,
      readCsrfToken: () => Promise.resolve("tok-3"),
    });

    // Act
    await transport.send("/tokens", { method: "POST", headers: { Authorization: "" } });

    // Assert
    const request = init(fetchMock);
    expect(request.credentials).toBe("same-origin");
    expect(headerOf(request, "X-CSRF-Token")).toBe("tok-3");
  });

  it("CSRF-отказ bearer-запроса не повторяется: токен перечитывать нечем", async () => {
    // Arrange
    const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(csrfProblem()));
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    const error = await transport
      .send("/x", { method: "POST", headers: { Authorization: "Bearer t" } })
      .catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "csrf" });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("транспорт не выдумывает Idempotency-Key", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const transport = createTransport({
      fetch: fetchMock,
      readCsrfToken: () => Promise.resolve("t"),
    });

    // Act
    await transport.send("/tokens", { method: "POST" });

    // Assert
    expect(headerOf(init(fetchMock), "Idempotency-Key")).toBeNull();
  });

  it("заголовок операции (Idempotency-Key вызывающего) доходит до сети", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    const transport = createTransport({
      fetch: fetchMock,
      readCsrfToken: () => Promise.resolve("t"),
    });

    // Act
    await transport.send("/tokens", { method: "POST", headers: { "Idempotency-Key": "k-1" } });

    // Assert
    expect(headerOf(init(fetchMock), "Idempotency-Key")).toBe("k-1");
  });

  it("успех возвращается как { data, status, headers }", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY, { ETag: "W/1" }));
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    const result = await transport.send<{ data: unknown; status: number; headers: Headers }>(
      "/status",
      {},
    );

    // Assert
    expect(result.data).toEqual(OK_BODY);
    expect(result.status).toBe(STATUS.ok);
    expect(result.headers.get("ETag")).toBe("W/1");
  });

  it("204 без тела даёт data undefined", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: STATUS.noContent }));
    const transport = createTransport({
      fetch: fetchMock,
      readCsrfToken: () => Promise.resolve("t"),
    });

    // Act
    const result = await transport.send<{ data: unknown }>("/x", { method: "DELETE" });

    // Assert
    expect(result.data).toBeUndefined();
  });
});

describe("createTransport: ошибки", () => {
  it("не-2xx бросает ApiProblem, а не возвращает значением", async () => {
    // Arrange
    const fetchMock = vi
      .fn()
      .mockResolvedValue(
        json(STATUS.notFound, { title: "Нет", status: STATUS.notFound, request_id: "r" }),
      );
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    const error = await transport.send("/packages/x", {}).catch((cause: unknown) => cause);

    // Assert
    expect(error).toBeInstanceOf(ApiProblem);
    expect(error).toMatchObject({ status: STATUS.notFound, requestId: "r" });
  });

  it("сбой сети — ApiProblem network", async () => {
    // Arrange
    const fetchMock = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    const error = await transport.send("/status", {}).catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "network" });
  });

  it("отмена запроса остаётся AbortError и не превращается в network", async () => {
    // Arrange
    const abort = new DOMException("aborted", "AbortError");
    const transport = createTransport({
      fetch: vi.fn().mockRejectedValue(abort),
      readCsrfToken: vi.fn(),
    });

    // Act
    const error = await transport.send("/status", {}).catch((cause: unknown) => cause);

    // Assert
    expect(error).toBe(abort);
  });

  it("2xx с телом не JSON — ApiProblem, а не пустой успех", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(new Response("<html>", { status: STATUS.ok }));
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    const error = await transport.send("/status", {}).catch((cause: unknown) => cause);

    // Assert
    expect(error).toBeInstanceOf(ApiProblem);
    expect(error).toMatchObject({ kind: "problem", status: STATUS.ok });
  });

  it("DOMException не про отмену — сбой сети, а не пробрасываемая ошибка", async () => {
    // Arrange
    const failure = new DOMException("blocked", "NetworkError");
    const transport = createTransport({
      fetch: vi.fn().mockRejectedValue(failure),
      readCsrfToken: vi.fn(),
    });

    // Act
    const error = await transport.send("/status", {}).catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "network" });
  });
});

describe("createTransport: CSRF-отказ", () => {
  it("один повтор после перечитывания токена", async () => {
    // Arrange
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(csrfProblem())
      .mockResolvedValueOnce(json(STATUS.ok, OK_BODY));
    const readCsrfToken = vi.fn().mockResolvedValueOnce("old").mockResolvedValueOnce("fresh");
    const transport = createTransport({ fetch: fetchMock, readCsrfToken });

    // Act
    await transport.send("/tokens", { method: "POST" });

    // Assert
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(headerOf(init(fetchMock, 0), "X-CSRF-Token")).toBe("old");
    expect(headerOf(init(fetchMock, 1), "X-CSRF-Token")).toBe("fresh");
  });

  it("второй отказ подряд — ошибка csrf, третьей попытки нет", async () => {
    // Arrange
    const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(csrfProblem()));
    const transport = createTransport({
      fetch: fetchMock,
      readCsrfToken: () => Promise.resolve("t"),
    });

    // Act
    const error = await transport
      .send("/tokens", { method: "POST" })
      .catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "csrf" });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("другая ошибка мутации (500) не повторяется", async () => {
    // Arrange
    const fetchMock = vi
      .fn()
      .mockImplementation(() => Promise.resolve(json(STATUS.serverError, {})));
    const transport = createTransport({
      fetch: fetchMock,
      readCsrfToken: () => Promise.resolve("t"),
    });

    // Act
    const error = await transport
      .send("/tokens", { method: "POST" })
      .catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "problem", status: STATUS.serverError });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("отказ CSRF на GET не повторяется", async () => {
    // Arrange
    const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(csrfProblem()));
    const transport = createTransport({ fetch: fetchMock, readCsrfToken: vi.fn() });

    // Act
    const error = await transport.send("/status", {}).catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "csrf" });
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});

describe("fetcher: модульный транспорт", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("ходит через глобальный fetch на /api/v1", async () => {
    // Arrange
    const fetchMock = vi.fn().mockResolvedValue(json(STATUS.ok, OK_BODY));
    vi.stubGlobal("fetch", fetchMock);

    // Act
    const result = await fetcher<{ data: unknown }>("/status", { method: "GET" });

    // Assert
    expect(result.data).toEqual(OK_BODY);
    expect(fetchMock.mock.calls[0]?.[0]).toBe("/api/v1/status");
  });
});
