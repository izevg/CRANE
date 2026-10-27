// Сценарий hosted: публикация, dist-теги, deprecate, установка, unpublish по политике. Сценарий proxy:
// установка пакета с npmjs дважды (промах, затем попадание). Клиент и разбор вывода npm - в npm.ts.
import { accessLines, type Expected, egressLines } from "./audit.ts";
import { type Result, tail } from "./exec.ts";
import {
  HOSTED_REPO,
  hasDeprecationWarning,
  type NpmClient,
  npmPurl,
  npmrc,
  packageJson,
  repoUrl,
  sameTags,
  TOKEN_ENV,
  upstreamFetches,
  versionsOf,
} from "./npm.ts";
import { expectCache } from "./proxy.ts";
import { npmProxyRepo, type Stand } from "./stand.ts";

const HTTP_CONFLICT = 409;
const SCOPE = "@dogfood";
const MESSAGE = "dogfood: use 2.x";
const CONSTANT = "crane-npm-dogfood";
const FIRST = "1.0.0";
const BETA = "1.1.0-beta.0";
const PROXY_PACKAGE = "is-number";
const PROXY_VERSION = "7.0.0";
const PROXY_PACKUMENT = new RegExp(`^/npm/${npmProxyRepo}/${PROXY_PACKAGE}$`);
const PROXY_TARBALL = new RegExp(
  `^/npm/${npmProxyRepo}/${PROXY_PACKAGE}/-/${PROXY_PACKAGE}-7\\.0\\.0\\.tgz$`,
);
const PROXY_TARBALL_PATH = `${PROXY_PACKAGE}/-/${PROXY_PACKAGE}-${PROXY_VERSION}.tgz`;

/** Всё, что нужно шагам сценария: запуск npm с токеном в окружении и проверка кода выхода. */
interface Tools {
  client: NpmClient;
  stand: Stand;
  agent: string;
  /** Выполняет `npm <args>` в /work/<dir>; код выхода не проверяет. */
  npm(dir: string, args: string[]): Result;
  /** То же, но ненулевой код — ошибка с хвостом stderr. */
  must(dir: string, args: string[]): Result;
  /** Новый пустой каталог кэша npm. */
  cache(): string;
}

function tools(stand: Stand, client: NpmClient): Tools {
  let caches = 0;
  const npm = (dir: string, args: string[]): Result =>
    client.exec(["npm", ...args], { cwd: `/work/${dir}`, env: { [TOKEN_ENV]: stand.token } });
  return {
    client,
    stand,
    agent: `npm/${client.version}`,
    npm,
    must(dir, args) {
      const result = npm(dir, args);
      if (result.status !== 0) {
        throw new Error(`npm ${args.join(" ")} -> ${result.status}\n${tail(result.stderr)}`);
      }
      return result;
    },
    cache() {
      caches += 1;
      return `/tmp/npm-cache-${caches}`;
    },
  };
}

const step = (name: string, body: () => void): void => {
  try {
    body();
  } catch (error) {
    throw new Error(`${name}: ${error instanceof Error ? error.message : String(error)}`);
  }
};

const expectEqual = (what: string, got: string, want: string): void => {
  if (got !== want) {
    throw new Error(`${what}: ожидалось «${want}», получено «${got}»`);
  }
};

/** Пакет в каталоге `dir`: манифест, index.js с константой и .npmrc реестра. */
function writePackage(t: Tools, dir: string, name: string, version: string, rc: string): void {
  t.client.write(`${dir}/package.json`, packageJson(name, version));
  t.client.write(`${dir}/index.js`, `module.exports = "${CONSTANT}";\n`);
  t.client.write(`${dir}/.npmrc`, rc);
}

const hostedRc = npmrc(repoUrl(HOSTED_REPO), { scope: SCOPE, registry: repoUrl(HOSTED_REPO) });

/** Шаги 1-4: публикация, теги, deprecate. Возвращает события аудита. */
function publishAndMark(t: Tools, pkg: string): Expected[] {
  const event = (operation: "publish" | "tag" | "untag" | "deprecate", purl: string): Expected => ({
    kind: "api",
    agent: t.agent,
    operation,
    purl,
  });
  const events: Expected[] = [];
  step("1 публикация 1.0.0 и 1.1.0-beta.0 --tag next", () => {
    writePackage(t, "pub", pkg, FIRST, hostedRc);
    t.must("pub", ["publish"]);
    writePackage(t, "pub", pkg, BETA, hostedRc);
    t.must("pub", ["publish", "--tag", "next"]);
    events.push(event("publish", npmPurl(pkg, FIRST)), event("publish", npmPurl(pkg, BETA)));
  });
  step("2 dist-tags пакета: latest и next", () => {
    const raw = t.must("pub", ["view", pkg, "dist-tags", "--json"]).stdout;
    if (!sameTags(raw, { latest: FIRST, next: BETA })) {
      throw new Error(`dist-tags: ${raw.trim()}`);
    }
  });
  step("3 dist-tag add, ls, rm", () => {
    t.must("pub", ["dist-tag", "add", `${pkg}@${FIRST}`, "stable"]);
    events.push(event("tag", npmPurl(pkg, FIRST)));
    const listed = t.must("pub", ["dist-tag", "ls", pkg]).stdout;
    if (!listed.includes(`stable: ${FIRST}`)) {
      throw new Error(`dist-tag ls не показал stable\n${tail(listed)}`);
    }
    t.must("pub", ["dist-tag", "rm", pkg, "stable"]);
    events.push(event("untag", npmPurl(pkg)));
    const after = t.must("pub", ["dist-tag", "ls", pkg]).stdout;
    if (after.includes("stable")) {
      throw new Error(`stable остался после rm\n${tail(after)}`);
    }
  });
  step("4 deprecate", () => {
    t.must("pub", ["deprecate", `${pkg}@${FIRST}`, MESSAGE]);
    events.push(event("deprecate", npmPurl(pkg, FIRST)));
    const shown = t.must("pub", ["view", `${pkg}@${FIRST}`, "deprecated"]).stdout.trim();
    expectEqual("deprecated", shown, MESSAGE);
  });
  return events;
}

/** Шаг 5: установка из пустого кэша, предупреждение deprecate, require возвращает константу. */
function installConsumer(t: Tools, pkg: string): void {
  step("5 установка 1.0.0 из пустого кэша", () => {
    t.client.write("consumer/package.json", packageJson("consumer", FIRST));
    t.client.write("consumer/.npmrc", hostedRc);
    const installed = t.must("consumer", [
      "install",
      `${pkg}@${FIRST}`,
      "--no-audit",
      "--cache",
      t.cache(),
    ]);
    if (!hasDeprecationWarning(installed.stderr, `${pkg}@${FIRST}`, MESSAGE)) {
      throw new Error(`нет предупреждения о deprecate\n${tail(installed.stderr)}`);
    }
    const printed = t.client.exec(["node", "-e", `console.log(require("${pkg}"))`], {
      cwd: "/work/consumer",
    });
    expectEqual("require", printed.stdout.trim(), CONSTANT);
  });
}

/** Шаг 6: unpublish в окне, номер версии не переиспользуется. */
function unpublish(t: Tools, pkg: string): Expected[] {
  const events: Expected[] = [];
  step("6 unpublish 1.1.0-beta.0 и отказ повторной публикации", () => {
    t.must("pub", ["unpublish", `${pkg}@${BETA}`]);
    events.push({ kind: "api", agent: t.agent, operation: "delete", purl: npmPurl(pkg, BETA) });
    const listed = versionsOf(t.must("pub", ["view", pkg, "versions", "--json"]).stdout);
    if (listed.includes(BETA)) {
      throw new Error(`версия осталась в списке: ${listed.join(", ")}`);
    }
    const again = t.npm("pub", ["publish", "--tag", "next"]);
    if (again.status === 0) {
      throw new Error("повторная публикация tombstone прошла");
    }
    events.push({
      kind: "api",
      agent: t.agent,
      operation: "publish",
      purl: npmPurl(pkg, BETA),
      outcome: "Failure",
    });
    const conflict = accessLines(t.stand.craneLogs()).some(
      (line) => line.status === HTTP_CONFLICT && line.path.endsWith(`lib-${t.stand.id}`),
    );
    if (!conflict) {
      throw new Error("в access-журнале нет 409 на повторную публикацию");
    }
  });
  return events;
}

/** Установка пакета npmjs через proxy: свежий кэш и node_modules на каждый прогон. */
function proxyInstall(t: Tools): void {
  const rc = npmrc(repoUrl(npmProxyRepo));
  const install = (run: number, extra: string[]): void => {
    const dir = `proxy-${run}`;
    t.client.write(`${dir}/package.json`, packageJson(`proxyapp-${run}`, FIRST));
    t.client.write(`${dir}/.npmrc`, rc);
    t.must(dir, ["install", `${PROXY_PACKAGE}@${PROXY_VERSION}`, "--cache", t.cache(), ...extra]);
    const present = t.client.exec(["test", "-f", `node_modules/${PROXY_PACKAGE}/package.json`], {
      cwd: `/work/${dir}`,
    });
    if (present.status !== 0) {
      throw new Error(`прогон ${run}: ${PROXY_PACKAGE} не установлен`);
    }
  };
  const seen = (): number => accessLines(t.stand.craneLogs()).length;
  const first = seen();
  // Первый прогон с аудитом: ответ 501 на audit не должен ронять установку.
  install(1, []);
  const second = seen();
  install(2, ["--no-audit"]);
  const lines = accessLines(t.stand.craneLogs());
  const watched = [PROXY_PACKUMENT, PROXY_TARBALL];
  expectCache(watched, lines.slice(first, second), "miss");
  expectCache(watched, lines.slice(second), "hit");

  const egress = egressLines(t.stand.craneLogs());
  for (const path of [PROXY_PACKAGE, PROXY_TARBALL_PATH]) {
    const fetches = upstreamFetches(egress, path);
    if (fetches !== 1) {
      throw new Error(`выборок из upstream для ${path}: ${fetches}, ожидалась ровно одна`);
    }
  }
  t.client.write("proxy-view/.npmrc", rc);
  const tarball = t
    .must("proxy-view", [
      "view",
      `${PROXY_PACKAGE}@${PROXY_VERSION}`,
      "dist.tarball",
      "--cache",
      t.cache(),
    ])
    .stdout.trim();
  if (!tarball.startsWith(repoUrl(npmProxyRepo))) {
    throw new Error(`dist.tarball указывает мимо proxy: ${tarball}`);
  }
}

/** Выполняет сценарий npm и возвращает события аудита, которые он обязан породить. */
export function npmScenario(stand: Stand, client: NpmClient): Expected[] {
  const t = tools(stand, client);
  const pkg = `${SCOPE}/lib-${stand.id}`;
  const events = publishAndMark(t, pkg);
  installConsumer(t, pkg);
  events.push(...unpublish(t, pkg));
  step("7 proxy npmjs: промах, затем попадание", () => {
    proxyInstall(t);
  });
  return events;
}
