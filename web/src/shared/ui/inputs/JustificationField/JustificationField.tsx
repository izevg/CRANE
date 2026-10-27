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
  /** Показать ошибку недобора сразу, например при попытке отправки; иначе она появляется после ухода из поля. */
  showError?: boolean;
}>;

/**
 * Обоснование действия (patterns §20, §23): обязательное textarea, счётчик и ошибки недобора и превышения. Длина
 * считается по code points без пробелов по краям, как в спеке, поэтому нативного `maxLength` (он считает UTF-16
 * единицы) нет: лишнее не обрезается молча, а объясняется ошибкой. Счётчик и ошибка связаны с полем через
 * `aria-describedby`.
 */
export function JustificationField({
  label,
  value,
  onChange,
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
  const status = justificationStatus(value);
  const invalid = status === "long" || (status === "short" && (touched || showError));
  const count = justificationLength(value);
  const left = justificationLeft(value);
  const limit = status === "short" ? JUSTIFICATION_MIN : JUSTIFICATION_MAX;
  return (
    <Field invalid={invalid} {...(className === undefined ? {} : { className })}>
      <FieldLabel>{label}</FieldLabel>
      <Textarea
        value={value}
        required
        aria-required="true"
        onChange={handleChange}
        onBlur={handleBlur}
      />
      <FieldDescription>
        {status === "ok"
          ? t`${plural(left, {
              one: "Остался # символ",
              few: "Осталось # символа",
              many: "Осталось # символов",
              other: "Осталось # символа",
            })}`
          : t`${count} из ${limit}`}
      </FieldDescription>
      <FieldError match={invalid}>
        {status === "long"
          ? t`${plural(JUSTIFICATION_MAX, {
              one: "Максимум # символ",
              few: "Максимум # символа",
              many: "Максимум # символов",
              other: "Максимум # символа",
            })}`
          : t`${plural(JUSTIFICATION_MIN, {
              one: "Минимум # символ",
              few: "Минимум # символа",
              many: "Минимум # символов",
              other: "Минимум # символа",
            })}`}
      </FieldError>
    </Field>
  );
}
