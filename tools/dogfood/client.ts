// Клиент стенда: долгоживущий контейнер в сетевом пространстве Caddy. Единственное место,
// где собирается `docker exec`; сценарии говорят на языке своего клиента (cargo, npm), а не docker.
import { type Options, type Result, run, runOk } from "./exec.ts";
import type { Stand } from "./stand.ts";

export interface ExecOptions {
  cwd?: string;
  /** Значения идут в окружение docker CLI, в argv попадают только имена: секреты не видны в `ps`. */
  env?: Record<string, string>;
  input?: string;
}

export interface Client {
  /** Выполняет команду в контейнере; код выхода не проверяет. */
  exec(cmd: string[], options?: ExecOptions): Result;
  /** Пишет файл (путь относительно /work), недостающие каталоги создаёт. */
  write(path: string, text: string): void;
  stop(): void;
}

/** Запускает контейнер `image` в сетевом пространстве Caddy; `env` — несекретные переменные. */
export function startClient(stand: Stand, image: string, env: Record<string, string>): Client {
  const container = runOk("docker", [
    "run",
    "-d",
    "--rm",
    "--label",
    `crane-dogfood=${stand.project}`,
    "--network",
    `container:${stand.caddy}`,
    "-v",
    `${stand.project}_caddy-data:/caddy:ro`,
    "-w",
    "/work",
    ...Object.entries(env).flatMap(([key, value]) => ["-e", `${key}=${value}`]),
    image,
    "sleep",
    "infinity",
  ]).trim();

  const exec = (cmd: string[], options: ExecOptions = {}): Result => {
    const env = options.env ?? {};
    const call: Options = { env };
    if (options.input !== undefined) {
      call.input = options.input;
    }
    return run(
      "docker",
      [
        "exec",
        ...(options.input === undefined ? [] : ["-i"]),
        ...(options.cwd === undefined ? [] : ["-w", options.cwd]),
        ...Object.keys(env).flatMap((key) => ["-e", key]),
        container,
        ...cmd,
      ],
      call,
    );
  };
  return {
    exec,
    write(path, text) {
      const result = exec(["install", "-D", "/dev/stdin", `/work/${path}`], { input: text });
      if (result.status !== 0) {
        throw new Error(`не записать ${path}: ${result.stderr.trim()}`);
      }
    },
    stop() {
      run("docker", ["rm", "-f", container]);
    },
  };
}
