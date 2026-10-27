// Создаёт недостающие файлы секретов для compose: токен, пароль PostgreSQL, URL каталога.
// Идемпотентно: существующие файлы не перезаписываются. Значения нигде не печатаются.
// Использование: task secrets:init (пути и имена берутся из .env).
import { randomBytes } from "node:crypto";
import { chmodSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

type Env = Record<string, string | undefined>;

const SECRET_BYTES = 32;
// Каталог 0700, файлы 0644: на Linux секрет читают uid 65532 и postgres, а не владелец хоста.
const DIR_MODE = 0o700;
const FILE_MODE = 0o644;
const REQUIRED = [
  "CRANE_TOKEN_SECRET",
  "POSTGRES_PASSWORD_SECRET",
  "CRANE_DATABASE_URL_SECRET",
  "POSTGRES_USER",
  "POSTGRES_DB",
];
const randomHex = (): string => randomBytes(SECRET_BYTES).toString("hex");

/** Возвращает пути созданных файлов (без значений). */
export function initSecrets(env: Env, root: string, random = randomHex): string[] {
  const missing = REQUIRED.filter((name) => !env[name]);
  if (missing.length > 0) {
    throw new Error(`не заданы ${missing.join(", ")}: создайте .env (cp .env.dist .env)`);
  }
  const value = (name: string): string => env[name] ?? "";
  const created: string[] = [];
  /** Создаёт файл, только если его нет; возвращает содержимое (новое или существующее). */
  const ensure = (name: string, make: () => string): string => {
    const path = resolve(root, value(name));
    mkdirSync(dirname(path), { recursive: true, mode: DIR_MODE });
    try {
      const content = make();
      writeFileSync(path, content, { flag: "wx" });
      chmodSync(path, FILE_MODE);
      created.push(value(name));
      return content;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "EEXIST") {
        throw error;
      }
      return readFileSync(path, "utf8").trim();
    }
  };

  ensure("CRANE_TOKEN_SECRET", random);
  const password = ensure("POSTGRES_PASSWORD_SECRET", random);
  ensure(
    "CRANE_DATABASE_URL_SECRET",
    () => `postgres://${value("POSTGRES_USER")}:${password}@postgres:5432/${value("POSTGRES_DB")}`,
  );
  return created;
}

if (import.meta.main) {
  try {
    const created = initSecrets(process.env, process.cwd());
    process.stdout.write(
      created.length > 0 ? `секреты созданы: ${created.join(", ")}\n` : "секреты уже на месте\n",
    );
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
