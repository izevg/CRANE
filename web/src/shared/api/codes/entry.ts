import type { MessageDescriptor } from "@lingui/core";
import type { Tone } from "~/shared/ui/primitives";

/** Запись словаря состояния: пропсы `StatusBadge` (`<StatusBadge {...VERSION_STATE[state]} />`). */
export type StatusEntry = { tone: Tone; label: MessageDescriptor };
