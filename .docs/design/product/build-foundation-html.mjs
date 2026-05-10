import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

const docs = [
  {
    source: "MISSION.md",
    output: "mission.html",
    label: "Mission",
    tone: "turmeric",
    icon: "star",
    summary: "DailyOS makes intelligence personal. Memory plus judgment, grounded in context when it matters most.",
  },
  {
    source: "VISION.md",
    output: "vision.html",
    label: "Vision",
    tone: "larkspur",
    icon: "eye",
    summary: "Open the app. Your day is ready. The product experience and ecosystem shape.",
  },
  {
    source: "PRODUCT-THESIS.md",
    output: "product-thesis.html",
    label: "Product Thesis",
    tone: "eucalyptus",
    icon: "crosshair",
    summary: "The category argument: personal intelligence under uncertainty, not disposable output.",
  },
  {
    source: "PHILOSOPHY.md",
    output: "philosophy.html",
    label: "Philosophy",
    tone: "terracotta",
    icon: "lightbulb",
    summary: "The manifesto for zero-guilt, AI-native productivity, and user-owned context.",
  },
  {
    source: "PRINCIPLES.md",
    output: "principles.html",
    label: "Principles",
    tone: "olive",
    icon: "compass",
    summary: "The decision rules that keep DailyOS prepared, proactive, and low-maintenance.",
  },
  {
    source: "DOS-ENGINE-CONCEPT.md",
    output: "dos-engine-concept.html",
    label: "DOS Engine",
    tone: "olive",
    icon: "network",
    summary: "The portable trust-oriented runtime underneath the app surface.",
  },
];

const iconFor = (index) => [
  "star",
  "eye",
  "crosshair",
  "lightbulb",
  "compass",
  "network",
][index % 6];

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function slugify(value) {
  return value
    .toLowerCase()
    .replace(/<[^>]+>/g, "")
    .replace(/&[a-z]+;/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 80) || "section";
}

function inlineMarkdown(raw) {
  const code = [];
  let value = escapeHtml(raw).replace(/`([^`]+)`/g, (_, text) => {
    const token = `@@CODE${code.length}@@`;
    code.push(`<code>${text}</code>`);
    return token;
  });

  value = value.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
  value = value.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  value = value.replace(/\*([^*]+)\*/g, "<em>$1</em>");
  value = value.replace(/_(.*?)_/g, "<em>$1</em>");

  code.forEach((html, index) => {
    value = value.replace(`@@CODE${index}@@`, html);
  });
  return value;
}

function splitTableRow(line) {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function isTableSeparator(line) {
  return /^\s*\|?[\s:-]+\|[\s|:-]+\s*$/.test(line);
}

function renderTable(lines, start) {
  const header = splitTableRow(lines[start]);
  let i = start + 2;
  const body = [];
  while (i < lines.length && /^\s*\|/.test(lines[i])) {
    body.push(splitTableRow(lines[i]));
    i += 1;
  }
  const html = [
    "<table>",
    "<thead><tr>",
    ...header.map((cell) => `<th>${inlineMarkdown(cell)}</th>`),
    "</tr></thead>",
    "<tbody>",
    ...body.map((row) => `<tr>${row.map((cell) => `<td>${inlineMarkdown(cell)}</td>`).join("")}</tr>`),
    "</tbody>",
    "</table>",
  ].join("");
  return { html, next: i };
}

function paragraphFrom(lines) {
  return `<p>${inlineMarkdown(lines.join(" "))}</p>`;
}

function renderMarkdown(markdown, { skipFirstTitle = true } = {}) {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const html = [];
  const headings = [];
  let i = 0;
  let skippedTitle = false;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (!trimmed) {
      i += 1;
      continue;
    }

    if (/^```/.test(trimmed)) {
      const lang = trimmed.slice(3).trim();
      const code = [];
      i += 1;
      while (i < lines.length && !/^```/.test(lines[i].trim())) {
        code.push(lines[i]);
        i += 1;
      }
      i += 1;
      html.push(`<pre><code${lang ? ` data-language="${escapeHtml(lang)}"` : ""}>${escapeHtml(code.join("\n"))}</code></pre>`);
      continue;
    }

    if (/^---+$/.test(trimmed)) {
      html.push("<hr>");
      i += 1;
      continue;
    }

    const headingMatch = /^(#{1,6})\s+(.+)$/.exec(line);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const text = inlineMarkdown(headingMatch[2].trim());
      const rawText = headingMatch[2].trim();
      if (skipFirstTitle && level === 1 && !skippedTitle) {
        skippedTitle = true;
        i += 1;
        continue;
      }
      const id = slugify(rawText);
      if (level === 2) headings.push({ id, text: rawText });
      html.push(`<h${level} id="${id}">${text}</h${level}>`);
      i += 1;
      continue;
    }

    if (trimmed.startsWith(">")) {
      const quote = [];
      while (i < lines.length && lines[i].trim().startsWith(">")) {
        quote.push(lines[i].replace(/^\s*>\s?/, ""));
        i += 1;
      }
      html.push(`<blockquote>${paragraphFrom(quote)}</blockquote>`);
      continue;
    }

    if (/^\s*\|/.test(line) && i + 1 < lines.length && isTableSeparator(lines[i + 1])) {
      const table = renderTable(lines, i);
      html.push(table.html);
      i = table.next;
      continue;
    }

    if (/^\s*[-*]\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*]\s+/, ""));
        i += 1;
      }
      html.push(`<ul>${items.map((item) => `<li>${inlineMarkdown(item)}</li>`).join("")}</ul>`);
      continue;
    }

    if (/^\s*\d+\.\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ""));
        i += 1;
      }
      html.push(`<ol>${items.map((item) => `<li>${inlineMarkdown(item)}</li>`).join("")}</ol>`);
      continue;
    }

    const para = [line.trim()];
    i += 1;
    while (
      i < lines.length &&
      lines[i].trim() &&
      !/^(#{1,6})\s+/.test(lines[i]) &&
      !/^```/.test(lines[i].trim()) &&
      !/^\s*[-*]\s+/.test(lines[i]) &&
      !/^\s*\d+\.\s+/.test(lines[i]) &&
      !/^\s*>/.test(lines[i]) &&
      !/^\s*\|/.test(lines[i]) &&
      !/^---+$/.test(lines[i].trim())
    ) {
      para.push(lines[i].trim());
      i += 1;
    }
    html.push(paragraphFrom(para));
  }

  return { html: html.join("\n"), headings };
}

function titleFromMarkdown(markdown, fallback) {
  return /^#\s+(.+)$/m.exec(markdown)?.[1].trim() || fallback;
}

function firstQuote(markdown) {
  const quoteLines = [];
  for (const line of markdown.split("\n")) {
    if (line.trim().startsWith(">")) {
      quoteLines.push(line.replace(/^\s*>\s?/, "").trim());
      continue;
    }
    if (quoteLines.length && line.trim() === ">") continue;
    if (quoteLines.length) break;
  }
  return quoteLines.join(" ").replace(/\s+/g, " ").trim();
}

function firstParagraph(markdown) {
  const stripped = markdown
    .replace(/^#\s+.+$/m, "")
    .replace(/^>.*$/gm, "")
    .replace(/^---+$/gm, "")
    .trim();
  return stripped.split(/\n\s*\n/)[0]?.replace(/\n/g, " ").trim() || "";
}

function head(title) {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>${escapeHtml(title)} · DailyOS Foundation</title>
  <link rel="stylesheet" href="../reference/_shared/fonts.css">
  <link rel="stylesheet" href="../reference/_shared/styles/design-tokens.css">
  <link rel="stylesheet" href="../reference/_shared/styles/AtmosphereLayer.module.css">
  <link rel="stylesheet" href="../reference/_shared/styles/FolioBar.module.css">
  <link rel="stylesheet" href="../reference/_shared/styles/FloatingNavIsland.module.css">
  <link rel="stylesheet" href="../reference/_shared/styles/MagazinePageLayout.module.css">
  <link rel="stylesheet" href="../reference/_shared/chrome.css">
  <link rel="stylesheet" href="../reference/_shared/inspector.css">
  <link rel="stylesheet" href="foundation.css">
</head>`;
}

function bodyAttrs(doc, title, chapters = "") {
  return `<body
  data-folio-label="Foundation"
  data-folio-crumbs="Reference > Product Foundation > ${escapeHtml(doc.label)}"
  data-folio-status="Memory plus judgment"
  data-tint="${doc.tone}"
  data-active-page="today"
  data-nav-base="../reference/surfaces"
  data-chapters="${escapeHtml(chapters)}">`;
}

function brandSvg() {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 433 407" width="16" height="16" aria-hidden="true"><path d="M159 407 161 292 57 355 0 259 102 204 0 148 57 52 161 115 159 0H273L271 115L375 52L433 148L331 204L433 259L375 355L271 292L273 407Z" fill="currentColor"/></svg>`;
}

function topnav(activeOutput = "") {
  const links = docs
    .map((doc) => `<a href="${doc.output}"${doc.output === activeOutput ? ' data-active="true"' : ""}>${doc.label}</a>`)
    .join("\n");
  return `<nav class="foundation-topnav" aria-label="Product foundation">
  <a class="foundation-mark" href="index.html">${brandSvg()} Product foundation</a>
  <div class="foundation-links">${links}</div>
</nav>`;
}

function finis() {
  return `<section class="finis">
  <div class="finis-mark">✱ ✱ ✱</div>
  <p class="finis-caption">Memory plus judgment.</p>
</section>`;
}

function pageShell(content) {
  return `<div class="MagazinePageLayout_magazinePage">
<main class="MagazinePageLayout_pageContainer">
<div class="foundation-shell">
${content}
</div>
</main>
</div>
<script src="../reference/_shared/chrome.js"></script>
<script src="../reference/_shared/inspector.js"></script>
</body>
</html>
`;
}

function documentPage(doc, index) {
  const markdown = readFileSync(join(here, doc.source), "utf8");
  const title = titleFromMarkdown(markdown, doc.label);
  const quote = firstQuote(markdown);
  const lede = quote || firstParagraph(markdown) || doc.summary;
  const rendered = renderMarkdown(markdown);
  const chapters = rendered.headings
    .slice(0, 8)
    .map((heading, chapterIndex) => `${heading.id}:${iconFor(chapterIndex)}:${heading.text}`)
    .join("|");
  const toc = rendered.headings.length
    ? `<aside class="foundation-toc"><p class="foundation-toc-title">In this document</p>${rendered.headings.map((heading) => `<a href="#${heading.id}">${escapeHtml(heading.text)}</a>`).join("")}</aside>`
    : `<aside class="foundation-toc"><p class="foundation-toc-title">Foundation</p><a href="index.html">Back to index</a></aside>`;
  const prev = docs[(index + docs.length - 1) % docs.length];
  const next = docs[(index + 1) % docs.length];

  return `${head(title)}
${bodyAttrs(doc, title, chapters)}
${pageShell(`${topnav(doc.output)}
<section class="foundation-cover">
  <p class="foundation-eyebrow">${escapeHtml(doc.label)} · Product foundation</p>
  <h1 class="foundation-title">${escapeHtml(title)}</h1>
  <p class="foundation-lede">${inlineMarkdown(lede)}</p>
  <div class="foundation-meta">
    <span>${rendered.headings.length} sections</span>
    <span>Source: ${escapeHtml(doc.source)}</span>
  </div>
</section>
<div class="foundation-layout">
  ${toc}
  <article class="foundation-document">
    ${rendered.html}
    <nav class="foundation-next" aria-label="Adjacent foundation documents">
      <a href="${prev.output}"><span class="foundation-next-label">Previous</span><span class="foundation-next-title">${escapeHtml(prev.label)}</span></a>
      <a href="${next.output}"><span class="foundation-next-label">Next</span><span class="foundation-next-title">${escapeHtml(next.label)}</span></a>
    </nav>
  </article>
</div>
${finis()}`)}`;
}

function indexPage() {
  const cards = docs.map((doc, index) => {
    const markdown = readFileSync(join(here, doc.source), "utf8");
    const title = titleFromMarkdown(markdown, doc.label);
    const sectionCount = renderMarkdown(markdown).headings.length;
    return `<a class="foundation-card" href="${doc.output}">
  <span class="foundation-card-index">${String(index + 1).padStart(2, "0")} · ${escapeHtml(doc.label)}</span>
  <h2 class="foundation-card-title">${escapeHtml(title)}</h2>
  <p class="foundation-card-text">${escapeHtml(doc.summary)}</p>
  <p class="foundation-card-meta">${sectionCount} sections · ${escapeHtml(doc.source)}</p>
</a>`;
  }).join("\n");

  const doc = { label: "Index", tone: "turmeric" };
  return `${head("DailyOS Product Foundation")}
${bodyAttrs(doc, "Index")}
${pageShell(`${topnav("index.html")}
<section class="foundation-cover">
  <p class="foundation-eyebrow">DailyOS · Product foundation</p>
  <h1 class="foundation-title">Memory plus judgment.</h1>
  <p class="foundation-kicker">The durable context layer underneath the app surface.</p>
  <p class="foundation-lede">These documents define why DailyOS exists, what it refuses, how it should behave, and why personal intelligence is more than a second brain on Markdown.</p>
  <div class="foundation-meta">
    <span>${docs.length} foundation docs</span>
    <span>Design system HTML</span>
    <span>Generated from Markdown</span>
  </div>
</section>
<section class="foundation-grid" aria-label="Foundation documents">
${cards}
</section>
${finis()}`)}`;
}

mkdirSync(here, { recursive: true });
writeFileSync(join(here, "index.html"), indexPage(), "utf8");
docs.forEach((doc, index) => {
  writeFileSync(join(here, doc.output), documentPage(doc, index), "utf8");
});

console.log(`Generated ${docs.length + 1} foundation HTML files in ${basename(here)}/`);
