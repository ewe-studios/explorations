# X-ray Web Scraper Deep Dive

## Overview

X-ray is a flexible web scraping library with powerful selector syntax, pagination support, and crawler capabilities. It provides a declarative API for defining what data to extract, handling the complexities of HTTP requests, HTML parsing, and data transformation automatically.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      X-ray Library                           │
├─────────────────────────────────────────────────────────────┤
│  Public API                                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ Xray(options)                                         │   │
│  │   .delay() .concurrency() .throttle() .limit()       │   │
│  │   .paginate() .write() .then() .stream()             │   │
│  └──────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  Core Engine                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Parser     │  │   Resolver   │  │   Walker     │      │
│  │  (selector)  │  │  (URLs)      │  │  (traverse)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├─────────────────────────────────────────────────────────────┤
│  Crawler Layer                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Request    │  │   Response   │  │   Cheerio    │      │
│  │   Queue      │  │   Handler    │  │   Parser     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Selector Syntax

X-ray uses a powerful selector syntax that combines CSS selectors with attribute extraction:

```javascript
// Basic text selection
x('https://example.com', 'title')

// Select by class
x('https://reddit.com', '.content')

// Select attribute (href, src, etc.)
x('https://techcrunch.com', 'img.logo@src')

// Select innerHTML
x('https://news.ycombinator.com', 'body@html')

// Complex selectors
x('https://dribbble.com', 'li.group', [
  {
    title: '.dribbble-img strong',
    image: '.dribbble-img [data-src]@data-src'
  }
])
```

### The @ Symbol

The `@` symbol in selectors indicates attribute extraction:

| Selector | Extracts |
|----------|----------|
| `'h1'` | Text content |
| `'a@href'` | href attribute |
| `'img@src'` | src attribute |
| `'body@html'` | Inner HTML |
| `'meta@content'` | Content attribute |

## Main Module

The core X-ray module orchestrates the scraping process:

```javascript
// x-ray/index.js
function Xray(options) {
  var crawler = Crawler()
  options = options || {}
  var filters = options.filters || {}

  function xray(source, scope, selector) {
    var args = params(source, scope, selector)
    selector = args.selector
    source = args.source
    scope = args.context

    var state = objectAssign({}, CONST.INIT_STATE)
    var store = enstore()
    var pages = []
    var stream

    var walkHTML = WalkHTML(xray, selector, scope, filters)
    var request = Request(crawler)

    function node(source2, fn) {
      // Handle source resolution
      if (isUrl(source)) {
        request(source, function(err, html) {
          if (err) return next(err)
          var $ = load(html, source)
          walkHTML($, next)
        })
      } else if (scope && ~scope.indexOf('@')) {
        // Resolve attribute-based URLs (e.g., a@href)
        var url = resolve(source, false, scope, filters)
        if (!isUrl(url)) {
          return walkHTML(load(''), next)
        }
        request(url, function(err, html) {
          var $ = load(html, url)
          walkHTML($, next)
        })
      } else {
        // Static HTML or null source
        var $ = load(source)
        walkHTML($, next)
      }

      function next(err, obj, $) {
        if (err) return fn(err)
        
        var paginate = state.paginate
        var limit = --state.limit

        // Create stream if not exists
        if (!stream) {
          if (paginate) stream = streamHelper.array(state.stream)
          else stream = streamHelper.object(state.stream)
        }

        // Handle pagination
        if (paginate) {
          if (isArray(obj)) {
            pages = pages.concat(obj)
          } else {
            pages.push(obj)
          }

          if (limit <= 0) {
            stream(obj, true)
            return fn(null, pages)
          }

          // Get next page URL
          var url = resolve($, false, paginate, filters)
          if (!isUrl(url)) {
            stream(obj, true)
            return fn(null, pages)
          }

          if (state.abort && state.abort(obj, url)) {
            stream(obj, true)
            return fn(null, pages)
          }

          stream(obj)
          request(url, function(err, html) {
            if (err) return next(err)
            var $ = load(html, url)
            walkHTML($, next)
          })
        } else {
          stream(obj, true)
          fn(null, obj)
        }
      }

      return node
    }

    // Chainable methods
    node.paginate = function(paginate) {
      if (!arguments.length) return state.paginate
      state.paginate = paginate
      return node
    }

    node.limit = function(limit) {
      if (!arguments.length) return state.limit
      state.limit = limit
      return node
    }

    node.write = function(path) {
      if (!arguments.length) return node.stream()
      state.stream = fs.createWriteStream(path)
      streamHelper.waitCb(state.stream, node)
      return state.stream
    }

    node.then = function(resHandler, errHandler) {
      return streamToPromise(node.stream()).then(resHandler, errHandler)
    }

    return node
  }

  // Delegate crawler methods
  CONST.CRAWLER_METHODS.forEach(function(method) {
    xray[method] = function() {
      if (!arguments.length) return crawler[method]()
      crawler[method].apply(crawler, arguments)
      return this
    }
  })

  return xray
}
```

## Request System

### Crawler Integration

X-ray delegates HTTP requests to `x-ray-crawler`:

```javascript
function Request(crawler) {
  return function request(url, fn) {
    debug('fetching %s', url)
    crawler(url, function(err, ctx) {
      if (err) return fn(err)
      debug('got response for %s with status code: %s', url, ctx.status)
      return fn(null, ctx.body)
    })
  }
}
```

### Crawler Configuration

```javascript
// Crawler methods available on X-ray instance
CONST.CRAWLER_METHODS = [
  'concurrency',   // Max concurrent requests
  'throttle',      // Requests per second limit
  'timeout',       // Request timeout in ms
  'delay',         // Delay between requests
  'limit',         // Max pages to crawl
  'abort',         // Condition to stop crawling
  'driver'         // Custom HTTP driver
]

// Usage example
var x = Xray()
  .delay('1s', '5s')      // Random delay 1-5 seconds
  .concurrency(3)         // Max 3 concurrent requests
  .throttle(10, '1s')     // Max 10 requests per second
  .timeout(30000)         // 30 second timeout
```

## HTML Parsing with Cheerio

### Loading HTML

```javascript
function load(html, url) {
  html = html || ''
  var $ = html.html ? html : cheerio.load(html, { decodeEntities: false })
  if (url) $ = absolutes(url, $)
  return $
}
```

### Absolute URL Conversion

X-ray automatically converts relative URLs to absolute:

```javascript
// lib/absolutes.js
function absolute(path, $) {
  var parts = url.parse(path)
  var remote = parts.protocol + '//' + parts.host
  
  // Handle <base> tags
  var base = $('head').find('base')
  if (base.length === 1) {
    var href = base.attr('href')
    if (href) {
      remote = href
    }
  }
  
  // Selectors to process
  var selector = [
    'a[href]', 'img[src]', 'script[src]', 
    'link[href]', 'source[src]', 'iframe[src]'
  ].join(',')
  
  $(selector).each(abs)
  
  function abs(i, el) {
    var $el = $(el)
    var key = $el.attr('href') ? 'href' : 'src'
    var src = $el.attr(key).trim()
    
    // Already absolute
    if (~src.indexOf('://')) return
    
    // Make absolute
    var current = url.resolve(remote, parts.pathname)
    src = url.resolve(current, src)
    $el.attr(key, src)
  }
  
  return $
}
```

## Selector Resolution

### Parameter Parsing

```javascript
// lib/params.js
function params(source, context, selector) {
  var args = {}
  
  if (undefined === context) {
    // x('selector')
    args.source = null
    args.context = null
    args.selector = source
  } else if (undefined === selector) {
    // x(url, 'selector') or x(html, 'selector')
    if (isUrl(source) || source.html || isHTML(source)) {
      args.source = source
      args.context = null
      args.selector = context
    } else {
      args.source = null
      args.context = source
      args.selector = context
    }
  } else {
    // x(url, scope, 'selector')
    args.source = source
    args.context = context
    args.selector = selector
  }
  
  return args
}
```

### Attribute Resolution

```javascript
// lib/resolve.js
function resolve($, scope, selector, filters) {
  filters = filters || {}
  var array = isArray(selector)
  var obj = parse(array ? selector[0] : selector)
  
  // Default to text content
  obj.attribute = obj.attribute || 'text'
  
  if (!obj.selector) {
    obj.selector = scope
    scope = null
  }
  
  // Find matching elements
  var value = find($, scope, array ? [obj.selector] : obj.selector, obj.attribute)
  
  // Apply filters
  if (array && typeof value.map === 'function') {
    value = value.map(function(v) {
      return filter(obj, $, scope, selector, v, filters)
    })
  } else {
    value = filter(obj, $, scope, selector, value, filters)
  }
  
  return value
}

function find($, scope, selector, attr) {
  if (scope && isArray(selector)) {
    // Collection within scoped context
    var $scope = select($, scope)
    var out = []
    $scope.map(function(i) {
      var $el = $scope.eq(i)
      var $children = select($el, selector[0])
      $children.map(function(i) {
        out.push(attribute($children.eq(i), attr))
      })
    })
    return out
  } else if (scope) {
    // Single element within scope
    $scope = select($, scope)
    return attribute($scope.find(selector).eq(0), attr)
  } else {
    // Global selector
    var $selector = select($, selector)
    return attribute($selector.eq(0), attr)
  }
}

function attribute($el, attr) {
  switch (attr) {
    case 'html': return $el.html()
    case 'text': return $el.text()
    default: return $el.attr(attr)
  }
}
```

## Walk Engine

The walk engine traverses nested selector structures:

```javascript
// lib/walk.js
function walk(value, fn, done, key) {
  var batch = Batch()
  var out

  if (isObject(value)) {
    // Walk object properties
    out = {}
    Object.keys(value).forEach(function(k) {
      var v = value[k]
      batch.push(function(next) {
        walk(v, fn, function(err, value) {
          if (err) return next(err)
          // Ignore undefined values
          if (undefined !== value && value !== '') {
            out[k] = value
          }
          next()
        }, k)
      })
    })
  } else {
    // Process leaf node
    out = null
    batch.push(function(next) {
      fn(value, key, function(err, v) {
        if (err) return next(err)
        out = v
        next()
      })
    })
  }

  batch.end(function(err) {
    if (err) return done(err)
    return done(null, out)
  })
}
```

## WalkHTML Implementation

```javascript
// x-ray/index.js
function WalkHTML(xray, selector, scope, filters) {
  return function walkHTML($, fn) {
    walk(selector, function(v, k, next) {
      // String selector - resolve directly
      if (typeof v === 'string') {
        var value = resolve($, root(scope), v, filters)
        return next(null, value)
      
      // Function selector - custom extraction
      } else if (typeof v === 'function') {
        return v($, function(err, obj) {
          if (err) return next(err)
          return next(null, obj)
        })
      
      // Array selector - collection
      } else if (isArray(v)) {
        if (typeof v[0] === 'string') {
          // Simple array of strings
          return next(null, resolve($, root(scope), v, filters))
        } else if (typeof v[0] === 'object') {
          // Array of objects - iterate over DOM elements
          var $scope = $.find ? $.find(scope) : $(scope)
          var pending = $scope.length
          var out = []

          if (!pending) return next(null, out)

          return $scope.each(function(i, el) {
            var $innerscope = $scope.eq(i)
            var node = xray(scope, v[0])
            node($innerscope, function(err, obj) {
              if (err) return next(err)
              out[i] = obj
              if (!--pending) {
                return next(null, compact(out))
              }
            })
          })
        }
      }
      return next()
    }, function(err, obj) {
      if (err) return fn(err)
      fn(null, obj, $)
    })
  }
}
```

## Pagination System

### Basic Pagination

```javascript
x('https://example.com/posts', '.post', ['title'])
  .paginate('.next a@href')  // Selector for "next" link
  .limit(10)                   // Max pages to crawl
  .then(function(results) {
    console.log(results)
  })
```

### Pagination Flow

```
Page 1 ──▶ Extract data
            │
            ▼
        Find next URL via selector
            │
            ▼
        Check limit
            │
            ▼
        Check abort condition
            │
            ▼
        Request Page 2 ──▶ ...
```

### Abort Conditions

```javascript
x('https://example.com', '.item', ['title'])
  .paginate('.next a@href')
  .abort(function(obj, url) {
    // Stop if we've seen this URL before
    if (visited.has(url)) return true
    visited.add(url)
    return false
  })
```

## Streaming Support

### Array Streaming

```javascript
var stream = x('https://example.com', '.item', ['title']).stream()
stream.pipe(fs.createWriteStream('results.json'))

// Stream writes array elements incrementally
// Output: [\n{...},\n{...},\n{...}\n]
```

### Object Streaming

```javascript
var stream = x('https://example.com', { title: 'title' }).stream()
stream.pipe(fs.createWriteStream('result.json'))

// Stream writes complete object
// Output: {...}
```

### Stream Helpers

```javascript
// lib/stream.js
module.exports = {
  array: function(stream) {
    if (!stream) return function() {}
    var first = true

    return function _stream_array(data, end) {
      var string = JSON.stringify(data, true, 2)
      var json = isArray(data) ? string.slice(1, -1) : string

      if (first && empty && !end) return
      if (first) { stream.write('[\n') }
      if (!first && !empty) { stream.write(',') }

      if (end) {
        stream.end(json + ']')
      } else {
        stream.write(json)
      }
      first = false
    }
  },

  object: function(stream) {
    if (!stream) return function() {}
    return function _stream_object(data, end) {
      var json = JSON.stringify(data, true, 2)
      if (end) {
        stream.end(json)
      } else {
        stream.write(json)
      }
    }
  }
}
```

## Collections

### Array of Objects

```javascript
x('https://dribbble.com', 'li.group', [
  {
    title: '.dribbble-img strong',
    image: '.dribbble-img [data-src]@data-src'
  }
])(function(err, results) {
  // results = [
  //   { title: "...", image: "..." },
  //   { title: "...", image: "..." }
  // ]
})
```

### Nested Collections

```javascript
x('http://mat.io', {
  title: 'title',
  items: x('.item', [
    {
      title: '.item-content h2',
      description: '.item-content section'
    }
  ])
})(function(err, results) {
  // results = {
  //   title: "...",
  //   items: [
  //     { title: "...", description: "..." },
  //     { title: "...", description: "..." }
  //   ]
  // }
})
```

## Filters

### Custom Filters

```javascript
var x = Xray({
  filters: {
    trim: function(value) {
      return value.trim()
    },
    uppercase: function(value) {
      return value.toUpperCase()
    },
    number: function(value) {
      return parseInt(value, 10)
    }
  }
})

// Use filters in selectors
x('https://example.com', 'h1|trim|uppercase')
x('https://example.com', '.count|number')
```

### Filter Application

```javascript
// lib/resolve.js
function filter(obj, $, scope, selector, value, filters) {
  var ctx = { $: $, selector: obj.selector, attribute: obj.attribute }
  
  return (obj.filters || []).reduce(function(out, filter) {
    var fn = filters[filter.name]
    if (typeof fn === 'function') {
      var args = [out].concat(filter.args || [])
      var filtered = fn.apply(ctx, args)
      return filtered
    } else {
      throw new Error('Invalid filter: ' + filter.name)
    }
  }, value)
}
```

## Responsible Scraping

### Rate Limiting

```javascript
var x = Xray()
  .delay('2s', '5s')      // Random delay between requests
  .concurrency(2)         // Max 2 concurrent requests
  .throttle(5, '1s')      // Max 5 requests per second
  .timeout(30000)         // 30 second timeout
```

### Best Practices

1. **Add delays** - Respect server resources
2. **Limit concurrency** - Don't overwhelm servers
3. **Set timeouts** - Handle unresponsive servers
4. **Check robots.txt** - Follow site rules
5. **Cache responses** - Avoid redundant requests
6. **Handle errors** - Graceful failure recovery

## Promise Integration

```javascript
// Thenable interface
x('https://example.com', 'title')
  .then(function(title) {
    console.log(title)
  })
  .catch(function(err) {
    console.error(err)
  })
```

## Error Handling

```javascript
// Error propagation through walk
walkHTML($, function(err, obj, $) {
  if (err) return fn(err)
  fn(null, obj, $)
})

// Invalid filter error
function filter(obj, $, scope, selector, value, filters) {
  return (obj.filters || []).reduce(function(out, filter) {
    var fn = filters[filter.name]
    if (typeof fn !== 'function') {
      throw new Error('Invalid filter: ' + filter.name)
    }
    // ...
  }, value)
}
```

## Complete Example

```javascript
var Xray = require('x-ray')
var x = Xray()
  .delay('1s', '3s')
  .concurrency(2)
  .limit(5)

x('https://news.ycombinator.com', 'tr.story', [
  {
    title: '.title a',
    link: '.title a@href',
    points: '.score',
    author: '.hnuser',
    comments: '.subtext a:last@text'
  }
])
  .paginate('.morelink@href')
  .write('hackernews.json')
  .then(function(results) {
    console.log('Scraped %d stories', results.length)
  })
  .catch(function(err) {
    console.error('Scraping failed:', err)
  })
```

## Performance Considerations

1. **Concurrency** - Balance speed vs server load
2. **Streaming** - Avoid memory issues with large datasets
3. **Selective parsing** - Only extract needed data
4. **Connection pooling** - Reuse HTTP connections
5. **Caching** - Avoid redundant requests

## Summary

X-ray provides:

1. **Declarative selectors** - CSS selectors with attribute extraction
2. **Pagination support** - Automatic multi-page crawling
3. **Streaming output** - Memory-efficient large dataset handling
4. **Custom filters** - Data transformation pipeline
5. **Rate limiting** - Responsible scraping controls
6. **Nested collections** - Complex data structure extraction
7. **Promise interface** - Modern async handling
8. **Cheerio integration** - Fast jQuery-like DOM parsing

The library's key insight is that web scraping should be declarative - describe what to extract, not how to extract it. X-ray handles HTTP requests, HTML parsing, URL resolution, and data transformation automatically based on selector definitions.
