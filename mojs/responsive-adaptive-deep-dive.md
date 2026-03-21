---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src
explored_at: 2026-03-20
---

# mo.js Responsive and Adaptive Animation - Deep Dive

**Scope:** DPI/retina handling, Reduced motion preferences, Viewport awareness, Dynamic resizing, Responsive animation patterns

---

## Table of Contents

1. [Responsive Architecture Overview](#1-responsive-architecture-overview)
2. [DPI and Retina Handling](#2-dpi-and-retina-handling)
3. [Reduced Motion Preferences](#3-reduced-motion-preferences)
4. [Viewport Awareness](#4-viewport-awareness)
5. [Dynamic Resizing](#5-dynamic-resizing)
6. [Responsive Animation Patterns](#6-responsive-animation-patterns)
7. [Adaptive Performance Scaling](#7-adaptive-performance-scaling)

---

## 1. Responsive Architecture Overview

### 1.1 Responsive Considerations

mo.js was designed primarily as a motion graphics library rather than a responsive animation framework. However, several patterns and techniques enable responsive animations:

```
┌─────────────────────────────────────────────────────────────────┐
│                  mo.js RESPONSIVE LAYERS                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              UNIT SYSTEM                                 │    │
│  │  - Relative units (%, em, rem, vh, vw)                  │    │
│  │  - Unit interpolation and merging                       │    │
│  │  - Automatic unit inference                             │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│         ┌────────────────────┼────────────────────┐             │
│         ▼                    ▼                    ▼             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │  VIEWPORT   │     │   REDUCED   │     │   ADAPTIVE  │       │
│  │  AWARENESS  │     │   MOTION    │     │  PERFORMANCE│       │
│  │  (manual)   │     │  (manual)   │     │  (manual)   │       │
│  └─────────────┘     └─────────────┘     └─────────────┘       │
│                                                                  │
│  Note: Most responsive features require manual implementation  │
│        using mo.js primitives and browser APIs                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Built-in vs Manual Features

| Feature | Built-in | Manual Implementation |
|---------|----------|----------------------|
| Unit interpolation | ✓ | - |
| Percentage-based values | ✓ | - |
| DPI-aware rendering | Partial | Full with canvas |
| Reduced motion detection | - | ✓ (via JS) |
| Viewport awareness | - | ✓ (via listeners) |
| Dynamic resizing | Partial | ✓ (via resize) |
| Adaptive performance | - | ✓ (via speed) |

---

## 2. DPI and Retina Handling

### 2.1 Numeric Precision for High-DPI

mo.js uses full IEEE 754 double-precision for all calculations:

```javascript
// All delta calculations maintain full precision
delta = {
  start: 0,
  end: 100,
  delta: 100
};

// Interpolation preserves precision
current = start + easedProgress * delta;  // 50.123456789...

// Only rounded at final render if needed
element.setAttribute('cx', Math.round(current));
```

**Benefits for Retina:**
- No precision loss during animation
- Smooth sub-pixel animations
- Compatible with high-DPI displays

### 2.2 SVG Automatic Scaling

SVG elements automatically scale with display density:

```javascript
// SVG shape - automatically retina-ready
new mojs.Shape({
  shape: 'circle',
  radius: 50,  // Vector units, not pixels
  x: 100,
  y: 100
});
```

**Why SVG is Retina-Ready:**
- Vector-based rendering
- Browser handles DPI scaling
- No pixelation at any zoom level

### 2.3 HTML Element Considerations

For HTML elements, consider device pixel ratio:

```javascript
// Get device pixel ratio
const dpr = window.devicePixelRatio || 1;

// Scale animation values for high-DPI
const baseRadius = 50;
const scaledRadius = baseRadius * dpr;

new mojs.Html({
  el: '#element',
  borderRadius: {0: scaledRadius}
});
```

### 2.4 Canvas Integration (Manual)

For canvas-based animations with mo.js timing:

```javascript
const canvas = document.querySelector('canvas');
const ctx = canvas.getContext('2d');
const dpr = window.devicePixelRatio || 1;

// Scale canvas for retina
canvas.width = canvas.offsetWidth * dpr;
canvas.height = canvas.offsetHeight * dpr;
ctx.scale(dpr, dpr);

// Use mojs.Tween for timing
const progressTween = new mojs.Tween({
  duration: 1000,
  onUpdate: (progress) => {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.beginPath();
    ctx.arc(
      100 * progress,  // Animated position
      100,
      50,
      0, Math.PI * 2
    );
    ctx.fill();
  }
});

progressTween.play();
```

---

## 3. Reduced Motion Preferences

### 3.1 Detecting Reduced Motion

mo.js does not have built-in reduced motion detection, but it can be implemented:

```javascript
// Check for reduced motion preference
function prefersReducedMotion() {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

// Listen for changes
function onReducedMotionChange(callback) {
  const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
  mq.addEventListener('change', (e) => callback(e.matches));
  return () => mq.removeEventListener('change', callback);
}
```

### 3.2 Adapting Animations

```javascript
// Create animation with reduced motion option
function createAnimation(options) {
  const isReducedMotion = prefersReducedMotion();

  const adaptedOptions = {
    ...options,
    duration: isReducedMotion ? 1 : options.duration,
    easing: isReducedMotion ? 'linear.none' : options.easing,
  };

  // Remove complex effects for reduced motion
  if (isReducedMotion) {
    delete adaptedOptions.children;  // Remove burst children
    delete adaptedOptions.count;     // Remove particle count
  }

  return new mojs.Burst(adaptedOptions);
}

// Usage
const burst = createAnimation({
  count: 20,
  radius: {0: 200},
  duration: 1000,
  easing: 'elastic.out',
  children: {
    shape: 'circle',
    radius: {10: 0}
  }
});

// Listen for preference changes
onReducedMotionChange((isReduced) => {
  burst.tune({
    duration: isReduced ? 1 : 1000,
    easing: isReduced ? 'linear.none' : 'elastic.out'
  });
});
```

### 3.3 Motion Sensitivity Levels

```javascript
// Granular motion sensitivity
function getMotionPreference() {
  const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
  const mqNoPreference = window.matchMedia('(prefers-reduced-motion: no-preference)');

  if (mq.matches) return 'reduced';
  if (mqNoPreference.matches) return 'full';
  return 'reduced';  // Default to reduced if unknown
}

function createAdaptiveAnimation(options) {
  const preference = getMotionPreference();

  const adaptations = {
    'reduced': {
      duration: 1,
      easing: 'linear.none',
      count: 1,  // Single particle instead of burst
    },
    'full': options
  };

  return new mojs.Burst(adaptations[preference]);
}
```

### 3.4 Disabling Specific Effects

```javascript
// Disable specific motion-intensive effects
const config = {
  // Always safe
  opacity: {1: 0},

  // Conditionally disable
  ...(prefersReducedMotion() ? {} : {
    scale: {1: 0},
    rotate: {0: 360}
  })
};

new mojs.Shape(config);
```

---

## 4. Viewport Awareness

### 4.1 Intersection Observer Integration

```javascript
// Trigger animation when element enters viewport
function animateOnView(selector, animationOptions) {
  const element = document.querySelector(selector);

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const animation = new mojs.Shape(animationOptions);
        animation.play();
        observer.unobserve(entry.target);  // One-time animation
      }
    });
  }, {
    threshold: 0.5,  // Trigger when 50% visible
    rootMargin: '0px'
  });

  observer.observe(element);
}

// Usage
animateOnView('.trigger-element', {
  shape: 'circle',
  radius: {0: 100},
  fill: 'blue'
});
```

### 4.2 Viewport-Based Progress

```javascript
// Animation progress tied to scroll position
function scrollProgressAnimation(options) {
  const {element, ...animationOptions} = options;

  const shape = new mojs.Shape(animationOptions);

  function onScroll() {
    const rect = element.getBoundingClientRect();
    const viewportHeight = window.innerHeight;

    // Calculate progress (0-1) based on scroll position
    let progress = (viewportHeight - rect.top) / (viewportHeight + rect.height);
    progress = Math.max(0, Math.min(1, progress));  // Clamp

    // Set animation progress
    shape.timeline._setProgress(progress);
  }

  window.addEventListener('scroll', onScroll);
  onScroll();  // Initial call

  return () => window.removeEventListener('scroll', onScroll);
}
```

### 4.3 Offscreen Pause

```javascript
// Pause animations when offscreen
class ViewportAwareAnimation {
  constructor(selector, AnimationClass, options) {
    this.element = document.querySelector(selector);
    this.AnimationClass = AnimationClass;
    this.options = options;
    this.animation = null;
    this.isVisible = false;

    this.setupObserver();
  }

  setupObserver() {
    this.observer = new IntersectionObserver((entries) => {
      const entry = entries[0];
      this.isVisible = entry.isIntersecting;

      if (this.isVisible && !this.animation) {
        this.animation = new this.AnimationClass(this.options);
        this.animation.play();
      } else if (!this.isVisible && this.animation) {
        this.animation.pause();
      }
    });

    this.observer.observe(this.element);
  }

  destroy() {
    this.observer.disconnect();
    if (this.animation) {
      this.animation.stop();
      this.animation = null;
    }
  }
}
```

---

## 5. Dynamic Resizing

### 5.1 Resize Event Handling

```javascript
// Responsive animation that adapts to container size
class ResponsiveAnimation {
  constructor(container, options) {
    this.container = container;
    this.baseOptions = options;
    this.animation = null;

    this.setupResizeListener();
    this.create();
  }

  setupResizeListener() {
    this.resizeObserver = new ResizeObserver(() => {
      this.recreate();
    });

    this.resizeObserver.observe(this.container);
  }

  create() {
    const rect = this.container.getBoundingClientRect();

    // Scale animation based on container size
    const scale = rect.width / 500;  // Base size reference

    this.animation = new mojs.Shape({
      ...this.baseOptions,
      radius: this.baseOptions.radius * scale,
      x: rect.width / 2,
      y: rect.height / 2,
    });

    this.animation.play();
  }

  recreate() {
    if (this.animation) {
      this.animation.stop();
      this.animation = null;
    }
    this.create();
  }

  destroy() {
    this.resizeObserver.disconnect();
    if (this.animation) {
      this.animation.stop();
    }
  }
}
```

### 5.2 Debounced Resize

```javascript
// Debounce resize handling
function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

// Usage
const handleResize = debounce(() => {
  // Recreate animations with new dimensions
  recreateAnimations();
}, 250);

window.addEventListener('resize', handleResize);
```

### 5.3 Responsive Burst

```javascript
// Burst that adapts to container size
function createResponsiveBurst(container, baseOptions) {
  const rect = container.getBoundingClientRect();
  const minDimension = Math.min(rect.width, rect.height);

  // Scale burst radius to container
  const radiusScale = minDimension / 300;  // Reference size

  return new mojs.Burst({
    ...baseOptions,
    radius: {0: baseOptions.radius * radiusScale},
    children: {
      ...baseOptions.children,
      radius: baseOptions.children.radius * radiusScale,
    }
  });
}
```

---

## 6. Responsive Animation Patterns

### 6.1 Relative Unit Animation

```javascript
// Use percentage-based values for responsive animations
new mojs.Shape({
  // Percentages relative to container
  x: { '0%': '100%' },
  y: { '0%': '50%' },

  // Ems for typography-based sizing
  radius: { '1em': '2em' },

  // Viewport units for full-screen effects
  // Note: Requires manual calculation or CSS custom properties
});
```

### 6.2 Unit Interpolation

mo.js automatically handles unit interpolation:

```javascript
// Different units - mo.js converts to end unit
new mojs.Shape({
  x: { '0px': '100%' }  // Start in px, end in %, converts to %
});

// Warning logged if units differ significantly
// "Two different units were specified on 'x' delta property..."
```

### 6.3 Responsive Easing

```javascript
// Adjust easing based on screen size
function getResponsiveEasing() {
  if (window.innerWidth < 768) {
    return 'quad.out';  // Gentler easing on mobile
  }
  return 'elastic.out'; // More dramatic on desktop
}

new mojs.Shape({
  y: {0: 100},
  easing: getResponsiveEasing()
});
```

### 6.4 Stagger Responsiveness

```javascript
// Adjust stagger timing based on item count and screen size
function getResponsiveStagger(baseDelay, itemCount) {
  const isSmallScreen = window.innerWidth < 768;

  // Reduce delay on small screens for faster completion
  const delay = isSmallScreen ? baseDelay * 0.5 : baseDelay;

  // Total stagger time
  const totalTime = delay * itemCount;

  // Cap total stagger time
  if (totalTime > 2000) {
    return `stagger(${2000 / itemCount}, 0)`;
  }

  return `stagger(${delay})`;
}

// Usage
const items = document.querySelectorAll('.item');
items.forEach((item, i) => {
  new mojs.Shape({
    el: item,
    delay: getResponsiveStagger(50, items.length),
    opacity: {0: 1}
  });
});
```

### 6.5 Conditional Effects

```javascript
// Disable effects on low-end devices or small screens
function getAdaptiveOptions(baseOptions) {
  const isLowEnd = navigator.hardwareConcurrency <= 2;
  const isSmallScreen = window.innerWidth < 768;

  if (isLowEnd || isSmallScreen) {
    return {
      ...baseOptions,
      count: 1,  // Single particle instead of burst
      isShowStart: true,  // Skip fade-in
    };
  }

  return baseOptions;
}
```

---

## 7. Adaptive Performance Scaling

### 7.1 Frame Rate Detection

```javascript
// Detect frame rate and adjust animation complexity
class AdaptiveAnimation {
  constructor(options) {
    this.options = options;
    this.fps = 60;
    this.quality = 'high';

    this.detectPerformance();
  }

  detectPerformance() {
    let lastTime = performance.now();
    let frameCount = 0;

    const measureFps = () => {
      const now = performance.now();
      frameCount++;

      if (now - lastTime >= 1000) {
        this.fps = frameCount;

        // Adjust quality based on FPS
        if (this.fps < 30) {
          this.quality = 'low';
        } else if (this.fps < 50) {
          this.quality = 'medium';
        } else {
          this.quality = 'high';
        }

        frameCount = 0;
        lastTime = now;
      }

      requestAnimationFrame(measureFps);
    };

    requestAnimationFrame(measureFps);
  }

  create() {
    const settings = {
      low: { count: 5, duration: 500 },
      medium: { count: 10, duration: 800 },
      high: this.options
    };

    return new mojs.Burst(settings[this.quality]);
  }
}
```

### 7.2 Speed Control

```javascript
// Adjust animation speed based on performance
function adjustAnimationSpeed(animation, targetFPS = 60) {
  const actualFPS = getActualFPS();
  const speed = Math.min(actualFPS / targetFPS, 1);

  animation.tune({ speed });

  // Speed < 1 slows down animation to maintain smoothness
  // Speed = 1 is normal speed
}
```

### 7.3 Battery Status Integration

```javascript
// Reduce animation complexity on battery power
async function getPowerAwareOptions(options) {
  let adaptedOptions = {...options};

  if ('getBattery' in navigator) {
    const battery = await navigator.getBattery();

    const updateForBattery = () => {
      if (!battery.charging && battery.level < 0.2) {
        // Low battery, reduce effects
        adaptedOptions = {
          ...options,
          duration: options.duration * 0.5,
          count: Math.floor(options.count * 0.5),
        };
      }
    };

    updateForBattery();

    battery.addEventListener('levelchange', updateForBattery);
    battery.addEventListener('chargingchange', updateForBattery);
  }

  return adaptedOptions;
}
```

### 7.4 Data Saver Mode

```javascript
// Respect Data Saver mode
function getDataSaverOptions(options) {
  const connection = navigator.connection || navigator.mozConnection || navigator.webkitConnection;

  if (connection?.saveData) {
    // Reduce animation data/complexity
    return {
      ...options,
      count: 1,  // Minimal particles
      duration: 1,  // Instant animation
    };
  }

  return options;
}
```

---

## Summary

mo.js provides foundational support for responsive and adaptive animations:

### Built-in Features
1. **Unit Interpolation:** Automatic handling of px, %, em, rem, vh, vw
2. **SVG Vector Scaling:** Resolution-independent rendering
3. **Numeric Precision:** Full IEEE 754 precision for smooth sub-pixel animation
4. **Speed Control:** Per-animation speed adjustment

### Manual Implementation Patterns
1. **Reduced Motion:** Detect via `matchMedia` and adapt animations
2. **Viewport Awareness:** Use IntersectionObserver for trigger-based animations
3. **Dynamic Resizing:** Use ResizeObserver or window resize events
4. **Adaptive Performance:** Scale complexity based on FPS, device capabilities
5. **Battery/Data Awareness:** Respect power and data constraints

### Best Practices
1. Use relative units (%, em) for responsive sizing
2. Detect and respect `prefers-reduced-motion`
3. Use IntersectionObserver for scroll-triggered animations
4. Implement quality tiers (low/medium/high) based on device capabilities
5. Debounce resize handlers to prevent excessive recreation
6. Consider battery status and Data Saver mode for mobile users

While mo.js requires manual implementation for full responsiveness, its flexible API and unit system provide all the primitives needed for adaptive, accessible animations.
