import { useLingui } from "@lingui/react/macro";
import { ProblemState } from "~/shared/ui/status";

/**
 * 404 внутри шелла. Один экран и для адреса без маршрута, и для ресурса, который loader не нашёл: нейтральный,
 * без правила, контакта и «Запросить доступ» (тип `not-found` их не допускает).
 */
export function NotFoundPage() {
  const { t } = useLingui();
  return (
    <ProblemState
      headingLevel={1}
      problem={{ kind: "not-found", home: { label: t`На главную`, run: { href: "/" } } }}
    />
  );
}
