// Local preview for the WebAssembly build: static files + the same RSS proxy
// the Cloudflare Pages Function provides. Run: node dev-server.mjs [port]
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';

const root = new URL('./dist-web/', import.meta.url).pathname;
const port = Number(process.argv[2] || 4173);
// Primary feed first, mirror as fallback when the primary is challenged.
const feeds = {
  v2ex: ['https://blog.v2ex.com/rss.xml'],
  linuxdo: [
    'https://linux.do/latest.rss',
    'https://linuxdorss.longpink.com/latest.xml',
  ],
};
const looksLikeFeed = (body) => {
  const head = body.trimStart().slice(0, 512).toLowerCase();
  return head.includes('<rss') || head.includes('<feed') || head.includes('<?xml');
};
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json',
  '.png': 'image/png',
  '.br': 'application/octet-stream',
  '.gz': 'application/octet-stream',
};

createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${port}`);
  if (url.pathname === '/api/rss') {
    const targets = feeds[url.searchParams.get('source')];
    if (!targets) {
      res.writeHead(400).end('Unknown source');
      return;
    }
    for (const target of targets) {
      try {
        const upstream = await fetch(target, {
          headers: {
            'user-agent': 'Mozilla/5.0 (compatible; WeComReader/1.0; +RSS)',
            accept: 'application/rss+xml, application/atom+xml, application/xml, text/xml, */*',
          },
        });
        const body = await upstream.text();
        if (upstream.ok && looksLikeFeed(body)) {
          res.writeHead(200, {
            'content-type': 'application/rss+xml; charset=utf-8',
            'access-control-allow-origin': '*',
            'cache-control': 'public, max-age=120',
          });
          res.end(body);
          return;
        }
      } catch {
        // Try the next URL.
      }
    }
    res.writeHead(502).end('RSS source unavailable');
    return;
  }

  let path = normalize(decodeURIComponent(url.pathname)).replace(/^(\.\.[/\\])+/, '');
  if (path === '/' || path.endsWith('/')) path += 'index.html';
  let file = join(root, path);
  try {
    const info = await stat(file);
    if (info.isDirectory()) file = join(file, 'index.html');
    const data = await readFile(file);
    res.writeHead(200, {
      'content-type': types[extname(file)] || 'application/octet-stream',
      'cache-control': file.includes('index.html') ? 'no-cache' : 'public, max-age=31536000, immutable',
    });
    res.end(data);
  } catch {
    // Single-page fallback.
    try {
      const data = await readFile(join(root, 'index.html'));
      res.writeHead(200, { 'content-type': types['.html'], 'cache-control': 'no-cache' });
      res.end(data);
    } catch {
      res.writeHead(404).end('Not found');
    }
  }
}).listen(port, '127.0.0.1', () => {
  console.log(`WeCom Reader preview: http://127.0.0.1:${port}/`);
});
