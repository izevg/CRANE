#!/usr/bin/env python3
"""Оценка трудоёмкости CRANE в агент-часах по замеру эталонного проекта.

Эталон замерен по git-истории и файлам (непустые строки, без lock-файлов,
сгенерированного кода, импортированных спецификаций и бинарных файлов).
Запуск: python3 tools/estimate-agent-hours.py [--start ГГГГ-ММ-ДД] [--holidays ГГГГ-ММ-ДД ...] [--gantt]
С --start печатает календарь релизов по трём сценариям (рабочие дни пн–пт, нерабочие даты задаются --holidays).
С --gantt дополнительно печатает диаграмму Ганта (Mermaid) для базового сценария.
Работы «после v1» считаются в итоге, но в календарь и диаграмму не попадают.
"""

import argparse
import datetime as dt
import math

# ---------- эталон: замер ----------
REF = {
    "product_kloc": 71.5,       # код продукта: backend 34.2 + агент 6.2 + SPA 29.4 (без генерации) + mock 1.7
    "tests_kloc": 93.0,
    "deploy_kloc": 25.8,        # Helm, CI, compose, правила алертов, конфигурация
    "docs_kloc": 38.2,          # авторская документация (без импортированных спецификаций)
    "agent_wall_h": (40, 47),   # активное время агентов по коммитам (сессии с разрывом ≤ 30–45 мин)
    "parallel": 3.8,            # среднее число одновременно активных веток за 30-минутное окно
    "operator_h": 20,           # внимание оператора
}
REF_AGENT_H = sum(REF["agent_wall_h"]) / 2 * REF["parallel"]            # ≈ 165 агент-ч
RATE = REF_AGENT_H / REF["product_kloc"]                               # агент-ч на KLOC кода продукта, полный цикл
OPERATOR_RATIO = REF["operator_h"] / REF_AGENT_H                       # ≈ 0.12

# Нагрузка и пентест требуют опубликованного проекта и реальной нагрузки (решение владельца 27.10.2026):
# часы входят в итог, но в календарь и релизы не ставятся.
POST_V1 = "после v1"

# ---------- CRANE: модули ----------
# (модуль, этап, KLOC кода продукта, язык, сложность относительно эталона)
MODULES = [
    ("Ядро: домен, порты, use cases, HTTP, очередь, outbox, миграции", "P0", 12, "rust", 1.0),
    ("Протоколы: npm, PyPI, Maven, NuGet, Cargo (sparse)", "P0", 24, "rust", 1.6),
    ("CAS, атомарная публикация, GC, retention, целостность", "P0", 6, "rust", 1.3),
    ("Proxy: TTL, SWR, singleflight, SSRF guard", "P0", 5, "rust", 1.4),
    ("Доступ: OIDC, токены crp/crr/crci, обмен, Cedar, роли, SoD, эпохи", "P0", 7, "rust", 1.3),
    ("Наблюдаемость: метрики, трассы, health, алерты", "P0", 3, "rust", 0.8),
    ("Аудит скачиваний: поток download_event, партиции, часовой дайджест, запрос «кто скачал»", "P0", 1.5, "rust", 1.1),
    # чистая проверка конфигурации: без проверки бакета через S3 API и ключа-подтверждения (решение владельца 27.10.2026)
    ("Проверка безопасной конфигурации при старте", "P0", 0.5, "rust", 1.0),
    ("SPA: каркас, каталог, карточка, администрирование", "P0", 14, "ts", 1.0),
    ("CLI: login, setup шести клиентов, NuGet и Cargo credential provider, robots, publish", "P0", 3, "rust", 0.9),
    ("GitLab workload identity, trust policy, provenance", "P1", 2, "rust", 1.3),
    # ~0,5 KLOC: таблица evidence и миграция, запись provenance из claims при publish из CI,
    # чтение для карточки версии. Остальные виды (SBOM, сканы) остаются в P2 и миграции не требуют.
    ("Evidence: таблица и provenance из claims", "P1", 0.5, "rust", 1.0),
    ("Диагностика отказов, CI templates", "P1", 2, "rust", 1.0),
    ("Импорт из Nexus", "P1", 2, "rust", 1.0),
    ("Классы хранения, размещение, онлайн-перенос", "P1", 2, "rust", 1.3),
    ("CLI: diag, warm, readiness, export", "P1", 2, "rust", 0.9),
    ("SPA: рабочие столы ролей, диагностика", "P1", 7, "ts", 1.0),
    ("Пакет для аудитора: отчёт о пересмотре доступа", "P1", 1, "rust", 1.0),
    ("Evidence, SBOM, Trivy, песочница анализа", "P2", 6.5, "rust", 1.3),
    # ~3 KLOC: три источника upstream (npm, PyPI, Maven) ~0,8 + проверка bundle, DSSE, цепочки Fulcio
    # и inclusion proof Rekor ~1 + trust root по TUF и офлайн-режим ~0,5 + PGP для Maven ~0,3
    # + приём из CI, хранение, gate ~0,4. Сложность 1,4 как у уязвимостей: криптография, три формата,
    # офлайн-режим, а ошибка ведёт к ложным блокировкам.
    # Источники: docs.npmjs.com/generating-provenance-statements, peps.python.org/pep-0740 и docs.pypi.org/api/integrity,
    # central.sonatype.org (новость 2025-01-28: Sigstore опционален, PGP обязателен), docs.sigstore.dev/about/bundle,
    # slsa.dev/spec/v1.0/levels.
    ("Attestations: Sigstore, in-toto/SLSA, проверка upstream", "P2", 3, "rust", 1.4),
    ("Лицензии: SPDX, политика, вирусность", "P2", 4, "rust", 1.2),
    ("Promotion, approvals, исключения", "P2", 4, "rust", 1.0),
    ("Уязвимости: фиды, сопоставление, P0–P4, VEX, экстренный режим", "P2", 9, "rust", 1.4),
    ("Скоринг systemone", "P2", 2, "rust", 1.0),
    ("События безопасности: OCSF, SIEM, Sigma, SOAR", "P2", 5, "rust", 1.1),
    ("Журнал действий: маскирование, раскрытие, экспорт, аудит доступа", "P2", 2, "rust", 1.0),
    ("Выгрузка пакета для аудитора", "P2", 0.5, "rust", 1.0),
    ("Матрица ответственности OSCAL", "P2", 0.5, "rust", 1.0),
    ("SPA: рабочие столы AppSec и SecOps, журнал действий, «Мои действия»", "P2", 11, "ts", 1.0),
    ("Граф потребителей и impact", "P3", 4, "rust", 1.2),
    ("Поиск и LLM: FTS, pgvector, RAG, rerank", "P3", 4, "rust", 1.0),
    ("A2A-агент", "P3", 3, "rust", 0.8),
    ("SPA: AI-поиск, ассистенты, граф", "P3", 6, "ts", 1.0),
]
LANG = {"rust": {"low": 1.3, "base": 1.6, "high": 2.0}, "ts": {"low": 1.0, "base": 1.0, "high": 1.1}}
# работы, которые не пропорциональны объёму кода (агент-ч)
EXTRA = [
    ("Матрица реальных клиентов: npm, pnpm, yarn, pip, uv, poetry, Maven, Gradle, dotnet, nuget, cargo", "P0", 44),
    ("Интеграция с реальными Keycloak, GitLab, Trivy, S3 на стенде", "P1", 30),
    ("Нагрузочный стенд профиля L и оптимизация горячего пути", POST_V1, 25),
    # регрессия, а не ёмкость: ёмкость требует целевого железа и реальных VU, поэтому идёт после v1
    ("Локальная проверка регрессии производительности: criterion, oha-смоук", "P0", 8),
    ("Безопасность релизов CRANE: SLSA provenance, cosign, SBOM, CSAF/VEX", "P0", 12),
    ("Исправление замечаний пентеста", POST_V1, 30),
    ("Подключение SIEM и SOAR на стенде", "P2", 15),
]
# этапы календаря, которые не ускоряются агентами (рабочие дни)
CALENDAR = [
    ("Нагрузочный PoC на целевом железе", POST_V1, 3),
    ("Пентест (Strix) на реальном корпоративном реестре", POST_V1, 7),
    ("Release candidate: стабилизация и обратная связь", "P1", 5),  # допущение, уточняется по ходу
]
SCENARIO_COMPLEXITY = {"low": 0.9, "base": 1.0, "high": 1.25}
PARALLEL = 4             # одновременно работающих агентов
AGENT_H_PER_DAY = 8      # часов работы агентов в рабочий день
OPERATOR_FACTOR = 1.5    # Rust и безопасность требуют больше ревью, чем эталон


# ---------- релизы: какие модули входят в какую версию ----------
# Замер v0.1.0: закрыта 2026-10-27 (PR #8–#16, ~4 ч календарного времени, 2–4 лида параллельно).
# Код продукта 5,1 тыс. непустых строк (Rust 5,0 + SQL 0,07); хвосты #[cfg(test)] и tests/ — тесты, 6,3 тыс.
# Доли = построено / объём модуля: ядро ≈ 4,2/12, CAS (objstore) ≈ 0,17/6, Cargo ≈ 0,7/24.
# Модель на этот объём даёт ≈ 21 агент-ч (base), замер 15–25 (время исполнителей не журналировалось):
# норма не опровергнута (замер грубый: разброс −29%…+19% шире порога 20%), не пересчитывается. Ошибка была в разбивке: план считал,
# что v0.1.0 строит модули ядра и CAS целиком. Остаток сложен в v0.5.0 «Полное ядро» допущением: на дату конца P0
# он не влияет, а даты v0.2.0–v0.4.0 оптимистичны, потому что часть остатка (namespace,
# журнал действий, owners) стоит в них. Объёмы модулей не уменьшены: Cargo — самый простой протокол,
# работа с реальными клиентами впереди.
# (версия, этап, название, [(префикс модуля или работы, доля)])
RELEASES = [
    ("v0.1.0", "P0", "Каркас и dogfood: Cargo hosted",
     [("Ядро:", 0.35), ("CAS", 0.03), ("Протоколы:", 0.03)]),
    ("v0.2.0", "P0", "Proxy-движок, proxy crates.io, npm",
     [("Proxy:", 1), ("Протоколы:", 0.2)]),
    ("v0.3.0", "P0", "PyPI, Maven, NuGet",
     [("Протоколы:", 0.6)]),
    ("v0.4.0", "P0", "Доступ, наблюдаемость, базовый UI, CLI",
     [("Доступ:", 1), ("Наблюдаемость:", 1), ("SPA: каркас", 1), ("CLI: login", 1), ("Аудит скачиваний", 1)]),
    ("v0.5.0", "P0", "Полное ядро: матрица клиентов, поставка",
     [("Матрица реальных клиентов", 1), ("Локальная проверка регрессии", 1),
      ("Ядро:", 0.65), ("CAS", 0.97), ("Протоколы:", 0.17), ("Проверка безопасной конфигурации", 1), ("Безопасность релизов", 1)]),
    ("v0.6.0", "P1", "Удобство: GitLab, диагностика, классы хранения, импорт из других реестров",
     [("GitLab", 1), ("Evidence: таблица", 1), ("Диагностика", 1), ("Импорт", 1), ("Классы", 1), ("CLI: diag", 1),
      ("SPA: рабочие столы ролей", 1), ("Интеграция с реальными", 1), ("Пакет для аудитора", 1)]),
    ("v1.1.0", "P2", "Безопасность цепочки поставки",
     [("Evidence, SBOM", 1), ("Attestations", 1), ("Лицензии", 1), ("Promotion", 1), ("Уязвимости", 1), ("Скоринг", 1),
      ("События безопасности", 1), ("Журнал действий", 1), ("SPA: рабочие столы AppSec", 1),
      ("Подключение SIEM", 1), ("Выгрузка пакета", 1), ("Матрица ответственности", 1)]),
    ("v1.2.0", "P3", "Развитие: граф, поиск и LLM, A2A",
     [("Граф", 1), ("Поиск", 1), ("A2A", 1), ("SPA: AI", 1)]),
    (POST_V1, POST_V1, "Проверка сообществом и на реальной нагрузке",
     [("Нагрузочный стенд профиля L", 1), ("Исправление замечаний пентеста", 1)]),
]
# выпущенные версии ставятся фактической датой, календарь от --start начинается со следующей
DONE = {"v0.1.0": dt.date(2026, 10, 27)}
# нерабочие даты сверх выходных; задаются через --holidays
HOLIDAYS = set()


def estimate(scn):
    by_stage = {}
    for name, stage, kloc, lang, k in MODULES:
        h = kloc * RATE * k * LANG[lang][scn] * SCENARIO_COMPLEXITY[scn]
        by_stage[stage] = by_stage.get(stage, 0) + h
    for name, stage, h in EXTRA:
        by_stage[stage] = by_stage.get(stage, 0) + h * SCENARIO_COMPLEXITY[scn]
    return by_stage


def main():
    print(f"Эталон: {REF_AGENT_H:.0f} агент-ч, {RATE:.2f} агент-ч/KLOC, оператор {OPERATOR_RATIO:.2f} ч на агент-ч")
    rust = sum(m[2] for m in MODULES if m[3] == "rust"); ts = sum(m[2] for m in MODULES if m[3] == "ts")
    print(f"CRANE: Rust {rust:g} KLOC, TypeScript {ts} KLOC")
    for scn in ("low", "base", "high"):
        st = estimate(scn); total = sum(st.values())
        print(f"\n[{scn}] всего {total:.0f} агент-ч (из них {POST_V1}: {st[POST_V1]:.0f}), "
              f"оператор {total * OPERATOR_RATIO * OPERATOR_FACTOR:.0f} ч")
        for s in ("P0", "P1", "P2", "P3", POST_V1):
            dev_days = st[s] / (PARALLEL * AGENT_H_PER_DAY)
            tail = ""
            if s == POST_V1:
                tail = f", календарь {sum(d for _, cs, d in CALENDAR if cs == s)} раб. дн., вне графика"
            print(f"  {s}: {st[s]:5.0f} агент-ч, разработка {dev_days:4.1f} раб. дн., "
                  f"оператор {st[s] * OPERATOR_RATIO * OPERATOR_FACTOR:4.0f} ч{tail}")
    st = estimate("base")
    print("\nМодули (base):")
    for name, stage, kloc, lang, k in MODULES:
        print(f"  {stage} {name}: {kloc} KLOC {lang} ×{k} → {kloc * RATE * k * LANG[lang]['base']:.0f} агент-ч")


def work_hours(scn):
    """Агент-часы каждого модуля и внемодульной работы в сценарии."""
    hours = {}
    for name, _, kloc, lang, k in MODULES:
        hours[name] = kloc * RATE * k * LANG[lang][scn] * SCENARIO_COMPLEXITY[scn]
    for name, _, h in EXTRA:
        hours[name] = h * SCENARIO_COMPLEXITY[scn]
    return hours


def release_hours(scn):
    hours = work_hours(scn)
    used = {name: 0.0 for name in hours}
    result = []
    for version, stage, title, parts in RELEASES:
        total = 0.0
        for prefix, share in parts:
            matches = [n for n in hours if n.startswith(prefix)]
            assert len(matches) == 1, f"{version}: префикс «{prefix}» → {matches}"
            total += hours[matches[0]] * share
            used[matches[0]] += share
        result.append((version, stage, title, total))
    unassigned = [n for n, u in used.items() if abs(u - 1) > 1e-9]
    assert not unassigned, f"модули не распределены по релизам ровно один раз: {unassigned}"
    return result


def workday(start, offset):
    """Дата рабочего дня с номером offset (0 — первый рабочий день не раньше start)."""
    day = start
    while day.weekday() >= 5 or day in HOLIDAYS:
        day += dt.timedelta(days=1)
    for _ in range(offset):
        day += dt.timedelta(days=1)
        while day.weekday() >= 5 or day in HOLIDAYS:
            day += dt.timedelta(days=1)
    return day


def schedule(scn, start):
    """Календарь релизов. Время — в рабочих днях от start (дробное для работы агентов)."""
    per_day = PARALLEL * AGENT_H_PER_DAY
    cal = {name: days for name, _, days in CALENDAR}
    rel = {v: (stage, title, h) for v, stage, title, h in release_hours(scn)}
    rows, t = [], 0.0

    def dev(version, begin):
        stage, title, h = rel[version]
        end = begin + h / per_day
        return title, h, begin, end

    def row(version, title, h, begin, end, note=""):
        rows.append((version, title, h, workday(start, math.floor(begin)),
                     workday(start, max(math.ceil(end) - 1, 0)), note))

    for v in ("v0.1.0", "v0.2.0", "v0.3.0", "v0.4.0", "v0.5.0"):
        if v in DONE:
            rows.append((v, rel[v][1], rel[v][2], DONE[v], DONE[v], "готово"))
            continue
        title, h, b, t = dev(v, t)
        row(v, title, h, b, t)
    title, h, b, p1 = dev("v0.6.0", t)
    row("v0.6.0", title, h, b, p1)
    ga = math.ceil(p1) + cal["Release candidate: стабилизация и обратная связь"]
    row("v1.0.0", "Релиз 1.0 (P0 + P1)", 0, p1, ga, "release candidate")
    title, h, b, p2 = dev("v1.1.0", p1)
    end = max(p2, ga)  # 1.1.0 выходит не раньше 1.0.0: порядок версий
    row("v1.1.0", title, h, b, end)
    title, h, b, p3 = dev("v1.2.0", p2)
    row("v1.2.0", title, h, b, max(p3, end))
    return rows


def print_schedule(start):
    for scn in ("low", "base", "high"):
        print(f"\nКалендарь [{scn}] от {start.isoformat()} (4 агента × 8 ч, рабочие дни пн–пт):")
        for version, title, h, b, e, note in schedule(scn, start):
            hrs = f"{h:4.0f} агент-ч" if h else " " * 11
            tail = f"  ({note})" if note else ""
            print(f"  {version:7} {b.isoformat()} → {e.isoformat()}  {hrs}  {title}{tail}")


def print_gantt(start):
    """Диаграмма Ганта (Mermaid) для базового сценария."""
    sections = {"P0": "P0 ядро", "P1": "P1 удобство", "P2": "P2 безопасность", "P3": "P3 развитие"}
    stage = {v: st for v, st, _, _ in RELEASES}
    stage["v1.0.0"] = "P1"
    print("```mermaid")
    print('%%{init: {"gantt": {"fontSize": 14, "sectionFontSize": 14, "barHeight": 26, "barGap": 6, "leftPadding": 150}}}%%')
    print("gantt\n  todayMarker off\n  dateFormat YYYY-MM-DD\n  axisFormat %d.%m\n  excludes weekends")
    current = None
    for version, title, _, b, e, note in schedule("base", start):
        if stage[version] != current:
            current = stage[version]
            print(f"  section {sections[current]}")
        label = f"{version} {title.split(':')[0]}".replace(":", " ").replace("(", "").replace(")", "")
        days, day = 0, b
        while day <= e:  # Mermaid с excludes weekends считает только рабочие дни
            days += day.weekday() < 5 and day not in HOLIDAYS
            day += dt.timedelta(days=1)
        tag = "done, " if version in DONE else ""
        print(f"  {label} :{tag}{b.isoformat()}, {max(days, 1)}d")
    print("```")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--start", type=dt.date.fromisoformat, help="дата старта, ГГГГ-ММ-ДД")
    parser.add_argument("--holidays", type=dt.date.fromisoformat, nargs="*", default=[],
                        help="нерабочие даты сверх выходных")
    parser.add_argument("--gantt", action="store_true", help="диаграмма Ганта (Mermaid), нужен --start")
    args = parser.parse_args()
    HOLIDAYS.update(args.holidays)
    main()
    if args.start:
        print_schedule(args.start)
        if args.gantt:
            print_gantt(args.start)
