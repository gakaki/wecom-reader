// Optional Workers Static Assets host (the active deployment uses Pages —
// see functions/api/rss.js). Kept in sync with the same-origin RSS proxy.
const FEEDS = {
  v2ex: { primary: 'https://blog.v2ex.com/rss.xml' },
  linuxdo: {
    primary: 'https://linux.do/latest.rss',
    fallback: 'https://linuxdorss.longpink.com/latest.xml',
  },
};

const cors = { 'access-control-allow-origin': '*' };
const looksLikeFeed = (body) => {
  const head = body.trimStart().slice(0, 512).toLowerCase();
  return head.includes('<rss') || head.includes('<feed') || head.includes('<?xml');
};

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname === '/api/rss') {
      const cfg = FEEDS[url.searchParams.get('source')];
      if (!cfg) return new Response('Unknown source', { status: 400, headers: cors });

      const urls = cfg.fallback ? [cfg.primary, cfg.fallback] : [cfg.primary];
      for (const target of urls) {
        try {
          const upstream = await fetch(target, {
            headers: {
              'user-agent': 'Mozilla/5.0 (compatible; WeComReader/1.0; +RSS)',
              accept: 'application/rss+xml, application/atom+xml, application/xml, text/xml, */*',
            },
            cf: { cacheTtl: 120, cacheEverything: true },
          });
          const body = await upstream.text();
          if (upstream.ok && looksLikeFeed(body)) {
            return new Response(body, {
              status: 200,
              headers: {
                ...cors,
                'content-type': 'application/rss+xml; charset=utf-8',
                'cache-control': 'public, max-age=120',
                'x-rss-source': target,
              },
            });
          }
        } catch {
          // Try next URL.
        }
      }
      return new Response('RSS source unavailable', { status: 502, headers: cors });
    }

    return env.ASSETS.fetch(request);
  },
};
