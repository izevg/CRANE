import { useLingui } from "@lingui/react/macro";
import { type ComponentProps, createContext, use } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import { Link } from "../../primitives";

const EXTERNAL = /^https?:\/\//i;

const REHYPE_PLUGINS = [rehypeSanitize];

// Картинка внутри ссылки (бейджи README) не может стать второй ссылкой: <a> в <a> невалиден.
const InsideLinkContext = createContext(false);

/** Текст недоверенный: ссылки идут через `Link` кита, внешние открываются в новой вкладке без `opener`. */
function MarkdownLink({ href, children }: ComponentProps<"a">) {
  const { t } = useLingui();
  if (href === undefined) {
    return <span>{children}</span>;
  }
  const content = <InsideLinkContext value>{children}</InsideLinkContext>;
  if (EXTERNAL.test(href)) {
    return (
      <Link
        href={href}
        target="_blank"
        rel="noopener noreferrer nofollow"
        className="text-brand underline"
      >
        {content}
        <span className="sr-only">{t` (откроется в новой вкладке)`}</span>
      </Link>
    );
  }
  return (
    <Link href={href} className="text-brand underline">
      {content}
    </Link>
  );
}

/**
 * Картинка из недоверенного текста не грузится: <img> сделал бы запрос к чужому хосту при открытии карточки
 * (tracking pixel, раздел 15 «Безопасность отдачи»). Вместо неё внешняя ссылка по правилам `MarkdownLink`;
 * адрес не http(s) (data:, относительный) ссылкой не становится, остаётся только alt.
 * Слой один, `components.img`: если убрать `img` из `tagNames` санитайзера, элемент вырежется до компонента
 * вместе с alt, и пользователь потеряет подпись и адрес.
 */
function MarkdownImage({ src, alt }: ComponentProps<"img">) {
  const insideLink = use(InsideLinkContext);
  const caption = alt ?? "";
  if (insideLink) {
    const text = caption === "" ? (src ?? "") : caption;
    return text === "" ? null : <span>{text}</span>;
  }
  if (typeof src === "string" && EXTERNAL.test(src)) {
    const label = caption === "" ? src : caption;
    return <MarkdownLink href={src}>{label}</MarkdownLink>;
  }
  if (caption === "") {
    return null;
  }
  return <span>{caption}</span>;
}

const COMPONENTS: Components = { a: MarkdownLink, img: MarkdownImage };

/**
 * Сырой HTML в исходнике не разбирается (`skipHtml`), а результат дополнительно проходит `rehype-sanitize`
 * со схемой по умолчанию. Адреса дополнительно проверяет встроенное преобразование ссылок react-markdown
 * (`defaultUrlTransform`): слоёв несколько намеренно, текст недоверенный.
 */
export function MarkdownContent({ source }: { source: string }) {
  return (
    <div className="flex flex-col gap-3 text-body wrap-anywhere [&_code]:rounded-xs [&_code]:bg-muted [&_code]:px-1 [&_code]:font-mono [&_code]:text-caption [&_h1]:text-h2 [&_h2]:text-h3 [&_h3]:font-semibold [&_ol]:ml-5 [&_ol]:list-decimal [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-(--code-bg) [&_pre]:p-3 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_ul]:ml-5 [&_ul]:list-disc">
      <ReactMarkdown skipHtml components={COMPONENTS} rehypePlugins={REHYPE_PLUGINS}>
        {source}
      </ReactMarkdown>
    </div>
  );
}
