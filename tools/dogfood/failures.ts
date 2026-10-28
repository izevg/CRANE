// Сценарии отказов на стенде: Valkey, PostgreSQL и S3 по очереди останавливаются, затем возвращаются.
// Источник истины — сервер: access-журнал, egress-журнал, журнал crane, аудит и число объектов бакета,
// а не вывод клиента. Чтения во время отказа идут через curl (один запрос — один ответ: cargo ретраит 5xx
// индекса и размножил бы запросы), публикация — через cargo publish (он не ретраит саму публикацию).
import {
  type Access,
  accessLines,
  type Egress,
  type Expected,
  egressLines,
  type Outcome,
} from "./audit.ts";
import {
  authorizedCurl,
  type CargoClient,
  cargoEnv,
  manifest,
  REGISTRY,
  startCargo,
} from "./cargo.ts";
import { MS_PER_SECOND, SECONDS_PER_MINUTE, tail, timed } from "./exec.ts";
import {
  checkCacheDegraded,
  checkDegradedFetch,
  checkNoEgress,
  checkNoServerErrors,
  checkServed,
  checkUnavailable,
} from "./failures-checks.ts";
import { indexPath } from "./hosted-wins.ts";
import type { Outage } from "./stand.outage.ts";
import { proxyRepo, type Stand } from "./stand.ts";

// Ключи proxy-репозитория proxyRepo(0): тёплый прогрет proxyScenario, холодные не встречаются ни в одном сценарии.
const WARM_INDEX = indexPath("form_urlencoded");
const WARM_CRATE = "api/v1/crates/form_urlencoded/1.2.1/download";
// Свой холодный ключ на вид отказа: ключ первого прогона во втором уже закэширован.
const VALKEY_COLD: Record<Outage, string> = { stop: indexPath("isatty"), pause: indexPath("libc") };
const POSTGRES_COLD = indexPath("urlencoding");
const FIRST_VERSION = "0.1.0";
const PUBLISH_PATH = "/api/v1/crates/new";
const PUBLISH_TIMEOUT_MINUTES = 5;
const PUBLISH_TIMEOUT_MS = PUBLISH_TIMEOUT_MINUTES * SECONDS_PER_MINUTE * MS_PER_SECOND;
const HOSTED = "/cargo/internal";

interface Context {
  stand: Stand;
  client: CargoClient;
  /** Префикс proxy-репозитория на crane: `/cargo/<repo>`. */
  base: string;
  /** Крейт, который сценарий публикует в hosted-репозиторий `internal`. */
  crate: string;
  events: Expected[];
  homes: number;
}

/** Журналы crane после отметки: строки access и egress, сырой NDJSON. */
interface Window {
  access: Access[];
  egress: Egress[];
  journal: string;
}

const journalLines = (text: string): string[] => text.split("\n").filter((l) => l.trim() !== "");

function mark(stand: Stand): () => Window {
  const before = stand.craneLogs();
  const seen = {
    access: accessLines(before).length,
    egress: egressLines(before).length,
    journal: journalLines(before).length,
  };
  return () => {
    const text = stand.craneLogs();
    return {
      access: accessLines(text).slice(seen.access),
      egress: egressLines(text).slice(seen.egress),
      journal: journalLines(text).slice(seen.journal).join("\n"),
    };
  };
}

/** Один запрос с токеном; статус смотрит сервер, поэтому здесь важно лишь, что curl дошёл до crane. */
function get(c: Context, path: string): void {
  const result = authorizedCurl(c.client, c.stand.token, [
    "-sS",
    "-o",
    "/dev/null",
    `https://localhost${path}`,
  ]);
  if (result.status !== 0) {
    throw new Error(`curl ${path} -> ${result.status}\n${tail(result.stderr)}`);
  }
}

const warmPaths = (c: Context): string[] => [
  `${c.base}/index/${WARM_INDEX}`,
  `${c.base}/${WARM_CRATE}`,
];

const hostedIndex = (c: Context): string => `${HOSTED}/index/${indexPath(c.crate)}`;

const forPaths = (lines: Access[], ...suffixes: string[]): Access[] =>
  lines.filter((line) => suffixes.some((suffix) => line.path.endsWith(suffix)));

function fail(what: string, problems: string[]): void {
  if (problems.length > 0) {
    throw new Error(`${what}: ${problems.join("; ")}`);
  }
}

/** `cargo publish` крейта `c.crate` версии `version`; код выхода возвращает вызывающему. */
function publish(c: Context, version: string): number {
  c.homes += 1;
  c.client.write(`${c.crate}/Cargo.toml`, manifest({ name: c.crate, version, files: {} }));
  c.client.write(
    `${c.crate}/src/lib.rs`,
    `pub fn version() -> &'static str {\n    "${version}"\n}\n`,
  );
  return c.client.exec(["cargo", "publish", "--registry", REGISTRY], {
    cwd: `/work/${c.crate}`,
    env: cargoEnv(`/tmp/failures-${c.homes}`, c.stand.token),
    timeoutMs: PUBLISH_TIMEOUT_MS,
  }).status;
}

function expectEvent(c: Context, version: string, outcome: Outcome): void {
  c.events.push({
    kind: "api",
    agent: `cargo/${c.client.version}`,
    operation: "publish",
    purl: `pkg:cargo/${c.crate}@${version}`,
    outcome,
  });
}

/** Публикация прошла: код 0 и событие Success в ожиданиях. */
function mustPublish(c: Context, version: string): void {
  const first = version === FIRST_VERSION;
  const status = publish(c, version);
  if (status !== 0) {
    throw new Error(`cargo publish ${c.crate}@${version} -> ${status}`);
  }
  // Первая публикация нового имени резервирует его (namespaces); у крейта вне `CRANE_NAMESPACES` — одно событие.
  if (first) {
    c.events.push({ kind: "reserve", agent: `cargo/${c.client.version}` });
  }
  expectEvent(c, version, "Success");
}

function expectObjects(c: Context, want: number, when: string): void {
  const got = c.stand.bucketObjects();
  fail(`бакет ${when}`, got === want ? [] : [`объектов ${got}, ожидалось ${want}`]);
}

/**
 * Valkey (`stop` или `pause`): тёплый ключ из L1, холодный идёт мимо L2 и координатора прямо в upstream; 5xx нет.
 * Журнал обязан содержать warn `valkey = "unavailable"` (смена доступности) и не содержать паник и ERROR.
 * Время каждого запроса печатается без порогов: это «до» для circuit breaker.
 */
function valkey(c: Context, kind: Outage): void {
  const cold = VALKEY_COLD[kind];
  const window = mark(c.stand);
  const during = c.stand.outage("valkey", kind, () => {
    timed(`Valkey ${kind}: тёплый ключ`, () => get(c, `${c.base}/index/${WARM_INDEX}`));
    timed(`Valkey ${kind}: холодный ключ`, () => get(c, `${c.base}/index/${cold}`));
    return window();
  });
  const lines = during.access.filter((line) => line.path.startsWith(`${c.base}/`));
  fail(`Valkey ${kind}: 5xx`, checkNoServerErrors(lines));
  fail(`Valkey ${kind}: тёплый ключ`, checkServed(forPaths(lines, `/index/${WARM_INDEX}`)));
  fail(
    `Valkey ${kind}: холодный ключ`,
    checkDegradedFetch(forPaths(lines, `/index/${cold}`), during.egress, cold),
  );
  fail(`Valkey ${kind}: журнал`, checkCacheDegraded(during.journal));
  // Valkey без персистентности: после возврата L2 пуст, это ожидаемо, полный сценарий греет свои ключи сам.
}

/** PostgreSQL: fail closed. Всё, что требует каталога, получает 503, upstream не трогается; публикации нет. */
function postgres(c: Context): void {
  for (const path of warmPaths(c)) {
    get(c, path);
  }
  const objects = c.stand.bucketObjects();
  const window = mark(c.stand);
  const during = timed("PostgreSQL остановлен: все запросы протокола", () =>
    c.stand.outage("postgres", "stop", () => {
      const ready = c.stand.ready();
      for (const path of [...warmPaths(c), `${c.base}/index/${POSTGRES_COLD}`, hostedIndex(c)]) {
        get(c, path);
      }
      return { ready, status: publish(c, "0.2.0"), ...window() };
    }),
  );
  fail("PostgreSQL: ready", during.ready ? ["/healthz/ready зелёный при остановленной БД"] : []);
  fail("PostgreSQL: публикация", during.status === 0 ? ["cargo publish прошёл"] : []);
  // config.json crane собирает сам, без каталога: он отвечает и при отказе.
  const lines = during.access.filter(
    (line) => line.path.startsWith("/cargo/") && !line.path.endsWith("/index/config.json"),
  );
  fail("PostgreSQL: ответы", checkUnavailable(lines));
  fail("PostgreSQL: upstream", checkNoEgress(during.egress));
  // cargo сначала читает индекс своего крейта; при 503 он останавливается до PUT, и аудита нет. Если же PUT
  // дошёл до сервера, он обязан дать Failure.
  if (forPaths(lines, PUBLISH_PATH).length > 0) {
    expectEvent(c, "0.2.0", "Failure");
  }
  expectObjects(c, objects, "после отказа PostgreSQL");
  timed("PostgreSQL вернулся: тот же ключ и повтор публикации", () => {
    const again = mark(c.stand);
    get(c, `${c.base}/index/${WARM_INDEX}`);
    fail(
      "PostgreSQL: после возврата",
      checkServed(forPaths(again().access, `/index/${WARM_INDEX}`)),
    );
    mustPublish(c, "0.2.0");
  });
  expectObjects(c, objects + 1, "после повтора публикации (не 409: строки не было)");
}

/** S3: чтение закэшированного, но без blob, и публикация дают 503; crane жив, после возврата повтор проходит. */
function s3(c: Context): void {
  // Прогрев до замера: запись Blob(digests) в кэше есть, байты лежат только в бакете.
  get(c, `${c.base}/${WARM_CRATE}`);
  const objects = c.stand.bucketObjects();
  const window = mark(c.stand);
  const during = timed("S3 остановлен: чтение blob и публикация", () =>
    c.stand.outage("seaweedfs", "stop", () => {
      get(c, `${c.base}/${WARM_CRATE}`);
      return { status: publish(c, "0.3.0"), ...window() };
    }),
  );
  fail("S3: публикация", during.status === 0 ? ["cargo publish прошёл"] : []);
  // Индекс при живой БД отвечает; 503 обязаны дать чтение blob и сам PUT, а он должен дойти до сервера.
  const lines = forPaths(during.access, `/${WARM_CRATE}`, PUBLISH_PATH);
  fail("S3: ответы", checkUnavailable(lines));
  fail(
    "S3: PUT публикации",
    forPaths(lines, PUBLISH_PATH).length > 0 ? [] : ["запрос не дошёл до сервера"],
  );
  expectEvent(c, "0.3.0", "Failure");
  // Считается только `blobs/`: хвосты `staging/` после оборванной записи не в счёт (их убирает lifecycle).
  expectObjects(c, objects, "после отказа S3");
  timed("S3 вернулся: повтор публикации", () => mustPublish(c, "0.3.0"));
  expectObjects(c, objects + 1, "после повтора публикации");
}

/** Выполняет сценарии отказов на клиенте `image` и возвращает события аудита, которые они обязаны породить. */
export function failuresScenario(stand: Stand, image: string): Expected[] {
  const client = startCargo(stand, image);
  try {
    const c: Context = {
      stand,
      client,
      base: `/cargo/${proxyRepo(0)}`,
      crate: `dff_${stand.id}`,
      events: [],
      homes: 0,
    };
    // Здоровая публикация нужна hosted-чтению во время отказа PostgreSQL: крейт обязан существовать.
    mustPublish(c, FIRST_VERSION);
    valkey(c, "stop");
    valkey(c, "pause");
    postgres(c);
    s3(c);
    return c.events;
  } finally {
    client.stop();
  }
}
