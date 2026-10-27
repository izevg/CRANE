import { useLingui } from "@lingui/react/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { useSessionDraft } from "~/shared/lib/state";
import { ConfirmDialog } from "~/shared/ui/dialogs";
import {
  isJustified,
  JUSTIFICATION_MAX,
  JUSTIFICATION_MIN,
  JustificationField,
} from "~/shared/ui/inputs";
import { Callout, CalloutDescription } from "~/shared/ui/primitives";
import { fitsViewport } from "../frame/fit";
import { desktop, mobile, mobile320, screenParameters } from "../frame/screen";
import { type RevealSlot, S9Page } from "./S9Page";

const isText = (value: unknown): value is string => typeof value === "string";

/** Раскрытие данных (patterns §20): ConfirmDialog с обоснованием; введённое переживает редирект step-up. */
function RevealDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useLingui();
  // Текст обоснования живёт только в черновике: он же переживает редирект step-up
  const [draft, saveDraft, clearDraft] = useSessionDraft("reveal-event", isText);
  const text = draft ?? "";
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t`Раскрыть данные события?`}
      description={t`Будут показаны логин и полный IP-адрес актора.`}
      consequences={[t`Раскрытие запишется в журнал`, t`Данные видны только в этой панели`]}
      confirm={{
        label: t`Раскрыть`,
        run: { onSelect: clearDraft },
        stepUp: true,
        ...(isJustified(text)
          ? {}
          : {
              disabledReason: t`Обоснование: от ${JUSTIFICATION_MIN} до ${JUSTIFICATION_MAX} символов`,
            }),
      }}
    >
      <Callout tone="info">
        <CalloutDescription>{t`Потребуется подтверждение входа (MFA)`}</CalloutDescription>
      </Callout>
      <JustificationField label={t`Обоснование`} value={text} onChange={saveDraft} />
    </ConfirmDialog>
  );
}

type ScreenProps = { auditor?: boolean; revealOpen?: boolean };

const renderReveal: RevealSlot = (state) => <RevealDialog {...state} />;

function S9Screen(props: ScreenProps) {
  return <S9Page {...props} renderReveal={renderReveal} />;
}

const meta = {
  title: "Экраны/S9 Журнал",
  component: S9Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S9Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

const WIDE = { name: "Desktop 1600×900", styles: { width: "1600px", height: "900px" } };
const wideParameters = {
  ...screenParameters,
  viewport: { options: { ...screenParameters.viewport.options, wide: WIDE } },
} as const;
const wide = { viewport: { value: "wide", isRotated: false } } as const;

/** 1440: панель события открывается как Sheet. */
export const Desktop1440: Story = {
  name: "Desktop 1440 · SecOps",
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    const dialog = await body.findByRole("dialog", { name: "Событие" });
    await expect(dialog).toBeVisible();
    // «Действие»: подпись и код рядом, как в таблице, а не сырой код
    const value = within(dialog).getByText("Действие").nextElementSibling;
    await expect(value?.querySelectorAll("span span")).toHaveLength(2);
  },
};

export const Desktop1600: Story = {
  name: "Desktop 1600 · панель docked",
  parameters: wideParameters,
  globals: wide,
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(
      await body.findByRole("heading", { level: 1, name: "Журнал действий" }),
    ).toBeVisible();
  },
};

export const DesktopDark: Story = {
  name: "Desktop dark · docked",
  parameters: wideParameters,
  globals: { ...wide, theme: "dark" },
};

export const Mobile390: Story = {
  name: "Mobile 390",
  globals: mobile,
};

export const AuditorMode: Story = {
  name: "Режим аудитора",
  args: { auditor: true },
  parameters: wideParameters,
  globals: wide,
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByText("Режим аудитора: данные маскированы")).toBeVisible();
    await expect(body.queryByRole("button", { name: "Экспорт" })).toBeNull();
    await expect(body.queryByRole("button", { name: "Раскрыть данные" })).toBeNull();
  },
};

export const RevealDialogOpen: Story = {
  name: "Диалог раскрытия данных",
  args: { revealOpen: true },
  parameters: wideParameters,
  globals: wide,
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    const dialog = await body.findByRole("alertdialog", { name: "Раскрыть данные события?" });
    await expect(within(dialog).getByRole("textbox", { name: "Обоснование" })).toBeVisible();
  },
};

export const English: Story = {
  ...Desktop1440,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByRole("dialog", { name: "Event" })).toBeVisible();
    // Тип актора в таблице — подпись интерфейса, а не контент: по-английски
    await expect(body.getAllByText("Human").length).toBeGreaterThan(0);
  },
};

/** Топбар не шире окна на телефоне (WCAG 1.4.10): ширины 390 и 320, обе локали. */
export const Fit390: Story = {
  ...Mobile390,
  name: "Телефон 390: топбар помещается",
  play: fitsViewport,
};

export const Fit320: Story = {
  ...Mobile390,
  name: "Телефон 320: топбар помещается",
  globals: mobile320,
  play: fitsViewport,
};

export const Fit390English: Story = {
  ...Mobile390,
  name: "Телефон 390 EN: топбар помещается",
  globals: { ...mobile, locale: "en" },
  play: fitsViewport,
};

export const Fit320English: Story = {
  ...Mobile390,
  name: "Телефон 320 EN: топбар помещается",
  globals: { ...mobile320, locale: "en" },
  play: fitsViewport,
};
