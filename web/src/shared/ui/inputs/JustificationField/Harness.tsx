import { useState } from "react";
import { JustificationField } from "./JustificationField";

/** Управляемая обёртка для тестов и stories: поле хранит значение само, как это делает форма. */
export function Harness({
  label,
  initial = "",
  showError = false,
}: {
  label: string;
  initial?: string;
  showError?: boolean;
}) {
  const [value, setValue] = useState(initial);
  return (
    <JustificationField label={label} value={value} onChange={setValue} showError={showError} />
  );
}
