import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { SlidersHorizontalIcon } from "lucide-react";
import { useCallback, useId, useState } from "react";
import { expect, within } from "storybook/test";
import { SearchResultCard } from "~/entities/package";
import { i18n } from "~/shared/lib/i18n";
import { FacetList, FilterSelect, SegmentedControl } from "~/shared/ui/inputs";
import { PageHeader } from "~/shared/ui/layout";
import {
  Button,
  Input,
  Link,
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
  Switch,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "~/shared/ui/primitives";
import { EmptyState } from "~/shared/ui/status";
import type { Banner } from "~/widgets/app-shell";
import { REPOSITORIES, WORKSPACES } from "../data/common";
import { FORMAT_FACETS, RESULTS, WORKSPACE_FACETS } from "../data/search";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";

function formatSegments() {
  return [
    {
      value: "all",
      label: i18n._(msg`Все`),
      count: 38,
    },
    { value: "npm", label: "npm", count: 21 },
    { value: "pypi", label: "PyPI", count: 8 },
    { value: "maven", label: "Maven", count: 5 },
    { value: "nuget", label: "NuGet", count: 3 },
    { value: "cargo", label: "Cargo", count: 1 },
  ] as const;
}

const REPOSITORY_OPTIONS = REPOSITORIES.map((repo) => ({ value: repo.id, label: repo.id }));
const WORKSPACE_OPTIONS = WORKSPACES.map((name) => ({ value: name, label: name }));

function degraded(): readonly Banner[] {
  return [
    {
      id: "semantic",
      tone: "warning",
      text: i18n._(msg`Ассистенты недоступны: поиск работает без семантики`),
      action: {
        label: i18n._(msg`Статус платформы`),
        run: { href: "/status" },
      },
    },
  ];
}

function Facets() {
  const { t } = useLingui();
  const [formats, setFormats] = useState<readonly string[]>([]);
  const [spaces, setSpaces] = useState<readonly string[]>([]);
  const toggleFormat = useCallback((value: string) => setFormats((prev) => flip(prev, value)), []);
  const toggleSpace = useCallback((value: string) => setSpaces((prev) => flip(prev, value)), []);
  return (
    <div className="flex flex-col gap-4">
      <FacetList
        title={t`По форматам`}
        items={FORMAT_FACETS}
        selected={formats}
        onToggle={toggleFormat}
      />
      <FacetList
        title={t`По workspace`}
        items={WORKSPACE_FACETS}
        selected={spaces}
        onToggle={toggleSpace}
      />
    </div>
  );
}

function flip(list: readonly string[], value: string): readonly string[] {
  return list.includes(value) ? list.filter((item) => item !== value) : [...list, value];
}

// Спека волны 2: screens S1 — будущий pages/search + features/search-filters
function FiltersRow() {
  const { t } = useLingui();
  const [format, setFormat] = useState("all");
  const [repository, setRepository] = useState<string | null>(null);
  const [workspace, setWorkspace] = useState<string | null>(null);
  const switchLabelId = useId();
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-3">
      <SegmentedControl
        label={t`Формат пакета`}
        items={formatSegments()}
        value={format}
        onValueChange={setFormat}
      />
      <FilterSelect
        label={t`Репозиторий`}
        allLabel={t`Все репозитории`}
        options={REPOSITORY_OPTIONS}
        value={repository}
        onValueChange={setRepository}
      />
      <FilterSelect
        label="Workspace"
        allLabel={t`Workspace: все`}
        options={WORKSPACE_OPTIONS}
        value={workspace}
        onValueChange={setWorkspace}
      />
      <span className="flex items-center gap-2 text-body">
        <Switch aria-labelledby={switchLabelId} />
        <span id={switchLabelId}>{t`Показывать устаревшие`}</span>
      </span>
    </div>
  );
}

function MobileFilters() {
  const { t } = useLingui();
  return (
    <Sheet>
      <SheetTrigger
        render={
          <Button variant="secondary" className="@4xl/page:hidden">
            <SlidersHorizontalIcon aria-hidden="true" />
            {t`Фильтры`}
          </Button>
        }
      />
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{t`Фильтры`}</SheetTitle>
          <SheetDescription>{t`Форматы и workspace`}</SheetDescription>
        </SheetHeader>
        <SheetBody>
          <Facets />
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}

function SearchBar({ query }: { query: string }) {
  const { t } = useLingui();
  return (
    <search className="flex gap-2">
      <Input type="search" aria-label={t`Поиск пакетов`} defaultValue={query} className="flex-1" />
      <Button>{t`Найти`}</Button>
    </search>
  );
}

function ResultsList() {
  const { t } = useLingui();
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-3">
      <h2 id={headingId} className="sr-only">
        {t`Результаты поиска`}
      </h2>
      <p role="status" className="text-caption text-muted-foreground tabular-nums">
        {t`38 результатов · 74 мс`}
      </p>
      {RESULTS.map(({ id, item }) => (
        <SearchResultCard key={id} item={item} href={`/packages/${id}`} />
      ))}
    </section>
  );
}

function NothingFound({ query }: { query: string }) {
  const { t } = useLingui();
  return (
    <div className="flex flex-col items-center gap-4 rounded-xl bg-card py-4 shadow-card">
      <EmptyState
        variant="filtered"
        title={t`Ничего не нашлось по «${query}»`}
        reset={{ label: t`Сбросить фильтры`, run: { onSelect: () => undefined } }}
      />
      <ul className="max-w-[60ch] list-disc space-y-1 pl-5 text-body text-muted-foreground">
        <li>{t`Проверьте формат: «format:npm» ищет только среди npm-пакетов`}</li>
        <li>{t`Уберите фильтры по репозиторию или workspace`}</li>
        <li>
          {t`Возможно, вы искали `}
          <Link href="/packages/acme-ui" className="text-brand underline">
            @acme/ui
          </Link>
        </li>
      </ul>
    </div>
  );
}

function SearchTab({ empty }: { empty: boolean }) {
  const { t } = useLingui();
  const query = empty ? "acme-uu-lib" : "acme ui";
  return (
    <div className="flex flex-col gap-4">
      <SearchBar query={query} />
      <FiltersRow />
      <MobileFilters />
      <div className="grid gap-6 @4xl/page:grid-cols-12">
        <div className="min-w-0 @4xl/page:col-span-8">
          {empty ? <NothingFound query={query} /> : <ResultsList />}
        </div>
        <aside aria-label={t`Фильтры`} className="hidden @4xl/page:col-span-4 @4xl/page:block">
          <Facets />
        </aside>
      </div>
    </div>
  );
}

// Спека волны 2: screens S1 — будущий pages/search
function S1Page({ empty = false }: { empty?: boolean }) {
  const { t } = useLingui();
  return (
    <Tabs defaultValue="search" className="gap-4">
      <PageHeader
        title={t`Поиск`}
        caption={t`Пакеты всех форматов в репозиториях, доступных вам`}
        tabs={
          <TabsList variant="line" aria-label={t`Разделы поиска`}>
            <TabsTrigger value="search">{t`Поиск`}</TabsTrigger>
            <TabsTrigger value="favorites">{t`Избранное`}</TabsTrigger>
            <TabsTrigger value="recent">{t`Недавние`}</TabsTrigger>
          </TabsList>
        }
      />
      <TabsContent value="search">
        <SearchTab empty={empty} />
      </TabsContent>
      <TabsContent value="favorites">{t`Избранные пакеты появятся здесь`}</TabsContent>
      <TabsContent value="recent">{t`Недавно открытые пакеты появятся здесь`}</TabsContent>
    </Tabs>
  );
}

type Props = { empty?: boolean; assistantsDown?: boolean };

function Screen({ empty = false, assistantsDown = false }: Props) {
  const { t } = useLingui();
  return (
    <ScreenFrame
      active="search"
      breadcrumbs={{ trail: [], current: t`Поиск` }}
      {...(assistantsDown ? { banners: degraded() } : {})}
    >
      <S1Page empty={empty} />
    </ScreenFrame>
  );
}

const meta = {
  title: "Экраны/S1 Поиск",
  component: Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DesktopLight: Story = {
  name: "Desktop 1440 · светлая",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Поиск" })).toBeVisible();
    await expect(canvas.getByText("38 результатов · 74 мс")).toBeVisible();
  },
};

export const DesktopDark: Story = {
  name: "Desktop 1440 · тёмная",
  globals: { ...desktop, theme: "dark" },
};

export const Mobile: Story = {
  name: "Mobile 390",
  globals: mobile,
};

export const NoResults: Story = {
  name: "Ничего не найдено",
  args: { empty: true },
};

export const AssistantsUnavailable: Story = {
  name: "Баннер: ассистенты недоступны",
  args: { assistantsDown: true },
};

export const English: Story = {
  ...DesktopLight,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Search" })).toBeVisible();
    await expect(canvas.getByText("38 results · 74 ms")).toBeVisible();
  },
};
