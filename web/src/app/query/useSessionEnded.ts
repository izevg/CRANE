import { useSyncExternalStore } from "react";
import { isSessionEnded, subscribeSessionEnded } from "./sessionEnded";

export function useSessionEnded(): boolean {
  return useSyncExternalStore(subscribeSessionEnded, isSessionEnded, isSessionEnded);
}
