#!/usr/bin/env python3
"""Оценка трудоёмкости CRANE в агент-часах по замеру эталонного проекта.

Эталон замерен по git-истории и файлам (непустые строки, без lock-файлов,
сгенерированного кода, импортированных спецификаций и бинарных файлов).
Запуск: python3 tools/estimate-agent-hours.py [--start ГГГГ-ММ-ДД]
С --start печатает календарь релизов (рабочие дни, праздники РФ) по трём сценариям.
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

# ---------- CRANE: модули ----------
# (модуль, этап, KLOC кода продукта, язык, сложность относительно эталона)
MODULES = [
    ("Ядро: домен, порты, use cases, HTTP, очередь, outbox, миграции", "P0", 12, "rust", 1.0),
    ("Протоколы: npm, PyPI, Maven, NuGet, Cargo (sparse)", "P0", 24, "rust", 1.6),
    ("CAS, атомарная публикация, GC, retention, целостность", "P0", 6, "rust", 1.3),
    ("Proxy: TTL, SWR, singleflight, SSRF guard", "P0", 5, "rust", 1.4),
    ("Доступ: OIDC, токены crp/crr/crci, обмен, Cedar, роли, SoD, эпохи", "P0", 7, "rust", 1.3),
    ("Наблюдаемость: метрики, трассы, health, алерты", "P0", 3, "rust", 0.8),
    ("SPA: каркас, каталог, карточка, администрирование", "P0", 14, "ts", 1.0),
    ("CLI: login, setup шести клиентов, NuGet и Cargo credential provider, robots, publish", "P0", 3, "rust", 0.9),
    ("GitLab workload identity, trust policy, provenance", "P1", 2, "rust", 1.3),
    ("Диагностика отказов, CI templates", "P1", 2, "rust", 1.0),
    ("Импорт из Nexus", "P1", 2, "rust", 1.0),
    ("Классы хранения, размещение, онлайн-перенос", "P1", 2, "rust", 1.3),
    ("CLI: diag, warm, readiness, export", "P1", 2, "rust", 0.9),
    ("SPA: рабочие столы ролей, диагностика", "P1", 7, "ts", 1.0),
    ("Evidence, SBOM, Trivy, песочница анализа", "P2", 7, "rust", 1.3),
    ("Лицензии: SPDX, политика, вирусность", "P2", 4, "rust", 1.2),
    ("Promotion, approvals, исключения", "P2", 4, "rust", 1.0),
    ("Уязвимости: фиды, сопоставление, P0–P4, VEX, экстренный режим", "P2", 9, "rust", 1.4),
    ("Скоринг systemone", "P2", 2, "rust", 1.0),
    ("События безопасности: OCSF, SIEM, Sigma, SOAR", "P2", 5, "rust", 1.1),
    ("Журнал действий: маскирование, раскрытие, экспорт, аудит доступа", "P2", 2, "rust", 1.0),
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
    ("Нагрузочный стенд и оптимизация горячего пути", "P0", 25),
    ("Исправление замечаний security review", "P2", 30),
    ("Подключение SIEM и SOAR пилотной организации", "P2", 15),
]
# этапы календаря, которые не ускоряются агентами (рабочие дни)
CALENDAR = [
    ("Нагрузочный PoC на целевом железе", "P0", 3),
    ("Security review и пентест", "P2", 7),
    ("Пилот с командами", "P1", 10),
    ("Миграция из Nexus волнами по командам", "P1", 10),
]
SCENARIO_COMPLEXITY = {"low": 0.9, "base": 1.0, "high": 1.25}
PARALLEL = 4             # одновременно работающих агентов
AGENT_H_PER_DAY = 8      # часов работы агентов в рабочий день
OPERATOR_FACTOR = 1.5    # Rust и безопасность требуют больше ревью, чем эталон


# ---------- релизы: какие модули входят в какую версию ----------
# (версия, этап, название, [(префикс модуля или работы, доля)])
RELEASES = [
    ("v0.1.0", "P0", "Каркас и dogfood: Cargo hosted",
     [("Ядро:", 1), ("CAS", 1), ("Протоколы:", 0.2)]),
    ("v0.2.0", "P0", "Proxy-движок, proxy crates.io, npm",
     [("Proxy:", 1), ("Протоколы:", 0.2)]),
    ("v0.3.0", "P0", "PyPI, Maven, NuGet",
     [("Протоколы:", 0.6)]),
    ("v0.4.0", "P0", "Доступ, наблюдаемость, базовый UI, CLI",
     [("Доступ:", 1), ("Наблюдаемость:", 1), ("SPA: каркас", 1), ("CLI: login", 1)]),
    ("v0.5.0", "P0", "Полное ядро: матрица клиентов, нагрузка, поставка",
     [("Матрица реальных клиентов", 1), ("Нагрузочный стенд", 1)]),
    ("v0.6.0", "P1", "Удобство: GitLab, диагностика, классы хранения, импорт из Nexus",
     [("GitLab", 1), ("Диагностика", 1), ("Импорт", 1), ("Классы", 1), ("CLI: diag", 1),
      ("SPA: рабочие столы ролей", 1), ("Интеграция с реальными", 1)]),
    ("v1.1.0", "P2", "Безопасность цепочки поставки",
     [("Evidence", 1), ("Лицензии", 1), ("Promotion", 1), ("Уязвимости", 1), ("Скоринг", 1),
      ("События безопасности", 1), ("Журнал действий", 1), ("SPA: рабочие столы AppSec", 1),
      ("Исправление замечаний", 1), ("Подключение SIEM", 1)]),
    ("v1.2.0", "P3", "Развитие: граф, поиск и LLM, A2A",
     [("Граф", 1), ("Поиск", 1), ("A2A", 1), ("SPA: AI", 1)]),
]
# производственный календарь РФ: нерабочие праздничные дни в горизонте плана
HOLIDAYS = {dt.date(2026, 11, 4)} | {dt.date(2027, 1, d) for d in range(1, 9)}


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
    print(f"CRANE: Rust {rust} KLOC, TypeScript {ts} KLOC")
    for scn in ("low", "base", "high"):
        st = estimate(scn); total = sum(st.values())
        cal = {s: d for _, s, d in CALENDAR}
        print(f"\n[{scn}] всего {total:.0f} агент-ч, оператор {total * OPERATOR_RATIO * OPERATOR_FACTOR:.0f} ч")
        for s in ("P0", "P1", "P2", "P3"):
            dev_days = st[s] / (PARALLEL * AGENT_H_PER_DAY)
            print(f"  {s}: {st[s]:5.0f} агент-ч, разработка {dev_days:4.1f} раб. дн., "
                  f"оператор {st[s] * OPERATOR_RATIO * OPERATOR_FACTOR:4.0f} ч")
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

    for v in ("v0.1.0", "v0.2.0", "v0.3.0", "v0.4.0"):
        title, h, b, t = dev(v, t)
        row(v, title, h, b, t)
    title, h, b, e = dev("v0.5.0", t)
    t = max(e, b + cal["Нагрузочный PoC на целевом железе"])
    row("v0.5.0", title, h, b, t, "нагрузочный PoC на железе идёт параллельно")
    title, h, b, p1 = dev("v0.6.0", t)
    row("v0.6.0", title, h, b, p1)
    ga = math.ceil(p1) + cal["Пилот с командами"]
    row("v1.0.0", "Промышленный релиз (P0 + P1 после пилота)", 0, p1, ga, "пилот с командами")
    title, h, b, p2 = dev("v1.1.0", p1)
    sec = max(math.ceil(p2) + cal["Security review и пентест"], ga)
    row("v1.1.0", title, h, b, sec, "security review и пентест")
    title, h, b, p3 = dev("v1.2.0", p2)
    row("v1.2.0", title, h, b, max(p3, sec))
    mig = ga + cal["Миграция из Nexus волнами по командам"]
    row("—", "Миграция из Nexus волнами по командам", 0, ga, mig, "после v1.0.0")
    return rows


def print_schedule(start):
    for scn in ("low", "base", "high"):
        print(f"\nКалендарь [{scn}] от {start.isoformat()} (4 агента × 8 ч, рабочие дни, праздники РФ):")
        for version, title, h, b, e, note in schedule(scn, start):
            hrs = f"{h:4.0f} агент-ч" if h else " " * 11
            tail = f"  ({note})" if note else ""
            print(f"  {version:7} {b.isoformat()} → {e.isoformat()}  {hrs}  {title}{tail}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--start", type=dt.date.fromisoformat, help="дата старта, ГГГГ-ММ-ДД")
    args = parser.parse_args()
    main()
    if args.start:
        print_schedule(args.start)
