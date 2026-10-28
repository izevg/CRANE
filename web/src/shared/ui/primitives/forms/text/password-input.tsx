import type { Input as InputPrimitive } from "@base-ui/react/input";
import { useLingui } from "@lingui/react/macro";
import { EyeIcon, EyeOffIcon } from "lucide-react";
import { useCallback, useId, useState } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { Button } from "../toggle/button";
import { Input } from "./input";

type PasswordInputProps = WithClassName<Omit<InputPrimitive.Props, "type">> & {
  /** Имя кнопки показа; по умолчанию «Показать пароль». На экране токена: «Показать токен». */
  toggleLabel?: string;
};

/**
 * Поле пароля или токена с кнопкой «Показать пароль». Вставку не перехватывает и длину не ограничивает: токены
 * длинные, а менеджеры паролей и вставка из буфера — единственный нормальный способ их ввести. Подсказки
 * клавиатуры и автоисправления выключены: токен не слово. На сенсорных экранах шрифт 16 px, иначе iOS
 * приближает страницу при фокусе. `autoComplete` задаёт вызывающий: `current-password` или `new-password`.
 */
export function PasswordInput({
  id,
  toggleLabel,
  className,
  spellCheck = false,
  autoCapitalize = "off",
  autoCorrect = "off",
  ...props
}: PasswordInputProps) {
  const { t } = useLingui();
  const generatedId = useId();
  const inputId = id ?? generatedId;
  const [shown, setShown] = useState(false);
  const toggle = useCallback(() => setShown((value) => !value), []);
  return (
    <div className="relative">
      <Input
        {...props}
        id={inputId}
        type={shown ? "text" : "password"}
        spellCheck={spellCheck}
        autoCapitalize={autoCapitalize}
        autoCorrect={autoCorrect}
        className={cx("pr-10 pointer-coarse:text-base", className)}
      />
      <div className="absolute inset-y-0 right-1.5 flex items-center">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={toggleLabel ?? t`Показать пароль`}
          aria-pressed={shown}
          aria-controls={inputId}
          onClick={toggle}
        >
          {shown ? <EyeOffIcon aria-hidden="true" /> : <EyeIcon aria-hidden="true" />}
        </Button>
      </div>
    </div>
  );
}
