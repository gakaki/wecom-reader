// Cloudflare Pages Function: same-origin RSS proxy for the WebAssembly build.
// Native builds talk to the feeds directly; browsers use /api/rss?source=...
// Each source has a primary URL and an optional fallback. We always try the
// primary (the official feed) first and only fall back when it is blocked.
const FEEDS = {
  // blog.v2ex.com/rss/ is an HTML article, not a feed; /rss.xml is the real feed.
  v2ex: {
    primary: 'https://blog.v2ex.com/rss.xml',
  },
  linuxdo: {
    // Official linux.do feed sits behind a Cloudflare managed challenge; a
    // programmatic fetch gets "Just a moment..." HTML, so fall back to a
    // community mirror that republishes the same topics as clean RSS.
    primary: 'https://linux.do/latest.rss',
    fallback: 'https://linuxdorss.longpink.com/latest.xml',
  },
};

const CORS = { 'access-control-allow-origin': '*' };
const HEADERS = {
  'user-agent': 'Mozilla/5.0 (compatible; WeComReader/1.0; +RSS)',
  accept: 'application/rss+xml, application/atom+xml, application/xml, text/xml, */*',
};

function looksLikeFeed(body) {
  const head = body.trimStart().slice(0, 512).toLowerCase();
  return head.includes('<rss') || head.includes('<feed') || head.includes('<?xml');
}

export async function onRequest(context) {
  const { request } = context;
  const source = new URL(request.url).searchParams.get('source');
  const cfg = FEEDS[source];

  if (!cfg) {
    return new Response('Unknown source', { status: 400, headers: CORS });
  }

  const urls = cfg.fallback ? [cfg.primary, cfg.fallback] : [cfg.primary];
  for (const upstream of urls) {
    try {
      const response = await fetch(upstream, {
        headers: HEADERS,
        cf: { cacheTtl: 120, cacheEverything: true },
      });
      const body = await response.text();
      if (response.ok && looksLikeFeed(body)) {
        return new Response(body, {
          status: 200,
          headers: {
            'content-type': 'application/rss+xml; charset=utf-8',
            ...CORS,
            'cache-control': 'public, max-age=120',
            'x-rss-source': upstream,
          },
        });
      }
      // Blocked/challenged/invalid — try the next URL if any.
    } catch (error) {
      // Network error — try the next URL if any.
    }
  }

  return new Response('RSS source unavailable', { status: 502, headers: CORS });
}
