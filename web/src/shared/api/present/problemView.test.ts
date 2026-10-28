import { afterEach, describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { ApiProblem } from "../http/problem";
import { problemViewOf } from "./problemView";

const DOWN = { badGateway: 502, unavailable: 503, gatewayTimeout: 504 };

const STATUS = {
  unauthorized: 401,
  forbidden: 403,
  notFound: 404,
  serverError: 500,
} as const;

const HOME: Action = { label: "Домой", run: { href: "/" } };
const RETRY: Action = { label: "Ещё раз", run: { onSelect: () => undefined } };
const SIGN_IN: Action = { label: "Войти", run: { href: "/login" } };

function problem(status: number, extra: Partial<ConstructorParameters<typeof ApiProblem>[0]> = {}) {
  return new ApiProblem({
    kind: "problem",
    status,
    title: "Сбой сервера",
    requestId: "req-1",
    ...extra,
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("problemViewOf", () => {
  it("planned несёт версию замены", () => {
    // Arrange
    const planned = new ApiProblem({ kind: "planned", status: 0, replacedIn: "v0.4.0" });

    // Act
    const view = problemViewOf(planned);

    // Assert
    expect(view).toEqual({ kind: "planned", replacedIn: "v0.4.0" });
  });

  it("planned без версии не теряет вид: версия пустая", () => {
    // Arrange
    const planned = new ApiProblem({ kind: "planned", status: 0 });

    // Act
    const view = problemViewOf(planned);

    // Assert
    expect(view).toStrictEqual({ kind: "planned", replacedIn: "" });
  });

  it("401 с действием входа — session с причиной и этим действием", () => {
    // Arrange
    const expired = new ApiProblem({
      kind: "unauthenticated",
      status: STATUS.unauthorized,
      reason: "session_expired",
    });

    // Act
    const view = problemViewOf(expired, { signIn: SIGN_IN });

    // Assert
    expect(view).toStrictEqual({ kind: "session", signIn: SIGN_IN, reason: "session_expired" });
  });

  it("неизвестная причина 401 не попадает в вид", () => {
    // Arrange
    const odd = new ApiProblem({
      kind: "unauthenticated",
      status: STATUS.unauthorized,
      reason: "reserved_internal_name",
    });

    // Act
    const view = problemViewOf(odd, { signIn: SIGN_IN });

    // Assert
    expect(view).toStrictEqual({ kind: "session", signIn: SIGN_IN });
  });

  it("401 без действия входа — версия, в которой вход появится, и никакой причины", () => {
    // Arrange
    const expired = new ApiProblem({
      kind: "unauthenticated",
      status: STATUS.unauthorized,
      reason: "session_expired",
    });

    // Act
    const view = problemViewOf(expired);

    // Assert
    expect(view).toStrictEqual({ kind: "session", availableIn: "v0.4.0" });
  });

  it("404 — нейтральный not-found с идентификатором запроса и «На главную»", () => {
    // Arrange
    const missing = problem(STATUS.notFound, { ruleId: "не должно попасть" });

    // Act
    const view = problemViewOf(missing, { home: HOME });

    // Assert
    expect(view).toEqual({ kind: "not-found", requestId: "req-1", home: HOME });
  });

  it("403 с правилом — forbidden с правилом, шагами и контактом", () => {
    // Arrange
    const denied = problem(STATUS.forbidden, {
      ruleId: "repo.deny",
      remediation: ["Шаг"],
      contact: "Петрова А.",
    });

    // Act
    const view = problemViewOf(denied, { home: HOME });

    // Assert
    expect(view).toEqual({
      kind: "forbidden",
      ruleId: "repo.deny",
      requestId: "req-1",
      remediation: ["Шаг"],
      contact: "Петрова А.",
      home: HOME,
    });
  });

  it("403 без правила не выдаётся за отказ по правилу", () => {
    // Arrange
    const denied = problem(STATUS.forbidden);

    // Act
    const view = problemViewOf(denied);

    // Assert
    expect(view.kind).toBe("error");
  });

  it("500 — error с текстом сервера, идентификатором и «Повторить»", () => {
    // Arrange
    const failure = problem(STATUS.serverError, {
      detail: "Хранилище недоступно",
      remediation: ["Подождите"],
    });

    // Act
    const view = problemViewOf(failure, { retry: RETRY });

    // Assert
    expect(view).toEqual({
      kind: "error",
      title: "Сбой сервера",
      detail: "Хранилище недоступно",
      requestId: "req-1",
      remediation: ["Подождите"],
      retry: RETRY,
    });
  });

  it("ответ без title получает локальный заголовок вместо пустого", () => {
    // Arrange
    const bare = problem(STATUS.serverError, { title: "" });

    // Act
    const view = problemViewOf(bare);

    // Assert
    expect(view).toMatchObject({ kind: "error", title: "Не удалось выполнить запрос" });
  });

  it.each(Object.values(DOWN))("%i — сервер CRANE недоступен", (status) => {
    // Arrange
    const down = problem(status);

    // Act
    const view = problemViewOf(down, { retry: RETRY });

    // Assert
    expect(view).toEqual({ kind: "offline", cause: "server", retry: RETRY });
  });

  it("сбой сети при онлайн-браузере — сервер недоступен", () => {
    // Arrange
    const network = new ApiProblem({ kind: "network", status: 0 });

    // Act
    const view = problemViewOf(network);

    // Assert
    expect(view).toEqual({ kind: "offline", cause: "server" });
  });

  it("step-up и csrf не имеют своего вида: обычная ошибка", () => {
    // Arrange
    const stepUp = new ApiProblem({
      kind: "step-up",
      status: STATUS.unauthorized,
      title: "MFA",
      requiredAcr: "mfa",
    });

    // Act
    const view = problemViewOf(stepUp);

    // Assert
    expect(view).toMatchObject({ kind: "error", title: "MFA" });
  });

  it("сбой сети без интернета у пользователя — «Нет соединения» (network)", () => {
    // Arrange
    vi.spyOn(navigator, "onLine", "get").mockReturnValue(false);
    const network = new ApiProblem({ kind: "network", status: 0 });

    // Act
    const view = problemViewOf(network);

    // Assert
    expect(view).toStrictEqual({ kind: "offline", cause: "network" });
  });

  it("ответ 503 при отключённой сети браузера всё равно «сервер недоступен»", () => {
    // Arrange
    vi.spyOn(navigator, "onLine", "get").mockReturnValue(false);
    const down = problem(DOWN.unavailable);

    // Act
    const view = problemViewOf(down);

    // Assert
    expect(view).toStrictEqual({ kind: "offline", cause: "server" });
  });

  it("ошибка без деталей и идентификатора не несёт этих ключей", () => {
    // Arrange
    const failure = problem(STATUS.serverError, { title: "Сбой", requestId: undefined });

    // Act
    const view = problemViewOf(failure);

    // Assert
    expect(view).toStrictEqual({ kind: "error", title: "Сбой", remediation: [] });
  });

  it("правило в ошибке сохраняется", () => {
    // Arrange
    const failure = problem(STATUS.serverError, { ruleId: "limits.artifact.size" });

    // Act
    const view = problemViewOf(failure);

    // Assert
    expect(view).toMatchObject({ kind: "error", ruleId: "limits.artifact.size" });
  });

  it("403 с правилом, но без идентификатора запроса — обычная ошибка", () => {
    // Arrange
    const denied = problem(STATUS.forbidden, { ruleId: "repo.deny", requestId: undefined });

    // Act
    const view = problemViewOf(denied);

    // Assert
    expect(view.kind).toBe("error");
  });

  it("404 без действия «На главную» — обычная ошибка, а не not-found без выхода", () => {
    // Arrange
    const missing = problem(STATUS.notFound);

    // Act
    const view = problemViewOf(missing);

    // Assert
    expect(view.kind).toBe("error");
  });

  it("404 без идентификатора запроса — not-found без этого ключа", () => {
    // Arrange
    const missing = problem(STATUS.notFound, { requestId: undefined });

    // Act
    const view = problemViewOf(missing, { home: HOME });

    // Assert
    expect(view).toStrictEqual({ kind: "not-found", home: HOME });
  });

  it("403 без контакта — forbidden без ключа contact", () => {
    // Arrange
    const denied = problem(STATUS.forbidden, { ruleId: "repo.deny" });

    // Act
    const view = problemViewOf(denied);

    // Assert
    expect(view).toStrictEqual({
      kind: "forbidden",
      ruleId: "repo.deny",
      requestId: "req-1",
      remediation: [],
    });
  });

  it("ошибка csrf показывается как обычная ошибка", () => {
    // Arrange
    const csrf = new ApiProblem({ kind: "csrf", status: STATUS.forbidden, title: "Токен устарел" });

    // Act
    const view = problemViewOf(csrf);

    // Assert
    expect(view).toMatchObject({ kind: "error", title: "Токен устарел" });
  });
});
