import { describe, expect, it } from "vitest";
import { ApiProblem, parseProblem, parseRetryAfter, problemFromResponse } from "./problem";

const RETRY_AFTER_MS = 7000;
const RETRY_AFTER_SHORT_MS = 3000;
const TWO_MINUTES_MS = 120_000;
const HALF_MINUTE_MS = 30_000;

const STATUS = {
  badRequest: 400,
  unauthorized: 401,
  forbidden: 403,
  notFound: 404,
  tooManyRequests: 429,
  serverError: 500,
  badGateway: 502,
  unavailable: 503,
} as const;

const NOW = Date.parse("2026-10-28T12:00:00Z");
const NO_EXTRA = { now: NOW };
const BODY = { type: "https://crane.example/problems/x", title: "Сбой", request_id: "req-1" };

describe("parseProblem: вид проблемы по ответу", () => {
  it("401 без required_acr — unauthenticated с причиной сессии", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.unauthorized, reason: "session_expired" };

    // Act
    const problem = parseProblem(STATUS.unauthorized, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({
      kind: "unauthenticated",
      status: STATUS.unauthorized,
      reason: "session_expired",
    });
  });

  it("401 с required_acr — step-up, а не вход", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.unauthorized, required_acr: "mfa" };

    // Act
    const problem = parseProblem(STATUS.unauthorized, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({ kind: "step-up", requiredAcr: "mfa" });
  });

  it("403 с rule_id несёт правило, контакт и шаги", () => {
    // Arrange
    const body = {
      ...BODY,
      status: STATUS.forbidden,
      rule_id: "repo.publish.require-maintainer",
      remediation: ["Запросите роль", null, "Повторите"],
      contact: { display: "Петрова А.", role: "owner" },
    };

    // Act
    const problem = parseProblem(STATUS.forbidden, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({
      kind: "problem",
      ruleId: "repo.publish.require-maintainer",
      remediation: ["Запросите роль", "Повторите"],
      contact: "Петрова А.",
    });
  });

  it("403 с признаком csrf — csrf", () => {
    // Arrange
    const body = { ...BODY, type: "https://crane.example/problems/csrf", status: STATUS.forbidden };

    // Act
    const problem = parseProblem(STATUS.forbidden, body, NO_EXTRA);

    // Assert
    expect(problem.kind).toBe("csrf");
  });

  it("403 по правилу, чьё имя лишь содержит csrf, остаётся обычной проблемой", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.forbidden, reason: "not_csrf_related" };

    // Act
    const problem = parseProblem(STATUS.forbidden, body, NO_EXTRA);

    // Assert
    expect(problem.kind).toBe("problem");
  });

  it("404 — обычная проблема без полей правила", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.notFound };

    // Act
    const problem = parseProblem(STATUS.notFound, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({
      kind: "problem",
      status: STATUS.notFound,
      ruleId: undefined,
      remediation: [],
    });
  });

  it("5xx — обычная проблема с title и идентификатором запроса", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.serverError, detail: "Хранилище недоступно" };

    // Act
    const problem = parseProblem(STATUS.serverError, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({
      kind: "problem",
      status: STATUS.serverError,
      title: "Сбой",
      detail: "Хранилище недоступно",
      requestId: "req-1",
    });
  });

  it("не-JSON тело даёт проблему со статусом и без полей; идентификатор берётся из заголовка", () => {
    // Arrange
    const html = "<html>Bad gateway</html>";

    // Act
    const problem = parseProblem(STATUS.badGateway, html, { now: NOW, requestId: "hdr-9" });

    // Assert
    expect(problem).toMatchObject({
      kind: "problem",
      status: STATUS.badGateway,
      title: "",
      requestId: "hdr-9",
    });
  });

  it("request_id тела важнее заголовка", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.serverError };

    // Act
    const problem = parseProblem(STATUS.serverError, body, { now: NOW, requestId: "hdr-9" });

    // Assert
    expect(problem.requestId).toBe("req-1");
  });

  it("Retry-After сохраняется в миллисекундах", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.tooManyRequests };

    // Act
    const problem = parseProblem(STATUS.tooManyRequests, body, { now: NOW, retryAfter: "7" });

    // Assert
    expect(problem.retryAfterMs).toBe(RETRY_AFTER_MS);
  });
});

describe("parseProblem: признак CSRF и пустые поля", () => {
  it("403 с reason вида csrf_token_mismatch — csrf", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.forbidden, reason: "csrf_token_mismatch" };

    // Act
    const problem = parseProblem(STATUS.forbidden, body, NO_EXTRA);

    // Assert
    expect(problem.kind).toBe("csrf");
  });

  it("признак csrf у не-403 ответа ничего не значит", () => {
    // Arrange
    const body = {
      ...BODY,
      type: "https://crane.example/problems/csrf",
      status: STATUS.badRequest,
    };

    // Act
    const problem = parseProblem(STATUS.badRequest, body, NO_EXTRA);

    // Assert
    expect(problem.kind).toBe("problem");
  });

  it("пустая строка в теле — то же, что отсутствие поля", () => {
    // Arrange
    const body = { type: "", title: "", detail: "", reason: "", rule_id: "", request_id: "" };

    // Act
    const problem = parseProblem(STATUS.serverError, body, { now: NOW, requestId: "hdr-1" });

    // Assert
    expect(problem).toMatchObject({
      title: "",
      detail: undefined,
      reason: undefined,
      ruleId: undefined,
      requestId: "hdr-1",
    });
  });

  it("тело JSON null разбирается как пустое", () => {
    // Arrange
    const body: unknown = null;

    // Act
    const problem = parseProblem(STATUS.serverError, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({ title: "", status: STATUS.serverError });
  });

  it("поле не строкового типа считается отсутствующим", () => {
    // Arrange
    const body = { title: 42, detail: { text: "x" }, request_id: 7 };

    // Act
    const problem = parseProblem(STATUS.serverError, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({ title: "", detail: undefined, requestId: undefined });
  });

  it("массив вместо объекта тела разбирается как пустое тело", () => {
    // Arrange
    const body = [{ title: "x" }];

    // Act
    const problem = parseProblem(STATUS.serverError, body, NO_EXTRA);

    // Assert
    expect(problem).toMatchObject({ title: "", status: STATUS.serverError });
  });

  it("contact без display не даёт контакта", () => {
    // Arrange
    const body = { ...BODY, status: STATUS.forbidden, contact: { role: "owner" } };

    // Act
    const problem = parseProblem(STATUS.forbidden, body, NO_EXTRA);

    // Assert
    expect(problem.contact).toBeUndefined();
  });
});

describe("ApiProblem", () => {
  it("сообщение — title сервера, а без него — вид проблемы; имя класса не теряется", () => {
    // Arrange
    const withTitle = new ApiProblem({
      kind: "problem",
      status: STATUS.serverError,
      title: "Сбой",
    });
    const bare = new ApiProblem({ kind: "planned", status: 0 });

    // Act
    const summary = [withTitle, bare].map((problem) => `${problem.name}: ${problem.message}`);

    // Assert
    expect(summary).toEqual(["ApiProblem: Сбой", "ApiProblem: planned"]);
  });

  it("без шагов remediation пуст, а не undefined", () => {
    // Arrange
    const problem = new ApiProblem({ kind: "problem", status: STATUS.serverError });

    // Act
    const steps = problem.remediation;

    // Assert
    expect(steps).toEqual([]);
  });
});

describe("parseRetryAfter", () => {
  it("секунды переводятся в миллисекунды", () => {
    // Arrange
    const header = " 120 ";

    // Act
    const ms = parseRetryAfter(header, NOW);

    // Assert
    expect(ms).toBe(TWO_MINUTES_MS);
  });

  it("дата HTTP считается от текущего времени", () => {
    // Arrange
    const header = "Wed, 28 Oct 2026 12:00:30 GMT";

    // Act
    const ms = parseRetryAfter(header, NOW);

    // Assert
    expect(ms).toBe(HALF_MINUTE_MS);
  });

  it("дата в прошлом даёт ноль, а не отрицательное ожидание", () => {
    // Arrange
    const header = "Wed, 28 Oct 2026 11:00:00 GMT";

    // Act
    const ms = parseRetryAfter(header, NOW);

    // Assert
    expect(ms).toBe(0);
  });

  it.each([null, "скоро", ""])("нечитаемое значение %j — undefined", (header) => {
    // Arrange
    const raw = header;

    // Act
    const ms = parseRetryAfter(raw, NOW);

    // Assert
    expect(ms).toBeUndefined();
  });
});

describe("problemFromResponse", () => {
  it("читает problem+json, заголовки X-Request-Id и Retry-After", async () => {
    // Arrange
    const response = new Response(
      JSON.stringify({ title: "Много запросов", status: STATUS.tooManyRequests }),
      {
        status: STATUS.tooManyRequests,
        headers: {
          "Content-Type": "application/problem+json",
          "X-Request-Id": "r-5",
          "Retry-After": "3",
        },
      },
    );

    // Act
    const problem = await problemFromResponse(response);

    // Assert
    expect(problem).toMatchObject({
      status: STATUS.tooManyRequests,
      title: "Много запросов",
      requestId: "r-5",
      retryAfterMs: RETRY_AFTER_SHORT_MS,
    });
  });

  it("тело не JSON не ломает разбор", async () => {
    // Arrange
    const response = new Response("<html>oops</html>", { status: STATUS.unavailable });

    // Act
    const problem = await problemFromResponse(response);

    // Assert
    expect(problem).toBeInstanceOf(ApiProblem);
    expect(problem.status).toBe(STATUS.unavailable);
  });
});
