import { msg } from "@lingui/core/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { SlidersHorizontalIcon } from "lucide-react";
import { useCallback, useId, useState } from "react";
import { expect, within } from "storybook/test";
import { textFormat } from "~/shared/lib/text";
import { FacetList, FilterSelect, SegmentedControl } from "~/shared/ui/inputs";
import { PageHeader } from "~/shared/ui/layout";
import {
  Badge,
  Button,
  Callout,
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
import { EmptyState, RelativeTime, StatusBadge } from "~/shared/ui/status";
import type { Banner } from "~/widgets/app-shell";
import { REPOSITORIES, WORKSPACES } from "../data/common";
import {
  FORMAT_FACETS,
  RESULTS,
  riskTone,
  type SearchResult,
  WORKSPACE_FACETS,
} from "../data/search";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";
import { tr } from "../frame/tr";

// Макеты экранов — на русском; форматирование чисел фиксирует ru, активная локаль витрины им не нужна.
const format = textFormat("ru");

const FORMAT_SEGMENTS = [
  {
    value: "all",
    get label() {
      return tr(msg`Все`);
    },
    count: 38,
  },
  { value: "npm", label: "npm", count: 21 },
  { value: "pypi", label: "PyPI", count: 8 },
  { value: "maven", label: "Maven", count: 5 },
  { value: "nuget", label: "NuGet", count: 3 },
  { value: "cargo", label: "Cargo", count: 1 },
] as const;

const REPOSITORY_OPTIONS = REPOSITORIES.map((repo) => ({ value: repo.id, label: repo.id }));
const WORKSPACE_OPTIONS = WORKSPACES.map((name) => ({ value: name, label: name }));

const DEGRADED: readonly Banner[] = [
  {
    id: "semantic",
    tone: "warning",
    get text() {
      return tr(msg`Ассистенты недоступны: поиск работает без семантики`);
    },
    action: {
      get label() {
        return tr(msg`Статус платформы`);
      },
      run: { href: "/status" },
    },
  },
];

// Спека волны 2: patterns §6 — будущий widgets/search-results/ResultCard
function ResultCard({ item }: { item: SearchResult }) {
  const titleId = useId();
  return (
    <article
      aria-labelledby={titleId}
      className="flex min-w-0 flex-col gap-1.5 rounded-xl bg-card p-4 shadow-card"
    >
      {item.exact ? <p className="text-overline text-brand">{tr(msg`Точное совпадение`)}</p> : null}
      {item.typosquatOf ? (
        <Callout tone="warning" className="py-1 text-caption">
          {tr(msg`Похоже на ${item.typosquatOf} — 1 символ (возможный typosquat). `)}
          <Link href={`/packages/${item.typosquatOf}`} className="underline">
            {item.typosquatOf}
          </Link>
        </Callout>
      ) : null}
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <h3 id={titleId} className="min-w-0 text-h3 wrap-anywhere">
          <Link href={`/packages/${item.id}`} className="text-foreground hover:text-brand">
            {item.name}
          </Link>
        </h3>
        <Badge variant="outline">{item.format}</Badge>
        <Badge variant="secondary">{`${item.repository} · ${item.workspace}`}</Badge>
        {item.deprecated ? <StatusBadge tone="warning" label={tr(msg`Устарел`)} /> : null}
        {item.risk ? (
          <span className="ml-auto">
            <StatusBadge
              tone={riskTone(item.risk.priority)}
              label={`${item.risk.priority} · ${item.risk.findings} ${item.risk.findings === 1 ? tr(msg`находка`) : tr(msg`находки`)}`}
            />
          </span>
        ) : null}
      </div>
      <p className="truncate text-body text-muted-foreground">{item.description}</p>
      <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-caption text-muted-foreground">
        <span className="flex items-center gap-2">
          <span>{tr(msg`Последняя ${item.version}`)}</span>
          <StatusBadge tone={item.state.tone} label={item.state.label} />
        </span>
        <span className="tabular-nums">
          {tr(msg`${format.number(item.downloads)} загрузок за 30 дн`)}
        </span>
        <span>{item.team}</span>
        <RelativeTime ts={item.updated} />
      </p>
    </article>
  );
}

function Facets() {
  const [formats, setFormats] = useState<readonly string[]>([]);
  const [spaces, setSpaces] = useState<readonly string[]>([]);
  const toggleFormat = useCallback((value: string) => setFormats((prev) => flip(prev, value)), []);
  const toggleSpace = useCallback((value: string) => setSpaces((prev) => flip(prev, value)), []);
  return (
    <div className="flex flex-col gap-4">
      <FacetList
        title={tr(msg`По форматам`)}
        items={FORMAT_FACETS}
        selected={formats}
        onToggle={toggleFormat}
      />
      <FacetList
        title={tr(msg`По workspace`)}
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
  const [format, setFormat] = useState("all");
  const [repository, setRepository] = useState<string | null>(null);
  const [workspace, setWorkspace] = useState<string | null>(null);
  const switchLabelId = useId();
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-3">
      <SegmentedControl
        label={tr(msg`Формат пакета`)}
        items={FORMAT_SEGMENTS}
        value={format}
        onValueChange={setFormat}
      />
      <FilterSelect
        label={tr(msg`Репозиторий`)}
        allLabel={tr(msg`Все репозитории`)}
        options={REPOSITORY_OPTIONS}
        value={repository}
        onValueChange={setRepository}
      />
      <FilterSelect
        label="Workspace"
        allLabel={tr(msg`Workspace: все`)}
        options={WORKSPACE_OPTIONS}
        value={workspace}
        onValueChange={setWorkspace}
      />
      <span className="flex items-center gap-2 text-body">
        <Switch aria-labelledby={switchLabelId} />
        <span id={switchLabelId}>{tr(msg`Показывать устаревшие`)}</span>
      </span>
    </div>
  );
}

function MobileFilters() {
  return (
    <Sheet>
      <SheetTrigger
        render={
          <Button variant="secondary" className="@4xl/page:hidden">
            <SlidersHorizontalIcon aria-hidden="true" />
            {tr(msg`Фильтры`)}
          </Button>
        }
      />
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{tr(msg`Фильтры`)}</SheetTitle>
          <SheetDescription>{tr(msg`Форматы и workspace`)}</SheetDescription>
        </SheetHeader>
        <SheetBody>
          <Facets />
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}

function SearchBar({ query }: { query: string }) {
  return (
    <search className="flex gap-2">
      <Input
        type="search"
        aria-label={tr(msg`Поиск пакетов`)}
        defaultValue={query}
        className="flex-1"
      />
      <Button>{tr(msg`Найти`)}</Button>
    </search>
  );
}

function ResultsList() {
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-3">
      <h2 id={headingId} className="sr-only">
        {tr(msg`Результаты поиска`)}
      </h2>
      <p role="status" className="text-caption text-muted-foreground tabular-nums">
        {tr(msg`38 результатов · 74 мс`)}
      </p>
      {RESULTS.map((item) => (
        <ResultCard key={item.id} item={item} />
      ))}
    </section>
  );
}

function NothingFound({ query }: { query: string }) {
  return (
    <div className="flex flex-col items-center gap-4 rounded-xl bg-card py-4 shadow-card">
      <EmptyState
        variant="filtered"
        title={tr(msg`Ничего не нашлось по «${query}»`)}
        reset={{ label: tr(msg`Сбросить фильтры`), run: { onSelect: () => undefined } }}
      />
      <ul className="max-w-[60ch] list-disc space-y-1 pl-5 text-body text-muted-foreground">
        <li>{tr(msg`Проверьте формат: «format:npm» ищет только среди npm-пакетов`)}</li>
        <li>{tr(msg`Уберите фильтры по репозиторию или workspace`)}</li>
        <li>
          {tr(msg`Возможно, вы искали `)}
          <Link href="/packages/acme-ui" className="text-brand underline">
            @acme/ui
          </Link>
        </li>
      </ul>
    </div>
  );
}

function SearchTab({ empty }: { empty: boolean }) {
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
        <aside
          aria-label={tr(msg`Фильтры`)}
          className="hidden @4xl/page:col-span-4 @4xl/page:block"
        >
          <Facets />
        </aside>
      </div>
    </div>
  );
}

// Спека волны 2: screens S1 — будущий pages/search
function S1Page({ empty = false }: { empty?: boolean }) {
  return (
    <Tabs defaultValue="search" className="gap-4">
      <PageHeader
        title={tr(msg`Поиск`)}
        caption={tr(msg`Пакеты всех форматов в репозиториях, доступных вам`)}
        tabs={
          <TabsList variant="line" aria-label={tr(msg`Разделы поиска`)}>
            <TabsTrigger value="search">{tr(msg`Поиск`)}</TabsTrigger>
            <TabsTrigger value="favorites">{tr(msg`Избранное`)}</TabsTrigger>
            <TabsTrigger value="recent">{tr(msg`Недавние`)}</TabsTrigger>
          </TabsList>
        }
      />
      <TabsContent value="search">
        <SearchTab empty={empty} />
      </TabsContent>
      <TabsContent value="favorites">{tr(msg`Избранные пакеты появятся здесь`)}</TabsContent>
      <TabsContent value="recent">{tr(msg`Недавно открытые пакеты появятся здесь`)}</TabsContent>
    </Tabs>
  );
}

type Props = { empty?: boolean; banners?: readonly Banner[] };

function Screen({ empty = false, banners }: Props) {
  return (
    <ScreenFrame
      active="search"
      breadcrumbs={{ trail: [], current: tr(msg`Поиск`) }}
      {...(banners ? { banners } : {})}
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
  args: { banners: DEGRADED },
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
