//! HTML-представление (PEP 503, атрибуты PEP 592 и PEP 345).

use super::{DistFile, Project, Yanked};

/// Версия репозитория по PEP 629: поля `data-*`, которые мы отдаём, входят в 1.1.
const REPOSITORY_VERSION: &str = "1.1";

pub(super) fn render(project: &Project) -> String {
    let title = escape(project.name.normalized());
    let mut out = format!(
        "<!DOCTYPE html>\n<html>\n<head>\n\
         <meta name=\"pypi:repository-version\" content=\"{REPOSITORY_VERSION}\">\n\
         <title>Links for {title}</title>\n</head>\n<body>\n<h1>Links for {title}</h1>\n"
    );
    for file in &project.files {
        out.push_str(&anchor(file));
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn anchor(file: &DistFile) -> String {
    let mut attributes = vec![format!(
        "href=\"{}#sha256={}\"",
        escape(&file.url),
        file.sha256
    )];
    if let Some(requires_python) = &file.requires_python {
        attributes.push(format!(
            "data-requires-python=\"{}\"",
            escape(requires_python)
        ));
    }
    match &file.yanked {
        Yanked::No => {}
        Yanked::Yes(None) => attributes.push("data-yanked".to_owned()),
        Yanked::Yes(Some(reason)) => {
            attributes.push(format!("data-yanked=\"{}\"", escape(reason)));
        }
    }
    format!(
        "<a {}>{}</a><br/>\n",
        attributes.join(" "),
        escape(&file.filename)
    )
}

/// Экранирование и для текста, и для значения в двойных кавычках: `<` и `>` в
/// `data-requires-python` PEP 503 требует экранировать, остальное защищает от выхода из атрибута.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}
