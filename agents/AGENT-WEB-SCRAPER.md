# Web Scraper Agent

Browser automation and web scraping with structured data extraction.

## D-Bus Interface
`org.dbusmcp.Agent.WebScraper`

## Overview
Automate web browsing, extract structured data, and interact with web pages using Playwright/Puppeteer automation.

## Tools

### navigate
Load a webpage and wait for content.

**Input Schema:**
```json
{
  "url": "string (target URL)",
  "wait_for": "load|domcontentloaded|networkidle (default: load)",
  "timeout_ms": "integer (default: 30000)",
  "viewport": {
    "width": "integer (default: 1280)",
    "height": "integer (default: 720)"
  }
}
```

**Output:**
```json
{
  "status_code": "integer",
  "final_url": "string (after redirects)",
  "title": "string",
  "load_time_ms": "integer"
}
```

### extract
Extract structured data using CSS selectors or XPath.

**Input Schema:**
```json
{
  "selectors": {
    "field_name": {
      "selector": "string (CSS or XPath)",
      "type": "text|attribute|html",
      "attribute": "string (if type=attribute)",
      "multiple": "boolean (extract array)"
    }
  }
}
```

**Output:**
```json
{
  "data": {
    "field_name": "value or [values]"
  },
  "extracted_count": "integer"
}
```

**Example:**
```json
{
  "selectors": {
    "title": {"selector": "h1", "type": "text"},
    "links": {"selector": "a[href]", "type": "attribute", "attribute": "href", "multiple": true},
    "prices": {"selector": ".price", "type": "text", "multiple": true}
  }
}
```

### click
Click an element and wait for navigation/response.

**Input Schema:**
```json
{
  "selector": "string (CSS selector)",
  "wait_for_navigation": "boolean (default: false)",
  "timeout_ms": "integer"
}
```

### fill_form
Fill and submit forms.

**Input Schema:**
```json
{
  "fields": {
    "selector": "value"
  },
  "submit_selector": "string (optional)"
}
```

**Example:**
```json
{
  "fields": {
    "#search-input": "web scraping",
    "#filter-dropdown": "price_low_to_high"
  },
  "submit_selector": "#search-button"
}
```

### screenshot
Capture page screenshot.

**Input Schema:**
```json
{
  "selector": "string (optional, specific element)",
  "full_page": "boolean (default: false)",
  "format": "png|jpeg (default: png)",
  "quality": "integer (1-100, JPEG only)"
}
```

**Output:**
```json
{
  "data": "base64 (image data)",
  "format": "string",
  "width": "integer",
  "height": "integer"
}
```

### get_accessibility_tree
Get structured accessibility snapshot (Microsoft Playwright).

**Output:**
```json
{
  "tree": [
    {
      "role": "button|link|heading|...",
      "name": "string",
      "level": "integer (heading level)",
      "children": [...]
    }
  ]
}
```

### execute_script
Run JavaScript in page context.

**Input Schema:**
```json
{
  "script": "string (JavaScript code)",
  "args": ["any (optional arguments)"]
}
```

**Example:**
```json
{
  "script": "return document.querySelectorAll('.item').length;"
}
```

### wait_for_selector
Wait for element to appear.

**Input Schema:**
```json
{
  "selector": "string",
  "state": "visible|attached|hidden (default: visible)",
  "timeout_ms": "integer (default: 30000)"
}
```

## Example Usage

### Via D-Bus
```bash
# Navigate and extract
busctl call org.dbusmcp.Agent.WebScraper \
  /org/dbusmcp/agent/scraper_001 \
  org.dbusmcp.Agent.WebScraper \
  Execute s '{
    "task_type": "navigate",
    "url": "https://example.com/products",
    "wait_for": "networkidle"
  }'

busctl call org.dbusmcp.Agent.WebScraper \
  /org/dbusmcp/agent/scraper_001 \
  org.dbusmcp.Agent.WebScraper \
  Execute s '{
    "task_type": "extract",
    "selectors": {
      "product_names": {"selector": ".product-title", "type": "text", "multiple": true},
      "prices": {"selector": ".price", "type": "text", "multiple": true}
    }
  }'
```

### Via MCP
```json
{
  "method": "tools/call",
  "params": {
    "name": "extract",
    "arguments": {
      "selectors": {
        "headline": {"selector": "h1.article-title", "type": "text"},
        "author": {"selector": ".author-name", "type": "text"},
        "publish_date": {"selector": "time", "type": "attribute", "attribute": "datetime"},
        "paragraphs": {"selector": "article p", "type": "text", "multiple": true}
      }
    }
  }
}
```

## Security Features

### URL Restrictions
- Whitelist/blacklist support
- No `file://` protocol
- HTTPS enforced (configurable)
- Rate limiting per domain

### Resource Limits
- Maximum pages per session: 50
- Maximum concurrent browsers: 5
- Memory limit per browser: 512MB
- Execution timeout per action: 60s

### Privacy
- No cookies persisted by default
- Incognito/private mode
- No browser cache
- Optional proxy support

## Anti-Detection Features
- User-Agent rotation
- Randomized viewport sizes
- Human-like delays
- Stealth mode (Playwright stealth plugin)

## Configuration
- `BROWSER_HEADLESS`: true|false (default: true)
- `BROWSER_TYPE`: chromium|firefox|webkit
- `MAX_CONCURRENT_BROWSERS`: Default 5
- `ALLOWED_DOMAINS`: Whitelist file path
- `PROXY_URL`: HTTP/SOCKS proxy (optional)
- `USER_AGENT`: Custom UA string (optional)

## Error Handling

### Navigation Timeout
```json
{
  "error": "NavigationTimeout",
  "message": "Page load exceeded 30000ms",
  "url": "https://slow-site.example.com"
}
```

### Selector Not Found
```json
{
  "error": "SelectorNotFound",
  "selector": ".missing-element",
  "waited_ms": 5000
}
```

## Use Cases
- **Data Collection**: Price monitoring, content aggregation
- **Testing**: Automated UI testing
- **Research**: Academic data gathering
- **Monitoring**: Website change detection
- **Integration**: Third-party service automation

## Performance Notes
- Browser launch: ~2-5 seconds (cached: ~500ms)
- Page load: Varies (typically 1-10s)
- Screenshot: ~100-500ms
- Extraction: ~10-100ms per selector
- JavaScript execution: ~5-50ms

## Best Practices
1. Use specific selectors (CSS > XPath for performance)
2. Wait for network idle on dynamic content
3. Reuse browser instances when possible
4. Handle navigation failures gracefully
5. Respect robots.txt and rate limits
