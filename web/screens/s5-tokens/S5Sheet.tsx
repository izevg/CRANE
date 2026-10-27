import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback, useId, useState } from "react";
import { i18n } from "~/shared/lib/i18n";
import { SegmentedControl } from "~/shared/ui/inputs";
import {
  Button,
  Checkbox,
  Field,
  FieldDescription,
  FieldLabel,
  Input,
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  Textarea,
} from "~/shared/ui/primitives";

function ttlSegments() {
  return [
    {
      value: "7",
      label: i18n._(msg`7 дней`),
    },
    {
      value: "30",
      label: i18n._(msg`30 дней`),
    },
    {
      value: "90",
      label: i18n._(msg`90 дней`),
    },
  ] as const;
}

const REPOS = ["npm-hosted", "pypi-hosted", "maven-central-proxy", "nuget-hosted"] as const;

// Спека волны 2: screens S5 — будущий features/tokens/NewTokenSheet (форма в Sheet)
export function NewTokenSheet({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const { t } = useLingui();
  const [ttl, setTtl] = useState("30");
  const close = useCallback(() => onOpenChange(false), [onOpenChange]);
  const rightsId = useId();
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{t`Новый токен`}</SheetTitle>
          <SheetDescription>{t`Секрет будет показан один раз после выпуска`}</SheetDescription>
        </SheetHeader>
        <SheetBody className="flex flex-col gap-4">
          <Field name="name">
            <FieldLabel>{t`Название`}</FieldLabel>
            <Input defaultValue="ci-platform · npm" />
            <FieldDescription>{t`До 64 символов, видно только вам`}</FieldDescription>
          </Field>
          <fieldset aria-labelledby={rightsId} className="flex flex-col gap-2">
            <legend id={rightsId} className="text-body font-semibold">
              {t`Права по репозиториям`}
            </legend>
            {REPOS.map((repo) => (
              <div
                key={repo}
                className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1"
              >
                <span className="text-body">{repo}</span>
                <span className="flex items-center gap-3 text-caption">
                  <span className="flex items-center gap-1.5">
                    <Checkbox aria-label={t`${repo}: чтение`} defaultChecked />
                    {"read"}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Checkbox aria-label={t`${repo}: публикация`} />
                    {"publish"}
                  </span>
                </span>
              </div>
            ))}
          </fieldset>
          <div className="flex flex-col gap-1.5">
            <p className="text-body font-semibold">{t`Срок действия`}</p>
            <SegmentedControl
              label={t`Срок действия`}
              items={ttlSegments()}
              value={ttl}
              onValueChange={setTtl}
            />
            <p className="text-caption text-muted-foreground">
              {t`Максимум 90 дней — ограничено политикой`}
            </p>
          </div>
          <Field name="description">
            <FieldLabel>{t`Описание`}</FieldLabel>
            <Textarea defaultValue={t`Публикация npm-пакетов из GitLab CI проекта platform/ui`} />
          </Field>
        </SheetBody>
        <SheetFooter>
          <Button>{t`Выпустить`}</Button>
          <Button variant="secondary" onClick={close}>
            {t`Отмена`}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
