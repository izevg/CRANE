import type { ComponentProps } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import { Link } from "../../primitives";

const NEW_TAB_HINT = " (откроется в новой вкладке)";
const EXTERNAL = /^https?:\/\//i;

const REHYPE_PLUGINS = [rehypeSanitize];

/** Текст недоверенный: ссылки идут через `Link` кита, внешние открываются в новой вкладке без `opener`. */
function MarkdownLink({ href, children }: ComponentProps<"a">) {
  if (href === undefined) {
    return <span>{children}</span>;
  }
  if (EXTERNAL.test(href)) {
    return (
      <Link href={href} target="_blank" rel="noopener noreferrer" className="text-brand underline">
        {children}
        <span className="sr-only">{NEW_TAB_HINT}</span>
      </Link>
    );
  }
  return (
    <Link href={href} className="text-brand underline">
      {children}
    </Link>
  );
}

const COMPONENTS: Components = { a: MarkdownLink };

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
