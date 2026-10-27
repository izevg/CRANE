/** Общие параметры экранных stories: полноэкранный макет и три вьюпорта. */
const VIEWPORTS = {
  desktop: { name: "Desktop 1440×900", styles: { width: "1440px", height: "900px" } },
  mobile: { name: "Mobile 390×844", styles: { width: "390px", height: "844px" } },
  mobile320: { name: "Mobile 320×640", styles: { width: "320px", height: "640px" } },
};

export const screenParameters = {
  layout: "fullscreen",
  viewport: { options: VIEWPORTS },
} as const;

/** Для `globals` story: `globals: desktop` / `globals: { ...mobile, theme: "dark" }`. */
export const desktop = { viewport: { value: "desktop", isRotated: false } } as const;
export const mobile = { viewport: { value: "mobile", isRotated: false } } as const;
export const mobile320 = { viewport: { value: "mobile320", isRotated: false } } as const;
