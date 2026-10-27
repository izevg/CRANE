import { useCallback, useSyncExternalStore } from "react";
import {
  type ColorMode,
  getColorMode,
  getResolvedScheme,
  type ResolvedScheme,
  setColorMode,
  subscribeColorMode,
} from "./colorScheme";

type ColorModeApi = {
  mode: ColorMode;
  setMode: (mode: ColorMode) => void;
  resolved: ResolvedScheme;
};

export function useColorMode(): ColorModeApi {
  const mode = useSyncExternalStore(subscribeColorMode, getColorMode, () => "system" as const);
  const resolved = useSyncExternalStore(
    subscribeColorMode,
    getResolvedScheme,
    () => "light" as const,
  );
  const setMode = useCallback((next: ColorMode) => setColorMode(next), []);
  return { mode, setMode, resolved };
}
