---
location: /home/darkvoid/Boxxed/@formulas/src.AppOSS/src.penpot/penpot/frontend/
repository: git@github.com:penpot/penpot
explored_at: 2026-03-17
language: ClojureScript
parent: exploration.md
---

# Penpot Frontend Architecture - Deep Dive

## Overview

The Penpot frontend is a ClojureScript single-page application (SPA) built with shadow-cljs, using a functional reactive programming (FRP) stack with Potok and Beikon.

## Build System

### shadow-cljs Configuration

```edn
{:target :browser
 :output-dir "resources/public/js/"
 :asset-path "/js"
 :modules
 {:shared  ;; Shared utilities
  {:entries []}

  :main  ;; Core app
  {:entries [app.main app.plugins.api]
   :depends-on #{:shared}
   :init-fn app.main/init}

  :main-auth  ;; Auth pages
  {:entries [app.main.ui.auth
             app.main.ui.auth.verify-token]
   :depends-on #{:main}}

  :main-viewer  ;; Viewer mode
  {:entries [app.main.ui.viewer]
   :depends-on #{:main :main-auth}}

  :main-workspace  ;; Design workspace
  {:entries [app.main.ui.workspace]
   :depends-on #{:main}}

  :main-dashboard  ;; Dashboard
  {:entries [app.main.ui.dashboard]
   :depends-on #{:main}}

  :main-settings  ;; Settings pages
  {:entries [app.main.ui.settings]
   :depends-on #{:main}}

  :render  ;; Wasm render bridge
  {:entries [app.render]
   :depends-on #{:shared}
   :init-fn app.render/init}

  :rasterizer  ;; Offscreen rasterization
  {:entries [app.rasterizer]
   :depends-on #{:shared}
   :init-fn app.rasterizer/init}}

 :js-options
 {:entry-keys ["module" "browser" "main"]
  :export-conditions ["module" "import" "browser" "require" "default"]
  :js-provider :external
  :external-index "target/index.js"
  :external-index-format :esm}}
```

### Module Code Splitting

```
resources/public/js/
├── manifest.json       ;; Module manifest
├── main.js            ;; Main entry + lazy load logic
├── shared.js          ;; Shared code (common utils)
├── main-auth.js       ;; Auth module
├── main-viewer.js     ;; Viewer module
├── main-workspace.js  ;; Workspace module (largest)
├── main-dashboard.js  ;; Dashboard module
├── main-settings.js   ;; Settings module
├── render.js          ;; Wasm render bridge
└── worker.js          ;; Web worker
```

## Application Initialization

### Entry Point

```clojure
(ns app.main
  (:require
   [app.config :as cf]
   [app.main.store :as st]
   [app.main.ui :as ui]
   [app.main.worker :as mw]
   [app.util.i18n :as i18n]
   [rumext.v2 :as mf]))

(defn ^:export init
  []
  (mw/init!)           ; Start web workers
  (i18n/init! cf/translations)  ; Initialize i18n
  (init-ui)            ; Create root component
  (st/emit! (initialize)))  ; Start data flow
```

### Root Component

```clojure
(defonce app-root
  (let [el (dom/get-element "app")]
    (mf/create-root el)))

(defn init-ui
  []
  (mf/render! app-root (mf/element ui/app)))
```

## FRP Stack

### Potok (Effects + State Machine)

```clojure
(defmethod ptk/fn ::initialize
  [state]
  (-> state
      (assoc :session-id (uuid/next))
      (assoc :loading? true)))

(defmethod ptk/fn ::load-profile
  [{:keys [profile-id]}]
  {:http/request
   {:method :get
    :path   "/api/v1/profile"
    :on-success ::profile-loaded
    :on-error   ::profile-error}})
```

**Key Concepts:**
- Events trigger effects
- Effects produce events
- State is immutable
- Flow is composable

### Beikon (Reactive Streams)

```clojure
(require '[beikon.v2.core :as rx])

;; WebSocket message stream
(def ws-stream
  (->> (rx/merge
        (rx/of (ev/initialize))
        (->> stream
             (rx/filter dp/profile-fetched?)
             (rx/take 1)
             (rx/map #(rt/init-routes)))))
```

## Data Layer

### Data Module Structure

```
frontend/src/app/main/data/
├── auth.cljs          ; Authentication state
├── changes.cljs       ; File change synchronization
├── comments.cljs      ; Comments data
├── common.cljs        ; Shared data utilities
├── dashboard.cljs     ; Dashboard data
├── event.cljs         ; Event dispatch
├── fonts.cljs         ; Font loading/management
├── helpers.cljs       ; Data helpers
├── media.cljs         ; Media uploads
├── modal.cljs         ; Modal state
├── notifications.cljs ; Notification system
├── persistence.cljs   ; Local storage
├── plugins.cljs       ; Plugin data
├── preview.cljs       ; Preview generation
├── profile.cljs       ; User profile
├── project.cljs       ; Project operations
├── render_wasm.cljs   ; Wasm render state
├── shortcuts.cljs     ; Keyboard shortcuts
├── style_dictionary.cljs ; Design tokens
├── team.cljs          ; Team data
├── viewer.cljs        ; Viewer mode data
└── websocket.cljs     ; WebSocket connection
```

### WebSocket Sync

```clojure
(ns app.main.data.websocket
  (:require
   [app.main.data.event :as ev]
   [beikon.v2.core :as rx]))

(defn initialize []
  (websocket/connect!
   {:url (cf/ws-url)
    :on-open (fn [_]
               (ev/dispatch! ::ws-connected))
    :on-message (fn [msg]
                  (ev/dispatch! ::ws-message msg))
    :on-close (fn [_]
                (ev/dispatch! ::ws-disconnected))}))
```

### File Change Synchronization

```clojure
(ns app.main.data.changes
  (:require
   [app.common.types.shape :as shape]
   [app.render-wasm :as wasm]))

(defn apply-changes!
  [state changes]
  (reduce (fn [state change]
            (case (:type change)
              :create (create-shape state change)
              :update (update-shape state change)
              :delete (delete-shape state change)
              :reorder (reorder-shapes state change)))
          state
          changes))
```

## UI Architecture

### Component Structure

```
frontend/src/app/main/ui/
├── alert.cljs              ; Alert dialogs
├── auth/                   ; Auth pages
│   ├── login.cljs
│   ├── register.cljs
│   └── recovery.cljs
├── comments.cljs           ; Comments panel
├── components/             ; Reusable components
│   ├── button.cljs
│   ├── dropdown.cljs
│   ├── input.cljs
│   ├── modal.cljs
│   └── select.cljs
├── dashboard/              ; Dashboard pages
│   ├── files.cljs
│   ├── projects.cljs
│   └── shared.cljs
├── ds/                     ; Design system
│   ├── buttons.cljs
│   ├── forms.cljs
│   └── layout.cljs
├── workspace/              ; Main design workspace
│   ├── canvas.cljs         ; Canvas container
│   ├── toolbar.cljs        ; Tools
│   ├── layers.cljs         ; Layers panel
│   ├── properties.cljs     ; Properties panel
│   └── header.cljs         ; Top bar
├── viewer.cljs             ; Viewer/presentation mode
└── settings.cljs           ; Settings pages
```

### Rumext Components

```clojure
(ns app.main.ui.workspace.canvas
  (:require
   [rumext.v2 :as mf]))

(mf/defc canvas []
  {:init (fn [state]
           (assoc state ::wasm-initialized? false))
   :did-mount (fn [state]
                (wasm/init! true)
                (assoc state ::wasm-initialized? true))
   :render (fn []
             [:div.canvas
              [:canvas#render-canvas]
              [:div.canvas-overlay]])})
```

### Design System (Storybook)

```clojure
(ns app.main.ui.ds
  (:require
   [rumext.v2 :as mf]))

(mf/defc button [{:keys [variant size children]}]
  [:button.btn
   {:class (str "btn-" variant " btn-" size)}
   children])

(def default
  {:button button
   :input  input
   :modal  modal})
```

## Wasm Integration

### Render Bridge

```clojure
(ns app.render-wasm
  (:require
   [app.common.types.path]
   [app.common.types.shape :as shape]
   [app.render-wasm.api :as api]
   [app.render-wasm.shape :as wasm.shape]))

(def module api/module)

(defn initialize [enabled?]
  (if enabled?
    (set! app.common.types.path/wasm:calc-bool-content api/calculate-bool)
    (set! app.common.types.path/wasm:calc-bool-content nil))
  (set! app.common.types.shape/wasm-enabled? enabled?)
  (set! app.common.types.shape/wasm-create-shape wasm.shape/create-shape))
```

### Shape Creation

```clojure
(ns app.render-wasm.shape
  (:require
   [app.render-wasm.mem :as mem]
   [app.render-wasm.uuid :as uuid]))

(defn create-shape [shape]
  (let [id (:id shape)]
    ;; Initialize Wasm shape
    (api/use-shape id)

    ;; Set transform
    (api/set-shape-transform (:a t) (:b t) (:c t) (:d t) (:e t) (:f t))

    ;; Set bounds
    (api/set-shape-selrect left top right bottom)

    ;; Set parent
    (when parent
      (api/set-parent parent))

    ;; Set children
    (when-not (empty? children)
      (mem/with-buffer! [buf]
        (uuid/write-uuids! buf children)
        (api/set-children)))

    shape))
```

## Web Workers

### Worker Initialization

```clojure
(ns app.main.worker)

(defn init!
  []
  (when (exists? js/Worker)
    (let [worker (js/Worker "/js/worker/main.js")]
      (swap! workers assoc :main worker)
      (.postMessage worker #js {:type :init}))))
```

### Worker Tasks

```clojure
(ns app.worker
  (:require
   [app.worker.tasks :as tasks]))

(defn handle-message [event]
  (let [data (.-data event)
        type (:type data)]
    (case type
      :thumbnail (tasks/generate-thumbnail data)
      :export (tasks/export-file data)
      :preview (tasks/generate-preview data))))
```

## State Management

### App Store

```clojure
(ns app.main.store
  (:require
   [beikon.v2.core :as rx]
   [potok.v2.core :as ptk]))

(defonce state (rx/behavior {}))

(defn emit!
  [& effects]
  (let [stream (ptk/exec-effects effects)]
    (rx/subscribe! stream state)))
```

### Navigation

```clojure
(ns app.main.ui.routes
  (:require
   [bidi.bidi :as bidi]
   [pushy.core :as pushy]))

(def routes
  ["/"
   [["auth" ["/login" ::auth/login
             ["/register" ::auth/register]]]
    ["dashboard" ["/" ::dashboard/files
                  ["/projects/" :project-id]
                  ["/shared"]]
     ["workspace" ["/" :file-id]]
     ["viewer" ["/" :file-id]]]])

(defn init-routes []
  (pushy/start!
   (pushy/pushy navigate!)
   (pushy/parse-value bidi/match-route routes)))
```

## Internationalization

```clojure
(ns app.util.i18n
  (:require
   [cuerdas.core :as str]))

(defonce locale (rx/behavior "en"))

(defn t
  [key & [params]]
  (let [translations (get-in @locale [:translations key])]
    (if params
      (str/format translations params)
      translations)))

;; Usage in components
[:span (t "common.loading")]
```

## Performance Optimizations

### Lazy Component Loading

```clojure
;; Modules loaded on demand
(defn load-dashboard! []
  (js/Promise.all
   #js [(js/import "./main-dashboard.js")]))
```

### Memoization

```clojure
(mf/defc shape-render [{:keys [shape zoom]}]
  {:should-update (fn [old new]
                    (and (= (:id old) (:id new))
                         (= (:zoom old) (:zoom new))))
   :render (fn []
             [:div.shape
              (render-shape shape zoom)])})
```

### Virtual Scrolling

```clojure
(ns app.main.ui.components.virtual-list)

(defn virtual-list
  [items render-item item-height]
  [:div.virtual-list
   {:style {:height (* (count items) item-height)}}
   (for [item (visible-items items scroll-pos)]
     ^{:key (:id item)}
     (render-item item))])
```

## Testing

### Unit Tests

```clojure
(ns frontend-test.app.main.data.changes-test
  (:require
   [cljs.test :refer-macros [deftest is testing]]
   [app.main.data.changes :as changes]))

(deftest apply-create-shape-test
  (let [state {}
        change {:type :create
                :id #uuid "..."
                :type :rect}]
    (is (contains? (changes/apply-changes! state [change])
                   (:id change)))))
```

### Playwright E2E

```javascript
// frontend/playwright/workspace.spec.js
import { test, expect } from '@playwright/test';

test('create rectangle', async ({ page }) => {
  await page.goto('/workspace/file-uuid');
  await page.click('[data-tool="rect"]');
  await page.mouse.move(100, 100);
  await page.mouse.down();
  await page.mouse.move(300, 200);
  await page.mouse.up();

  const shape = await page.$('.shape[data-type="rect"]');
  await expect(shape).toBeVisible();
});
```

## Configuration

### Config File

```clojure
(ns app.config)

(def target :browser)

(def flags
  #{:enable-feature-render-wasm
    :enable-render-wasm-dpr
    :enable-design-tokens})

(def version
  {:full "2.0.0"
   :major 2
   :minor 0
   :patch 0})

(def public-uri
  (or (some-> js/process.env .-PUBLIC_URI)
      "http://localhost:3448"))
```

### Feature Flags

```clojure
;; Enable in config.js
window.APP_CONFIG = {
  flags: [
    'enable-feature-render-wasm',
    'enable-render-wasm-dpr'
  ]
};
```
