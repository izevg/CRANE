import { ApiProblem } from "./problem";

type Reader = () => Promise<string | null>;

/**
 * Токен CSRF живёт мемоизированным промисом, а не в React-состоянии: мутации идут не только из компонентов.
 * Отклонённый промис не запоминается: временный сбой `GET /session` не должен отравить все следующие мутации.
 */
export function createCsrfSource(read: Reader): {
  token: () => Promise<string>;
  reset: () => void;
} {
  let memo: Promise<string> | undefined;

  async function load(): Promise<string> {
    const token = await read();
    if (token === null) {
      // сессии нет: мутация без неё всё равно получила бы 401, не тратим запрос
      throw new ApiProblem({ kind: "unauthenticated", status: 401 });
    }
    return token;
  }

  return {
    token() {
      memo ??= load().catch((error: unknown) => {
        memo = undefined;
        throw error;
      });
      return memo;
    },
    reset() {
      memo = undefined;
    },
  };
}
