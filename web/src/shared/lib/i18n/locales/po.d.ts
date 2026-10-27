// Каталоги .po компилирует @lingui/vite-plugin при импорте.
declare module "*.po" {
  import type { Messages } from "@lingui/core";
  export const messages: Messages;
}
