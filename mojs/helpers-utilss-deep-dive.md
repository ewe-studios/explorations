---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src/h.coffee
explored_at: 2026-03-20
---

# mo.js Helpers and Utilities - Deep Dive

**Scope:** Math helpers, DOM helpers, Object/array utilities, String parsing, SVG namespace handling, Color utilities, Delta utilities

---

## Table of Contents

1. [Helpers Overview](#1-helpers-overview)
2. [Namespace and Constants](#2-namespace-and-constants)
3. [Browser Detection](#3-browser-detection)
4. [Object Utilities](#4-object-utilities)
5. [Math Helpers](#5-math-helpers)
6. [DOM Helpers](#6-dom-helpers)
7. [String Parsing Utilities](#7-string-parsing-utilities)
8. [Color Utilities](#8-color-utilities)
9. [Delta Utilities](#9-delta-utilities)
10. [Array Utilities](#10-array-utilities)
11. [SVG Utilities](#11-svg-utilities)
12. [Debug Utilities](#12-debug-utilities)

---

## 1. Helpers Overview

### 1.1 Singleton Pattern

```coffeescript
class Helpers
  # ... all helper methods ...

h = new Helpers
module.exports = h
```

**Design Decision:** Single instance exported as `h` - imported throughout mojs codebase

### 1.2 Usage Pattern

```javascript
import h from './h';

// Throughout mojs codebase
h.cloneObj(obj)
h.parseUnit(value)
h.rand(0, 100)
h.getRadialPoint({rotate: 45, radius: 50, center: {x: 0, y: 0}})
```

---

## 2. Namespace and Constants

### 2.1 SVG Namespace

```coffeescript
NS: 'http://www.w3.org/2000/svg'
```

**Usage:**
```javascript
const svg = document.createElementNS(h.NS, 'svg')
const circle = document.createElementNS(h.NS, 'circle')
```

### 2.2 Color Shortcuts

```coffeescript
shortColors:
  transparent: 'rgba(0,0,0,0)'
  none:        'rgba(0,0,0,0)'
  aqua:        'rgb(0,255,255)'
  black:       'rgb(0,0,0)'
  blue:        'rgb(0,0,255)'
  fuchsia:     'rgb(255,0,255)'
  gray:        'rgb(128,128,128)'
  green:       'rgb(0,128,0)'
  lime:        'rgb(0,255,0)'
  maroon:      'rgb(128,0,0)'
  navy:        'rgb(0,0,128)'
  olive:       'rgb(128,128,0)'
  purple:      'rgb(128,0,128)'
  red:         'rgb(255,0,0)'
  silver:      'rgb(192,192,192)'
  teal:        'rgb(0,128,128)'
  white:       'rgb(255,255,255)'
  yellow:      'rgb(255,255,0)'
  orange:      'rgb(255,128,0)'
```

**Purpose:** Fast lookup for CSS color names to RGB conversion

### 2.3 Property Maps

```coffeescript
# Non-tweenable properties
chainOptionMap: {}

# Callback names
callbacksMap:
  onRefresh:        1
  onStart:          1
  onComplete:       1
  onFirstUpdate:    1
  onUpdate:         1
  onProgress:       1
  onRepeatStart:    1
  onRepeatComplete: 1
  onPlaybackStart:    1
  onPlaybackPause:    1
  onPlaybackStop:     1
  onPlaybackComplete: 1

# Tween configuration options
tweenOptionMap:
  duration:         1
  delay:            1
  speed:            1
  repeat:           1
  easing:           1
  backwardEasing:   1
  isYoyo:           1
  shiftTime:        1
  isReversed:       1
  callbacksContext: 1

# Properties that need units
unitOptionMap:
  left:             1
  top:              1
  x:                1
  y:                1
  rx:               1
  ry:               1

# Conversion constant
RAD_TO_DEG: 180/Math.PI  # 57.29577951308232
```

---

## 3. Browser Detection

### 3.1 Constructor Detection

```coffeescript
constructor: ->
  @vars()

vars: ->
  @prefix = @getPrefix()
  @getRemBase()
  @isFF = @prefix.lowercase is 'moz'
  @isIE = @prefix.lowercase is 'ms'

  ua = navigator.userAgent
  @isOldOpera = ua.match /presto/gim
  @isSafari   = ua.indexOf('Safari') > -1
  @isChrome   = ua.indexOf('Chrome') > -1
  @isOpera    = ua.toLowerCase().indexOf("op") > -1

  # Chrome and Safari differentiation
  @isChrome and @isSafari and (@isSafari = false)
  (ua.match /PhantomJS/gim) and (@isSafari = false)
  @isChrome and @isOpera and (@isChrome = false)

  @is3d = @checkIf3d()
  @uniqIDs = -1

  # Create div for style testing
  @div = document.createElement('div')
  document.body.appendChild @div
  @defaultStyles = @computedStyle @div
```

### 3.2 Prefix Detection

```coffeescript
getPrefix: ->
  styles = window.getComputedStyle(document.documentElement, "")
  v = Array::slice.call(styles).join("").match(/-(moz|webkit|ms)-/)
  pre = (v or (styles.OLink is "" and ["", "o"]))[1]
  dom = ("WebKit|Moz|MS|O").match(new RegExp("(" + pre + ")", "i"))?[1]

  dom: dom           # "WebKit", "Moz", "MS", "O", or ""
  lowercase: pre     # "webkit", "moz", "ms", "o", or ""
  css: "-" + pre + "-"  # "-webkit-", "-moz-", etc.
  js: pre?[0].toUpperCase() + pre?.substr(1)  # "Webkit", "Moz", etc.
```

**Result Examples:**
```javascript
# Chrome
h.prefix.css: '-webkit-'
h.prefix.js: 'webkit'

# Firefox
h.prefix.css: '-moz-'
h.prefix.js: 'Moz'

# Modern browsers (no prefix)
h.prefix.css: '-'
h.prefix.js: ''
```

### 3.3 3D Transform Support

```coffeescript
checkIf3d: ->
  div = document.createElement 'div'
  @style div, 'transform', 'translateZ(0)'
  style = div.style
  prefixed = "#{@prefix.css}transform"
  tr = if style[prefixed]? then style[prefixed] else style.transform
  tr isnt ''
```

**Purpose:** Detect if browser supports 3D transforms for GPU acceleration

---

## 4. Object Utilities

### 4.1 Clone Object

```coffeescript
cloneObj: (obj, exclude) ->
  keys = Object.keys(obj)
  newObj = {}
  i = keys.length

  while i--
    key = keys[i]
    if exclude?
      newObj[key] = obj[key] if !exclude[key]
    else
      newObj[key] = obj[key]

  newObj
```

**Usage:**
```javascript
// Clone all properties
h.cloneObj({a: 1, b: 2})  // {a: 1, b: 2}

// Clone with exclusions
h.cloneObj({a: 1, b: 2, c: 3}, {b: 1})  // {a: 1, c: 3}
```

### 4.2 Extend Object

```coffeescript
extend: (objTo, objFrom) ->
  for key, value of objFrom
    objTo[key] ?= objFrom[key]
  objTo
```

**Purpose:** Copy properties from `objFrom` to `objTo` only if not already defined

```javascript
h.extend({a: 1}, {a: 99, b: 2})  // {a: 1, b: 2}
```

### 4.3 Get Last Item

```coffeescript
getLastItem: (arr) ->
  arr[arr.length - 1]
```

### 4.4 Is Object Check

```coffeescript
isObject: (variable) ->
  variable != null and typeof variable is 'object'
```

---

## 5. Math Helpers

### 5.1 Clamp

```coffeescript
clamp: (value, min, max) ->
  if value < min then min
  else if value > max then max
  else value
```

**Usage:**
```javascript
h.clamp(150, 0, 100)    // 100
h.clamp(-10, 0, 100)    // 0
h.clamp(50, 0, 100)     // 50
```

### 5.2 Random

```coffeescript
rand: (min, max) ->
  (Math.random() * ((max) - min)) + min
```

**Usage:**
```javascript
h.rand(0, 10)           // Random float between 0-10
h.rand(0, 1)            // Random float between 0-1
```

### 5.3 Radial Point Calculation

```coffeescript
getRadialPoint: (o = {}) ->
  radAngle = (o.rotate - 90) * 0.017453292519943295  # Math.PI/180
  radiusX = if o.radiusX? then o.radiusX else o.radius
  radiusY = if o.radiusY? then o.radiusY else o.radius

  point =
    x: o.center.x + (Math.cos(radAngle) * radiusX)
    y: o.center.y + (Math.sin(radAngle) * radiusY)
```

**Mathematics:**
```
angle = (rotate - 90) * π/180  # -90 to start from top (12 o'clock)
x = center.x + cos(angle) * radiusX
y = center.y + sin(angle) * radiusY
```

**Usage:**
```javascript
# Point at 90 degrees (right), radius 50, center (100, 100)
h.getRadialPoint({
  rotate: 90,
  radius: 50,
  center: {x: 100, y: 100}
})
// x: 100 + cos(0) * 50 = 150
// y: 100 + sin(0) * 50 = 100
```

---

## 6. DOM Helpers

### 6.1 Parse Element

```coffeescript
parseEl: (el) ->
  if h.isDOM(el) then return el
  else if typeof el is 'string'
    el = document.querySelector el

  if el == null
    h.error "Can't parse HTML element: ", el
  el
```

**Usage:**
```javascript
// String selector
h.parseEl('#myElement')  // Returns DOM element

// Already DOM element
h.parseEl(domElement)    // Returns same element

// Invalid - throws error
h.parseEl(null)          // Error: Can't parse HTML element
```

### 6.2 Is DOM Check

```coffeescript
isDOM: (o) ->
  return false if !o?
  isNode = typeof o.nodeType is 'number' and typeof o.nodeName is 'string'
  typeof o is 'object' and isNode
```

### 6.3 Set Prefixed Style

```coffeescript
setPrefixedStyle: (el, name, value) ->
  (name is 'transform') and (el.style["#{@prefix.css}#{name}"] = value)
  el.style[name] = value
```

**Purpose:** Apply vendor-prefixed styles for cross-browser transform support

```javascript
h.setPrefixedStyle(el, 'transform', 'translateX(100px)')
// Sets both: el.style['-webkit-transform'] and el.style['transform']
```

### 6.4 Style Method

```coffeescript
style: (el, name, value) ->
  if typeof name is 'object'
    keys = Object.keys(name)
    len = keys.length
    while len--
      key = keys[len]
      value = name[key]
      @setPrefixedStyle el, key, value
  else
    @setPrefixedStyle el, name, value
```

**Usage:**
```javascript
// Single property
h.style(el, 'width', '100px')

// Multiple properties
h.style(el, {
  width: '100px',
  height: '50px',
  transform: 'translateX(50px)'
})
```

### 6.5 Force 3D Layer

```coffeescript
force3d: (el) ->
  @setPrefixedStyle el, 'backface-visibility', 'hidden'
  el
```

**Purpose:** Force GPU acceleration by creating composite layer

### 6.6 Get Child Elements

```coffeescript
getChildElements: (element) ->
  childNodes = element.childNodes
  children = []
  i = childNodes.length

  while i--
    if childNodes[i].nodeType == 1  # Element node
      children.unshift childNodes[i]

  children
```

### 6.7 Computed Style

```coffeescript
computedStyle: (el) ->
  getComputedStyle el
```

---

## 7. String Parsing Utilities

### 7.1 Parse Unit

```coffeescript
parseUnit: (value) ->
  if typeof value is 'number'
    return {
      unit:     'px'
      isStrict: false
      value:    value
      string:   if value is 0 then "#{value}" else "#{value}px"
    }

  else if typeof value is 'string'
    regex = /px|%|rem|em|ex|cm|ch|mm|in|pt|pc|vh|vw|vmin|deg/gim
    unit = value.match(regex)?[0]
    isStrict = true

    # Plain number string - default to px
    if !unit
      unit = 'px'
      isStrict = false

    amount = parseFloat value

    return {
      unit:     unit
      isStrict: isStrict
      value:    amount
      string:   if amount is 0 then "#{amount}" else "#{amount}#{unit}"
    }

  value
```

**Unit Structure:**
```javascript
{
  unit: 'px|%'|rem'|'em'|...,  // CSS unit
  isStrict: boolean,            // Was unit explicitly specified?
  value: number,                // Numeric value
  string: string                // Full string representation
}
```

**Examples:**
```javascript
h.parseUnit(100)             // {unit: 'px', isStrict: false, value: 100, string: '100px'}
h.parseUnit('100px')         // {unit: 'px', isStrict: true, value: 100, string: '100px'}
h.parseUnit('50%')           // {unit: '%', isStrict: true, value: 50, string: '50%'}
h.parseUnit('2rem')          // {unit: 'rem', isStrict: true, value: 2, string: '2rem'}
h.parseUnit(0)               // {unit: 'px', isStrict: false, value: 0, string: '0'}
```

### 7.2 Merge Units

```coffeescript
mergeUnits: (start, end, key) ->
  if !end.isStrict and start.isStrict
    # End inherits start's unit
    end.unit = start.unit
    end.string = "#{end.value}#{end.unit}"

  else if end.isStrict and !start.isStrict
    # Start inherits end's unit
    start.unit = end.unit
    start.string = "#{start.value}#{start.unit}"

  else if end.isStrict and start.isStrict
    if end.unit isnt start.unit
      # Different units - convert start to end's unit with warning
      start.unit = end.unit
      start.string = "#{start.value}#{start.unit}"
      @warn "Two different units were specified on \"#{key}\" delta
             property, mo·js will fallback to end \"#{end.unit}\" unit"
```

**Examples:**
```javascript
// Implicit end unit inherits start
start = {value: 0, unit: 'px', isStrict: false}
end = {value: 100, isStrict: false}
h.mergeUnits(start, end, 'x')
// end becomes: {value: 100, unit: 'px', isStrict: false, string: '100px'}

// Explicit unit mismatch
start = {value: 0, unit: 'px', isStrict: true}
end = {value: 100, unit: '%', isStrict: true}
h.mergeUnits(start, end, 'width')
// start converted to %, warning logged
```

### 7.3 Parse Rand

```coffeescript
parseRand: (string) ->
  randArr = string.split /rand\(|,|\)/
  units = @parseUnit randArr[2]
  rand = @rand(parseFloat(randArr[1]), parseFloat(randArr[2]))

  if units.unit and randArr[2].match(units.unit)
    then rand + units.unit
    else rand
```

**Usage:**
```javascript
h.parseRand('rand(10, 50)')      // Random float between 10-50
h.parseRand('rand(10px, 50px)')  // Random float with 'px' suffix
h.parseRand('rand(0.1, 1)')      // Random float between 0.1-1
```

### 7.4 Parse Stagger

```coffeescript
parseStagger: (string, index) ->
  value = string.split(/stagger\(|\)$/)[1].toLowerCase()

  # Split on commas to get base and step
  splittedValue = value.split(/(rand\(.*?\)|[^\(,\s]+)(?=\s*,|\s*$)/gim)

  # Check for base value
  value = if splittedValue.length > 3
    base = @parseUnit(@parseIfRand(splittedValue[1]))
    splittedValue[3]
  else
    base = @parseUnit(0)
    splittedValue[1]

  value = @parseIfRand(value)
  unitValue = @parseUnit(value)

  number = index * unitValue.value + base.value

  # Add units if original had units
  unit = if base.isStrict then base.unit
  else if unitValue.isStrict then unitValue.unit
  else ''

  if unit then "#{number}#{unit}" else number
```

**Usage:**
```javascript
h.parseStagger('stagger(50)', 0)     // 0
h.parseStagger('stagger(50)', 1)     // 50
h.parseStagger('stagger(50)', 2)     // 100

h.parseStagger('stagger(20, 10)', 0) // 10 (base 10 + 0*20)
h.parseStagger('stagger(20, 10)', 1) // 30 (base 10 + 1*20)
h.parseStagger('stagger(20, 10)', 2) // 50 (base 10 + 2*20)

h.parseStagger('stagger(50px)', 3)   // '150px'
```

### 7.5 Parse If Rand

```coffeescript
parseIfRand: (str) ->
  if typeof str is 'string' and str.match(/rand\(/)
    then @parseRand(str)
    else str
```

### 7.6 Parse If Stagger

```coffeescript
parseIfStagger: (value, i) ->
  if !(typeof value is 'string' and value.match /stagger/g)
    then value
    else @parseStagger(value, i)
```

### 7.7 Parse String Option

```coffeescript
parseStringOption: (value, index = 0) ->
  if typeof value is 'string'
    value = @parseIfStagger(value, index)
    value = @parseIfRand(value)
  value
```

**Pipeline:** String → Parse Stagger → Parse Rand → Result

---

## 8. Color Utilities

### 8.1 Make Color Object

```coffeescript
makeColorObj: (color) ->
  # HEX parsing
  if color[0] is '#'
    result = /^#?([a-f\d]{1,2})([a-f\d]{1,2})([a-f\d]{1,2})$/i.exec(color)
    colorObj = {}

    if result
      r = if result[1].length is 2
        then result[1] else result[1] + result[1]
      g = if result[2].length is 2
        then result[2] else result[2] + result[2]
      b = if result[3].length is 2
        then result[3] else result[3] + result[3]

      colorObj =
        r: parseInt(r, 16)
        g: parseInt(g, 16)
        b: parseInt(b, 16)
        a: 1

  # Named colors and RGB
  if color[0] isnt '#'
    isRgb = color[0] is 'r' and color[1] is 'g' and color[2] is 'b'

    if isRgb
      rgbColor = color
    else
      # Named color lookup
      if !@shortColors[color]
        # Use div to resolve CSS color name
        @div.style.color = color
        rgbColor = @computedStyle(@div).Color
      else
        rgbColor = @shortColors[color]

    # Parse RGB/RGBA
    regexString1 = '^rgba?\\((\\d{1,3}),\\s?(\\d{1,3}),'
    regexString2 = '\\s?(\\d{1,3}),?\\s?(\\d{1}|0?\\.\\d{1,})?\\)$'
    result = new RegExp(regexString1 + regexString2, 'gi').exec(rgbColor)

    alpha = parseFloat(result[4] or 1)

    if result
      colorObj =
        r: parseInt(result[1], 10)
        g: parseInt(result[2], 10)
        b: parseInt(result[3], 10)
        a: if alpha? and !isNaN(alpha) then alpha else 1

  colorObj
```

**Color Object Structure:**
```javascript
{
  r: 0-255,      // Red channel
  g: 0-255,      // Green channel
  b: 0-255,      // Blue channel
  a: 0.0-1.0     // Alpha channel
}
```

**Examples:**
```javascript
h.makeColorObj('#ff0000')      // {r: 255, g: 0, b: 0, a: 1}
h.makeColorObj('#f00')         // {r: 255, g: 0, b: 0, a: 1}
h.makeColorObj('red')          // {r: 255, g: 0, b: 0, a: 1}
h.makeColorObj('rgba(255,0,0,0.5)')  // {r: 255, g: 0, b: 0, a: 0.5}
```

---

## 9. Delta Utilities

### 9.1 Parse Delta

```coffeescript
parseDelta: (key, value, index) ->
  # Clone delta object
  value = @cloneObj value

  # Parse easing
  easing = value.easing
  if easing? then easing = mojs.easing.parseEasing(easing)
  delete value.easing

  # Parse curve
  curve = value.curve
  if curve? then curve = mojs.easing.parseEasing(curve)
  delete value.curve

  start = Object.keys(value)[0]
  end = value[start]

  delta = start: start

  # Color values
  if isNaN(parseFloat(start)) and !start.match(/rand\(/) and !start.match(/stagger\(/)
    if key is 'strokeLinecap'
      @warn "Sorry, stroke-linecap property is not animatable yet,
             using the start(#{start}) value instead", value
      return delta

    startColorObj = @makeColorObj start
    endColorObj = @makeColorObj end

    delta =
      type:     'color'
      name:     key
      start:    startColorObj
      end:      endColorObj
      easing:   easing
      curve:    curve
      delta:
        r: endColorObj.r - startColorObj.r
        g: endColorObj.g - startColorObj.g
        b: endColorObj.b - startColorObj.b
        a: endColorObj.a - startColorObj.a

  # Array values (stroke-dasharray, stroke-dashoffset, origin)
  else if key is 'strokeDasharray' or key is 'strokeDashoffset' or key is 'origin'
    startArr = @strToArr start
    endArr = @strToArr end
    @normDashArrays startArr, endArr

    for start, i in startArr
      end = endArr[i]
      @mergeUnits start, end, key

    delta =
      type:     'array'
      name:     key
      start:    startArr
      end:      endArr
      delta:    @calcArrDelta startArr, endArr
      easing:   easing
      curve:    curve

  # Numeric values
  else
    if !@callbacksMap[key] and !@tweenOptionMap[key]
      # Unit values (position properties)
      if @unitOptionMap[key]
        end = @parseUnit @parseStringOption end, index
        start = @parseUnit @parseStringOption start, index
        @mergeUnits start, end, key

        delta =
          type:     'unit'
          name:     key
          start:    start
          end:      end
          delta:    end.value - start.value
          easing:   easing
          curve:    curve

      # Plain numeric values
      else
        end = parseFloat @parseStringOption end, index
        start = parseFloat @parseStringOption start, index

        delta =
          type:     'number'
          name:     key
          start:    start
          end:      end
          delta:    end - start
          easing:   easing
          curve:    curve

  delta
```

### 9.2 Is Delta Check

```coffeescript
isDelta: (optionsValue) ->
  isObject = @isObject optionsValue
  isObject = isObject and !optionsValue.unit
  return !(!isObject or @isArray(optionsValue) or @isDOM(optionsValue))
```

### 9.3 Get Delta End

```coffeescript
getDeltaEnd: (obj) ->
  key = Object.keys(obj)[0]
  return obj[key]
```

### 9.4 Get Delta Start

```coffeescript
getDeltaStart: (obj) ->
  key = Object.keys(obj)[0]
  return key
```

### 9.5 Is Tween Property

```coffeescript
isTweenProp: (keyName) ->
  @tweenOptionMap[keyName] or @callbacksMap[keyName]
```

---

## 10. Array Utilities

### 10.1 String to Array

```coffeescript
strToArr: (string) ->
  arr = []

  # Plain number
  if typeof string is 'number' and !isNaN(string)
    arr.push @parseUnit string
    return arr

  # String array (space-separated)
  string.trim().split(/\s+/gim).forEach (str) =>
    arr.push @parseUnit @parseIfRand str

  arr
```

**Examples:**
```javascript
h.strToArr(10)              // [{value: 10, unit: 'px', ...}]
h.strToArr('5 10')          // [{value: 5, ...}, {value: 10, ...}]
h.strToArr('5px 10%')       // [{value: 5, unit: 'px'}, {value: 10, unit: '%'}]
```

### 10.2 Calculate Array Delta

```coffeescript
calcArrDelta: (arr1, arr2) ->
  delta = []

  for num, i in arr1
    delta[i] = @parseUnit "#{arr2[i].value - arr1[i].value}#{arr2[i].unit}"

  delta
```

### 10.3 Normalize Dash Arrays

```coffeescript
normDashArrays: (arr1, arr2) ->
  arr1Len = arr1.length
  arr2Len = arr2.length

  if arr1Len > arr2Len
    lenDiff = arr1Len - arr2Len
    startI = arr2.length
    for i in [0...lenDiff]
      currItem = i + startI
      arr2.push @parseUnit "0#{arr1[currItem].unit}"

  else if arr2Len > arr1Len
    lenDiff = arr2Len - arr1Len
    startI = arr1.length
    for i in [0...lenDiff]
      currItem = i + startI
      arr1.push @parseUnit "0#{arr2[currItem].unit}"

  [arr1, arr2]
```

**Purpose:** Equalize array lengths for stroke-dasharray interpolation

```javascript
arr1 = [{value: 5}, {value: 10}, {value: 15}]
arr2 = [{value: 1}, {value: 2}]

h.normDashArrays(arr1, arr2)
// arr1: [{value: 5}, {value: 10}, {value: 15}]
// arr2: [{value: 1}, {value: 2}, {value: 0}]  # Padded with 0
```

### 10.4 Is Array Check

```coffeescript
isArray: (variable) ->
  variable instanceof Array
```

---

## 11. SVG Utilities

### 11.1 Parse Path

```coffeescript
parsePath: (path) ->
  if typeof path is 'string'
    return if path.charAt(0).toLowerCase() is 'm'
      # SVG path string - create path element
      domPath = document.createElementNS @NS, 'path'
      domPath.setAttributeNS null, 'd', path
      domPath
    else
      # CSS selector
      document.querySelector path

  # Already an SVG element
  return path if path.style
```

**Usage:**
```javascript
// Path string
h.parsePath('M0,0 L100,100')  // Creates SVGPathElement

// CSS selector
h.parsePath('#myPath')        // Queries DOM

// SVG element
h.parsePath(svgPathElement)   // Returns as-is
```

---

## 12. Debug Utilities

### 12.2 Log Badge Styling

```coffeescript
logBadgeCss: 'background:#3A0839;color:#FF512F;border-radius:5px;
  padding: 1px 5px 2px; border: 1px solid #FF512F;'
```

### 12.3 Prepare for Log

```coffeescript
prepareForLog: (args) ->
  args = Array::slice.apply args
  args.unshift('::')
  args.unshift(@logBadgeCss)
  args.unshift('%cmo·js%c')
  args
```

### 12.4 Log Method

```coffeescript
log: ->
  return if mojs.isDebug is false
  console.log.apply console, @prepareForLog arguments
```

### 12.5 Warn Method

```coffeescript
warn: ->
  return if mojs.isDebug is false
  console.warn.apply console, @prepareForLog arguments
```

### 12.6 Error Method

```coffeescript
error: ->
  return if mojs.isDebug is false
  console.error.apply console, @prepareForLog arguments
```

**Usage:**
```javascript
h.log('Animation started')
// Console: [mo·js] Animation started (with styled badge)

h.warn('Deprecated API usage')
// Console: [mo·js] Deprecated API usage (styled as warning)

h.error('Invalid configuration')
// Console: [mo·js] Invalid configuration (styled as error)
```

---

## 13. Utility Methods

### 13.1 Get Unique ID

```coffeescript
getUniqID: ->
  ++@uniqIDs
```

### 13.2 Close Enough (Float Comparison)

```coffeescript
closeEnough: (num1, num2, eps) ->
  Math.abs(num1 - num2) < eps
```

**Purpose:** Compare floating point numbers with epsilon tolerance

### 13.3 Bind

```coffeescript
bind: (func, context) ->
  wrapper = ->
    args = Array::slice.call(arguments)
    unshiftArgs = bindArgs.concat(args)
    func.apply context, unshiftArgs

  bindArgs = Array::slice.call(arguments, 2)
  wrapper
```

### 13.4 Capitalize

```coffeescript
capitalize: (str) ->
  if typeof str isnt 'string'
    throw Error 'String expected - nothing to capitalize'
  str.charAt(0).toUpperCase() + str.substring(1)
```

### 13.5 Delta Helper

```coffeescript
delta: (start, end) ->
  type1 = typeof start
  type2 = typeof end

  isType1 = type1 is 'string' or (type1 is 'number' and !isNaN(start))
  isType2 = type2 is 'string' or (type2 is 'number' and !isNaN(end))

  if !isType1 or !isType2
    @error "delta method expects Strings or Numbers at input
            but got - #{start}, #{end}"
    return

  obj = {}
  obj[start] = end
  obj
```

**Usage:**
```javascript
h.delta(0, 100)        // {0: 100}
h.delta('0px', '100%') // {'0px': '100%'}
```

---

## Summary

The `h.coffee` helpers file (609 lines) is the backbone of mo.js, providing:

1. **Browser Detection:** Prefix detection, 3D support checking, browser flags
2. **Object Utilities:** Cloning, extending, property manipulation
3. **Math Helpers:** Clamp, random, radial point calculation
4. **DOM Utilities:** Element parsing, style setting, computed styles
5. **String Parsing:** Unit extraction, rand/stagger parsing, string-to-array conversion
6. **Color Utilities:** HEX/RGB/RGBA parsing, named color resolution
7. **Delta System:** Delta detection, parsing, and manipulation
8. **Array Utilities:** Dash array normalization, delta calculation
9. **SVG Handling:** Path parsing, namespace handling
10. **Debug Tools:** Styled console logging with mo·js badge

Every helper is optimized for performance and reusability across the mo.js codebase, with careful attention to edge cases like unit mismatches, floating-point comparison, and cross-browser compatibility.
