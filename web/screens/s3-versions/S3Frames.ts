import { expect, within } from "storybook/test";
import { screenParameters } from "../frame/screen";

const SELECTED_ROW = /1\.4\.2/;

const WIDE = { name: "Desktop 1600×900", styles: { width: "1600px", height: "900px" } };
export const wideParameters = {
  ...screenParameters,
  viewport: { options: { ...screenParameters.viewport.options, wide: WIDE } },
} as const;

export async function expectScreen(canvasElement: HTMLElement) {
  const body = within(canvasElement.ownerDocument.body);
  await expect(await body.findByRole("heading", { level: 1, name: "@acme/ui" })).toBeVisible();
  await expect(body.getByRole("row", { name: SELECTED_ROW, selected: true })).toBeVisible();
}
