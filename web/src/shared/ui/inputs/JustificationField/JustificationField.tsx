import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback, useState } from "react";
import type { WithClassName } from "~/shared/lib/core";
import { Field, FieldDescription, FieldError, FieldLabel, Textarea } from "../../primitives";
import {
  JUSTIFICATION_MAX,
  JUSTIFICATION_MIN,
  justificationLeft,
  justificationLength,
  justificationStatus,
} from "./justification";

type JustificationFieldProps = WithClassName<{
  /** Подпись поля: «Обоснование», «Причина удаления». */
  label: string;
  value: string;
  onChange: (value: string) => void;
  minLength?: number;
  maxLength?: number;
  /** Показать ошибку недобора сразу, например при попытке отправки; иначе она появляется после ухода из поля. */
  showError?: boolean;
}>;

/**
 * Обоснование действия (patterns §20, §23): textarea, счётчик и ошибка недобора. Длина считается по code points
 * без пробелов по краям, как в спеке. Счётчик и ошибка связаны с полем через `aria-describedby`.
 */
export function JustificationField({
  label,
  value,
  onChange,
  minLength = JUSTIFICATION_MIN,
  maxLength = JUSTIFICATION_MAX,
  showError = false,
  className,
}: JustificationFieldProps) {
  const { t } = useLingui();
  const [touched, setTouched] = useState(false);
  const handleBlur = useCallback(() => setTouched(true), []);
  const handleChange = useCallback(
    (event: { target: { value: string } }) => onChange(event.target.value),
    [onChange],
  );
  const status = justificationStatus(value, minLength, maxLength);
  const invalid = status === "short" && (touched || showError);
  const count = justificationLength(value);
  const left = justificationLeft(value, maxLength);
  return (
    <Field invalid={invalid} {...(className === undefined ? {} : { className })}>
      <FieldLabel>{label}</FieldLabel>
      <Textarea value={value} maxLength={maxLength} onChange={handleChange} onBlur={handleBlur} />
      <FieldDescription>
        {status === "short"
          ? t`${count} из ${minLength}`
          : t`${plural(left, {
              one: "Остался # символ",
              few: "Осталось # символа",
              many: "Осталось # символов",
              other: "Осталось # символа",
            })}`}
      </FieldDescription>
      <FieldError match={invalid}>{t`Минимум ${minLength} символов`}</FieldError>
    </Field>
  );
}
