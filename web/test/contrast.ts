/** Контраст по WCAG 2.x для проверок в story-тестах: относительная яркость и отношение двух цветов. */

type Rgba = { r: number; g: number; b: number; a: number };

const CHANNEL_MAX = 255;
const LINEAR_THRESHOLD = 0.03928;
const LINEAR_DIVISOR = 12.92;
const GAMMA_OFFSET = 0.055;
const GAMMA_DIVISOR = 1.055;
const GAMMA = 2.4;
const WEIGHT_R = 0.2126;
const WEIGHT_G = 0.7152;
const WEIGHT_B = 0.0722;
const LUMINANCE_OFFSET = 0.05;
const RGB = /rgba?\(([^)]+)\)/;
const SRGB = /color\(srgb ([^)]+)\)/;
const RGB_SEPARATORS = /[ ,/]+/;
const SRGB_SEPARATORS = /[ /]+/;

function parseCssColor(value: string): Rgba {
  const rgb = RGB.exec(value);
  if (rgb?.[1] !== undefined) {
    const [r = 0, g = 0, b = 0, a = 1] = rgb[1].split(RGB_SEPARATORS).map(Number);
    return { r, g, b, a };
  }
  const srgb = SRGB.exec(value);
  if (srgb?.[1] !== undefined) {
    const [r = 0, g = 0, b = 0, a = 1] = srgb[1].split(SRGB_SEPARATORS).map(Number);
    return { r: r * CHANNEL_MAX, g: g * CHANNEL_MAX, b: b * CHANNEL_MAX, a };
  }
  throw new Error(`Не удалось разобрать цвет: ${value}`);
}

function linear(channel: number): number {
  const c = channel / CHANNEL_MAX;
  return c <= LINEAR_THRESHOLD ? c / LINEAR_DIVISOR : ((c + GAMMA_OFFSET) / GAMMA_DIVISOR) ** GAMMA;
}

export function luminance(color: string): number {
  const { r, g, b } = parseCssColor(color);
  return WEIGHT_R * linear(r) + WEIGHT_G * linear(g) + WEIGHT_B * linear(b);
}

/** Отношение контраста двух цветов, ≥ 1. */
export function contrastRatio(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return ((light ?? 0) + LUMINANCE_OFFSET) / ((dark ?? 0) + LUMINANCE_OFFSET);
}

/** Цвет фона элемента: первый непрозрачный фон вверх по предкам (кнопка ghost прозрачна — фон даёт страница). */
export function effectiveBackground(element: Element): string {
  for (let node: Element | null = element; node !== null; node = node.parentElement) {
    const color = getComputedStyle(node).backgroundColor;
    if (parseCssColor(color).a > 0) {
      return color;
    }
  }
  return "rgb(255, 255, 255)";
}
