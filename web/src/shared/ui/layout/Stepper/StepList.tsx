import { useLingui } from "@lingui/react/macro";
import { CircleCheckIcon, CircleDashedIcon, CircleIcon, type LucideIcon } from "lucide-react";
import { cx } from "~/shared/lib/core";
import { Link } from "../../primitives";

export type StepStatus = "todo" | "done" | "done_by_env" | "unavailable";

export type StepperStep = {
  id: string;
  label: string;
  status: StepStatus;
  /** Ссылкой шаг становится только с `href`: доступность шага решает вызывающий, а не кит. */
  href?: string;
};

const STATUS_ICON = {
  todo: CircleIcon,
  done: CircleCheckIcon,
  done_by_env: CircleCheckIcon,
  unavailable: CircleDashedIcon,
} satisfies Record<StepStatus, LucideIcon>;

const STATUS_COLOR = {
  todo: "text-muted-foreground",
  done: "text-success",
  done_by_env: "text-success",
  unavailable: "text-muted-foreground",
} satisfies Record<StepStatus, string>;

const ROW =
  "flex min-h-6 items-start gap-2 rounded-md px-2 py-1.5 text-body transition-colors duration-(--dur-fast)";

type StepListProps = { steps: readonly StepperStep[]; current: string };

function useStatusLabels(): Record<StepStatus, string> {
  const { t } = useLingui();
  return {
    todo: t`не выполнено`,
    done: t`выполнено`,
    done_by_env: t`выполнено окружением`,
    unavailable: t`недоступно`,
  };
}

/** Статус стоит в тексте шага, а значок скрыт от скринридера. */
function StepBody({ step, index, status }: { step: StepperStep; index: number; status: string }) {
  const Icon = STATUS_ICON[step.status];
  return (
    <>
      <Icon
        aria-hidden="true"
        className={cx(
          "mt-0.5 size-4 shrink-0 forced-colors:text-[CanvasText]",
          STATUS_COLOR[step.status],
        )}
      />
      <span className="min-w-0">
        <span aria-hidden="true" className="mr-1.5 text-muted-foreground tabular-nums">
          {index + 1}
        </span>
        <span className="wrap-anywhere">{step.label}</span>
        {/* Запятая видна: sr-only-запятая в Chromium даёт в имени «Хранилище , выполнено» */}
        <span className="text-caption text-muted-foreground">
          {", "}
          {status}
        </span>
      </span>
    </>
  );
}

/** Нумерованный список шагов: `<ol>` внутри `<nav>`, текущий шаг помечен `aria-current="step"`. */
export function StepList({ steps, current }: StepListProps) {
  const statusLabels = useStatusLabels();
  return (
    <ol className="m-0 flex list-none flex-col gap-0.5 p-0">
      {steps.map((step, index) => {
        const isCurrent = step.id === current;
        const className = cx(
          ROW,
          isCurrent
            ? "bg-brand-container font-semibold text-brand inset-ring inset-ring-brand"
            : "font-normal",
        );
        const body = <StepBody step={step} index={index} status={statusLabels[step.status]} />;
        return (
          <li key={step.id}>
            {step.href === undefined ? (
              <span aria-current={isCurrent ? "step" : undefined} className={className}>
                {body}
              </span>
            ) : (
              <Link
                href={step.href}
                aria-current={isCurrent ? "step" : undefined}
                className={cx(className, "hover:bg-accent")}
              >
                {body}
              </Link>
            )}
          </li>
        );
      })}
    </ol>
  );
}
