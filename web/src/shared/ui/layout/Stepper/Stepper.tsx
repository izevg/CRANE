import { Collapsible } from "@base-ui/react/collapsible";
import { useLingui } from "@lingui/react/macro";
import { ChevronDownIcon } from "lucide-react";
import { useBreakpoint, type WithClassName } from "~/shared/lib/core";
import { buttonStyles } from "../../primitives";
import { StepList, type StepperStep } from "./StepList";

export type { StepperStep } from "./StepList";

type StepperProps = WithClassName<{
  /** Имя навигации для скринридера, например «Шаги настройки». */
  label: string;
  steps: readonly StepperStep[];
  /** `id` текущего шага. */
  current: string;
}>;

function NarrowStepper({ steps, current }: Pick<StepperProps, "steps" | "current">) {
  const { t } = useLingui();
  const index = steps.findIndex((step) => step.id === current);
  const step = steps[index];
  // current вне списка — ошибка вызывающего: строку «Шаг 0 из N» не рисуем, остаётся только список
  if (step === undefined) {
    return <StepList steps={steps} current={current} />;
  }
  const position = index + 1;
  const total = steps.length;
  const title = step.label;
  return (
    <Collapsible.Root className="flex flex-col gap-1">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="m-0 min-w-0 text-body font-semibold wrap-anywhere">
          {t`Шаг ${position} из ${total} · ${title}`}
        </p>
        <Collapsible.Trigger className={buttonStyles({ variant: "ghost", size: "sm" })}>
          {t`Все шаги`}
          <ChevronDownIcon
            aria-hidden="true"
            className="transition-transform duration-(--dur-fast) group-data-panel-open/button:rotate-180 motion-reduce:transition-none"
          />
        </Collapsible.Trigger>
      </div>
      <Collapsible.Panel>
        <StepList steps={steps} current={current} />
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}

/**
 * Шаги мастера: на ширине от `lg` — рейл со всеми шагами, уже — строка «Шаг n из N · название» и свёрнутый
 * список «Все шаги». Роутера кит не знает: ссылки идут через `Link`, так что `LinkProvider` приложения их подхватит.
 */
export function Stepper({ label, steps, current, className }: StepperProps) {
  const wide = useBreakpoint("lg");
  return (
    <nav aria-label={label} className={className}>
      {wide ? (
        <StepList steps={steps} current={current} />
      ) : (
        <NarrowStepper steps={steps} current={current} />
      )}
    </nav>
  );
}
