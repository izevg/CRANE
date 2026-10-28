# Stories кита

Витрина и спецификация компонентов `shared/ui`. Запуск: `task web:storybook` (порт 6006). Story-тесты идут в `task web:test` (проект `storybook`, headless Chromium); один раз на машине: `pnpm --filter web exec playwright install chromium`.

## Правила

- Файл `Component.stories.tsx` лежит рядом с компонентом, формат CSF3: `const meta = {...} satisfies Meta<typeof Component>`, `type Story = StoryObj<typeof meta>`. Файл-вход — `export default meta`.
- Импорт компонентов в stories — через барель группы (`~/shared/ui/primitives`), `storybook/test` — для `expect`, `fn`, `userEvent`, `within`.
- `title` по группе: «Примитивы/…», «Действия/…», «Статус/…». Строки и данные — русские и из предметной области CRANE (`@acme/ui` 1.4.2, `lodash` 4.17.20, `CVE-2026-31337`, `crp_9f8e7d6c`, «Карантин»/«Выпущена»/«Заблокирована»).
- Обязательны: по story на каждое состояние (варианты, пусто, ошибка, disabled) и `play` там, где важно поведение: клавиатура, фокус, disabled с причиной, открытие и закрытие оверлея. Story «Открыто» не закрывайте (см. ниже); story с проверкой закрытия закрывает оверлей сама.
- Axe проверяет весь `body` (`parameters.a11y.context = "body"`), поэтому порталы оверлеев попадают в проверку. У каждого оверлея должна быть story, которая остаётся ОТКРЫТОЙ в момент проверки: `defaultOpen` в args или `play`, который открывает и не закрывает. Закрывать в конце `play` можно только там, где проверяется само закрытие.
- Тёмная тема — story с `globals: { theme: "dark" }` (проверяет контраст).
- Декораторы из `preview.tsx` уже дают `LinkProvider`, `TooltipProvider`, `LiveRegion` и `Toaster`; тулбар переключает тему (`data-theme`) и reduced motion (`data-motion="reduce"` на `<html>`).
- Запросы в `play` — по ролям и именам; для порталов (Dialog, Menu, Tooltip) используйте `within(document.body)`.
- Stories не проверяются AAA-чекером и не считаются в правиле четырёх. Правила Biome `noJsxLiterals` и `noMagicNumbers` для них выключены.

## a11y

`parameters.a11y.test = "error"` включён глобально: нарушение axe валит story-тест. Сначала исправьте компонент. Отключать правило можно только в конкретной story и с причиной:

```tsx
export const OpenSelect: Story = {
  parameters: {
    a11y: {
      // Причина: Base UI держит фокус-ловушки с aria-hidden, пока список открыт (aria-hidden-focus), это не дефект разметки кита.
      config: { rules: [{ id: "aria-hidden-focus", enabled: false }] },
    },
  },
};
```

## Язык

Тулбар «Язык» (RU/EN) вызывает `activateLocale` в `loaders`, `I18nProvider` стоит в `StoryShell`. Выбор в `localStorage` витрина не пишет. Юнит-тесты (jsdom) получают `I18nProvider` автоматически: в `vitest.unit.config.ts` `@testing-library/react` подменён на `test/rtl.tsx`; импортируйте `render` как обычно.
