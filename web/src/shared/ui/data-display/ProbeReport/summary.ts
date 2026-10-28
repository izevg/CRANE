export type ProbeStatus = "pass" | "warn" | "fail" | "skipped";

export type ProbeSummary = { pass: number; warn: number; fail: number };

/**
 * Сводка проверки по статусам. Пропущенные (`skipped`) не входят ни в одну графу: пропуск — не результат,
 * и «0 с ошибкой» рядом с пропущенной OIDC-проверкой не должно читаться как «всё хорошо».
 */
export function summarize(checks: readonly { status: ProbeStatus }[]): ProbeSummary {
  const summary: ProbeSummary = { pass: 0, warn: 0, fail: 0 };
  for (const { status } of checks) {
    if (status !== "skipped") {
      summary[status] += 1;
    }
  }
  return summary;
}
