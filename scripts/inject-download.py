#!/usr/bin/env python3
"""Inject a multi-platform download panel into the built Web app's index.html.

Run this after `nsh package . --target web` and copying the output to dist-web.
Idempotent: re-running will not duplicate the injected block (old single-button
markers are removed first).
"""
import sys

BASE = "https://pub-b0f6a4366b7e4d00a76a30bb008e9416.r2.dev"
MARKER = "<!-- wecom-reader-download-panel -->"
OLD_MARKER = "<!-- wecom-reader-download-btn -->"

CSS = """
  /* Download panel (injected by scripts/inject-download.py) */
  #dl-panel { position: fixed; right: 18px; bottom: 18px; z-index: 2147483000;
    font: 500 14px/1 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; }
  #dl-toggle { display: inline-flex; align-items: center; gap: 8px; height: 44px; padding: 0 18px;
    border: 0; border-radius: 22px; background: #07c160; color: #fff; cursor: pointer;
    font: inherit; box-shadow: 0 4px 16px rgba(7,193,96,.38); transition: transform .15s, box-shadow .15s; }
  #dl-toggle:hover { transform: translateY(-1px); box-shadow: 0 6px 20px rgba(7,193,96,.48); }
  #dl-toggle svg { width: 20px; height: 20px; fill: currentColor; }
  #dl-menu { position: absolute; right: 0; bottom: 54px; min-width: 190px; padding: 6px;
    background: #fff; border: 1px solid rgba(0,0,0,.08); border-radius: 14px;
    box-shadow: 0 8px 30px rgba(0,0,0,.16); display: none; }
  #dl-panel.open #dl-menu { display: block; }
  #dl-menu a { display: flex; align-items: center; gap: 10px; padding: 10px 12px;
    border-radius: 9px; color: #1c1c1e; text-decoration: none; white-space: nowrap; }
  #dl-menu a:hover { background: #f2f3f5; }
  #dl-menu a .ico { width: 20px; text-align: center; font-size: 17px; }
  #dl-menu a .meta { margin-left: auto; font-size: 12px; color: #8e8e93; }
  @media (max-width: 600px) { #dl-panel { right: 12px; bottom: 12px; } }
"""

HTML = f"""{MARKER}
<div id="dl-panel">
  <div id="dl-menu">
    <a href="{BASE}/WeCom-Reader.apk" download="WeCom-Reader.apk"><span class="ico">🤖</span><span>Android</span><span class="meta">64 MB</span></a>
    <a href="{BASE}/WeCom-Reader-macOS.zip" download="WeCom-Reader-macOS.zip"><span class="ico">🍎</span><span>macOS</span><span class="meta">22 MB</span></a>
    <a href="{BASE}/WeCom-Reader-Windows.zip" download="WeCom-Reader-Windows.zip"><span class="ico">🪟</span><span>Windows</span><span class="meta">19 MB</span></a>
  </div>
  <button id="dl-toggle" type="button" aria-label="下载客户端">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3a1 1 0 0 1 1 1v10.59l3.3-3.3a1 1 0 0 1 1.4 1.42l-5 5a1 1 0 0 1-1.4 0l-5-5a1 1 0 1 1 1.4-1.42l3.3 3.3V4a1 1 0 0 1 1-1zM5 20a1 1 0 0 1 1 1H4a1 1 0 0 1-1-1h2zm14 0h2a1 1 0 0 1-1 1h-1a1 1 0 0 1 0-1z"/></svg>
    <span>下载客户端</span>
  </button>
</div>
<script>
(function () {{
  var p = document.getElementById('dl-panel');
  var t = document.getElementById('dl-toggle');
  if (p && t) t.addEventListener('click', function (e) {{
    e.stopPropagation(); p.classList.toggle('open');
  }});
  document.addEventListener('click', function () {{ if (p) p.classList.remove('open'); }});
}})();
</script>
"""


def strip_old(html: str) -> str:
    """Remove any previously injected block, from its marker up to its closing tag/script."""
    # Remove old single-button block: marker .. </a>
    if OLD_MARKER in html:
        start = html.index("<!-- wecom-reader-download-btn -->")
        # old block ended right after the </a> line
        end = html.find("</a>", start)
        if end != -1:
            html = html[:start].rstrip("\n") + "\n" + html[end + 4:].lstrip("\n")
    # Remove old CSS rule block if present (from comment to the media query line)
    old_css_start = html.find("  /* Android download button (injected")
    if old_css_start != -1:
        media = html.find("@media (max-width: 600px) { #dl-android", old_css_start)
        if media != -1:
            eol = html.find("\n", media)
            html = html[:old_css_start] + html[eol + 1:]
    return html


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "dist-web/index.html"
    with open(path, "r", encoding="utf-8") as f:
        html = f.read()

    if MARKER in html:
        print("Already injected; nothing to do.")
        return

    html = strip_old(html)

    if "</style>" not in html:
        sys.exit("ERROR: no </style> in " + path)
    if '<div id="app"></div>' not in html:
        sys.exit("ERROR: no #app div in " + path)

    html = html.replace("</style>", CSS + "\n</style>", 1)
    html = html.replace('<div id="app"></div>', '<div id="app"></div>\n' + HTML, 1)

    with open(path, "w", encoding="utf-8") as f:
        f.write(html)
    print(f"Injected download panel into {path}")


if __name__ == "__main__":
    main()
