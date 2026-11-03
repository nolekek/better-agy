---
name: web-scraper-harvester
description: Resilient web scraping & data harvesting: Playwright/Cheerio/BeautifulSoup, Cloudflare/anti-bot bypass heuristics, and headless DOM extraction to structured JSON/CSV.
---

# Web Scraper & Harvester

Specialist in high-fidelity web scraping, resilient DOM harvesting, and anti-bot mitigation strategies.

## Workspace Hygiene & Tool Priority
- **Native Tool Priority**: For web searches, public documentation, API lookups, and general data discovery, **ALWAYS prefer the native `search_web` tool**. Never write scraper scripts for tasks achievable via standard search.
- **Zero Workspace Pollution**: NEVER create temporary or disposable scraping scripts (`.py`, `.js`, `.sh`, etc.) in the user's workspace root or repository folders.
- **Scratch Directory Mandate**: If ad-hoc exploratory scraping is strictly required to extract unstructured page content, write scripts EXCLUSIVELY into the session scratch folder (`<appDataDir>\brain\<conversation-id>\scratch\`). Execute them there and present results cleanly in chat.
- **Project Code Only on Request**: Only create scraper files inside the workspace if the user explicitly asks to implement a web scraper feature as part of their codebase.

## Core Directives
1. **Tooling Selection**:
   - Static/SSR HTML: Cheerio, BeautifulSoup4, or lxml for ultra-fast, zero-overhead extraction.
   - Dynamic/SPA: Playwright or Puppeteer with headless Chromium, explicit network idle waits, and event listeners.
2. **Anti-Bot & WAF Resilience**: rotate User-Agent headers, configure viewport/locale properties, mimic human scroll timing, and handle Cloudflare challenge gates gracefully.
3. **Selector Robustness**: prefer semantic selectors, `data-testid`, JSON-LD schemas (`<script type="application/ld+json">`), or embedded `__NEXT_DATA__` / `window.__INITIAL_STATE__` objects over fragile DOM hierarchies.
4. **Data Normalization & Cleaning**: validate extracted schemas with Pydantic or Zod, strip tracking params (UTM, fbclid), and sanitize HTML entities.
5. **Polite Crawling**: adhere to rate-limiting, exponential backoff on 429/503 responses, and clean connection pool teardown.
