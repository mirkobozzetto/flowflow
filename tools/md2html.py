#!/usr/bin/env python3
"""Render a Markdown file (GFM tables + mermaid diagrams) to a readable HTML page.

Usage: python3 tools/md2html.py [file.md]
Defaults to RFC 0009. Writes to /tmp/flowflow-md/<stem>.html and opens it.
Rendering is client-side (marked + mermaid via CDN), so the .md is embedded
verbatim as base64 - no escaping pitfalls, no local markdown dependency.
"""
import sys, os, base64, subprocess, pathlib

DEFAULT = "docs/rfcs/0009-user-accounts-premium-entitlements-admin-iap/RFC.md"


def strip_frontmatter(text: str) -> str:
    if text.startswith("---"):
        lines = text.split("\n")
        for i in range(1, len(lines)):
            if lines[i].strip() == "---":
                return "\n".join(lines[i + 1:]).lstrip("\n")
    return text


TEMPLATE = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>__TITLE__</title>
<style>
  :root {
    --bg: #faf8f5; --fg: #2a2520; --muted: #8a8078; --accent: #e85d0a;
    --card: #fffdfb; --line: #e7e0d6; --code-bg: #f3efe9;
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; background: var(--bg); color: var(--fg);
    font: 16px/1.65 -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
  }
  header.bar {
    position: sticky; top: 0; z-index: 5;
    background: rgba(250,248,245,.85); backdrop-filter: blur(8px);
    border-bottom: 1px solid var(--line); padding: 12px 24px;
    font-size: 13px; color: var(--muted);
  }
  header.bar b { color: var(--accent); }
  main { max-width: 880px; margin: 0 auto; padding: 32px 24px 96px; }
  h1, h2, h3, h4 { line-height: 1.25; font-weight: 700; margin: 1.8em 0 .6em; }
  h1 { font-size: 2rem; margin-top: .3em; padding-bottom: .3em; border-bottom: 3px solid var(--accent); }
  h2 { font-size: 1.5rem; padding-bottom: .2em; border-bottom: 1px solid var(--line); }
  h3 { font-size: 1.2rem; } h4 { font-size: 1.05rem; color: #4a423a; }
  a { color: var(--accent); }
  p, li { overflow-wrap: anywhere; }
  code { background: var(--code-bg); padding: .12em .4em; border-radius: 4px; font-size: .88em;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  pre { background: var(--code-bg); border: 1px solid var(--line); border-radius: 8px;
    padding: 14px 16px; overflow-x: auto; }
  pre code { background: none; padding: 0; font-size: .85em; }
  blockquote { margin: 1em 0; padding: .6em 1em; border-left: 4px solid var(--accent);
    background: #fff5ee; border-radius: 0 8px 8px 0; }
  table { border-collapse: collapse; width: 100%; margin: 1em 0; font-size: .9em; display: block; overflow-x: auto; }
  th, td { border: 1px solid var(--line); padding: 8px 10px; text-align: left; vertical-align: top; }
  th { background: #f0e9e0; font-weight: 600; }
  tr:nth-child(even) td { background: #fdfbf8; }
  pre.mermaid { background: var(--card); border: 1px solid var(--line); text-align: center; }
  hr { border: none; border-top: 1px solid var(--line); margin: 2em 0; }
</style>
</head>
<body>
<header class="bar">rendu de <b>__TITLE__.md</b> - tables + diagrammes mermaid rendus localement</header>
<main id="doc">Chargement...</main>
<script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"></script>
<script>
  const B64 = "__B64__";
  const md = new TextDecoder().decode(Uint8Array.from(atob(B64), c => c.charCodeAt(0)));
  const doc = document.getElementById("doc");
  marked.setOptions({ gfm: true });
  doc.innerHTML = marked.parse(md);
  doc.querySelectorAll("pre code.language-mermaid").forEach(el => {
    const div = document.createElement("pre");
    div.className = "mermaid";
    div.textContent = el.textContent;
    el.closest("pre").replaceWith(div);
  });
  mermaid.initialize({ startOnLoad: false, theme: "neutral", securityLevel: "loose" });
  mermaid.run({ querySelector: "pre.mermaid" }).catch(() => {});
</script>
</body>
</html>
"""


def main() -> None:
    src = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else DEFAULT)
    md = strip_frontmatter(pathlib.Path(src).read_text(encoding="utf-8"))
    b64 = base64.b64encode(md.encode("utf-8")).decode("ascii")
    stem = pathlib.Path(src).stem
    out_dir = "/tmp/flowflow-md"
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, stem + ".html")
    pathlib.Path(out).write_text(
        TEMPLATE.replace("__B64__", b64).replace("__TITLE__", stem), encoding="utf-8"
    )
    print(out)
    subprocess.run(["open", out], check=False)


if __name__ == "__main__":
    main()
