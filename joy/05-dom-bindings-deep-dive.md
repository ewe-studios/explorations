# Joy DOM Bindings and Virtual DOM Deep Dive

## Overview

Joy compiler generates DOM bindings from Web IDL (Interface Definition Language) definitions. This allows Go code to interact with browser APIs using type-safe interfaces that mirror the native DOM API.

## Web IDL Processing Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    Web IDL Processing                             │
├─────────────────────────────────────────────────────────────────┤
│  1. Parse Web IDL                                                 │
│     browser.webidl.xml → raw.Interface definitions               │
├─────────────────────────────────────────────────────────────────┤
│  2. Build Index                                                   │
│     Map interface names to definitions                           │
│     Resolve extends/implements relationships                     │
├─────────────────────────────────────────────────────────────────┤
│  3. Generate Go Code                                              │
│     Interface → Go struct with methods/properties                │
│     Event definitions → Go event types                           │
│     Dictionary → Go struct                                       │
├─────────────────────────────────────────────────────────────────┤
│  4. Compile Time                                                  │
│     Go type checking on DOM APIs                                 │
│     Method signature validation                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Web IDL Interface Structure

```go
// internal/_dom/raw/raw.go
type Interface struct {
    Name                string
    Extends             string
    Implements          []string
    Methods             []*Method
    Properties          []*Property
    Events              []*Event
    Constructor         *Constructor
    NamedConstructor    *Constructor
    NoInterfaceObject   bool
}

type Method struct {
    Name       string
    Params     []*Param
    Returns    *Return
    IsStatic   bool
    IsGetter   bool
    IsSetter   bool
}

type Property struct {
    Name       string
    Type       string
    IsStatic   bool
    IsReadOnly bool
}

type Param struct {
    Name     string
    Type     string
    Optional bool
    Variadic bool
}
```

## Interface Generation

### Interface Definition

```go
// internal/_dom/defs/interface.go
type Interface interface {
    ImplementedBy() ([]def.Definition, error)
    Properties() []Property
    Methods() []Method
    FindEvent(name string) (def.Definition, error)
    Ancestors() (ancestors []def.Definition, err error)
    Implements() (defs []def.Definition, err error)
    def.Definition
}
```

### Interface Implementation

```go
type iface struct {
    data *raw.Interface
    pkg  string
    file string

    index         index.Index
    implementedBy []def.Definition
    methods       []Method
    properties    []Property
}

// ID returns the interface ID
func (d *iface) ID() string {
    return d.data.Name
}

// Name returns the interface name
func (d *iface) Name() string {
    return d.data.Name
}

// Kind returns the definition kind
func (d *iface) Kind() string {
    return "INTERFACE"
}

// Type returns the Go type name
func (d *iface) Type(caller string) (string, error) {
    imps, err := d.ImplementedBy()
    if err != nil {
        return "", errors.Wrapf(err, "implemented by")
    }

    if caller == d.pkg || d.pkg == "" {
        if len(imps) > 0 || d.data.NoInterfaceObject {
            return gen.Capitalize(d.data.Name), nil
        }
        return gen.Pointer(gen.Capitalize(d.data.Name)), nil
    }

    if len(imps) > 0 || d.data.NoInterfaceObject {
        return d.pkg + "." + gen.Capitalize(d.data.Name), nil
    }

    return gen.Pointer(d.pkg + "." + gen.Capitalize(d.data.Name)), nil
}
```

### Ancestor Resolution

```go
// Parents returns direct parent interfaces (extends)
func (d *iface) Parents() (parents []def.Definition, err error) {
    if d.data.Extends != "" && d.data.Extends != "Object" {
        parent, isset := d.index[d.data.Extends]
        if !isset {
            return parents, fmt.Errorf("extends doesn't exist %s on %s", 
                d.data.Extends, d.data.Name)
        }
        parents = append(parents, parent)
    }
    return parents, nil
}

// Ancestors returns all ancestor interfaces
func (d *iface) Ancestors() (ancestors []def.Definition, err error) {
    if d.data.Extends != "" && d.data.Extends != "Object" {
        def := d.index.Find(d.data.Extends)
        if def == nil {
            return ancestors, fmt.Errorf("extends '%s' not found", d.data.Extends)
        }
        ancestors = append(ancestors, def)

        switch t := def.(type) {
        case Interface:
            a, err := t.Ancestors()
            if err != nil {
                return ancestors, errors.Wrapf(err, "error getting ancestors")
            }
            ancestors = append(ancestors, a...)
        }
    }
    return ancestors, nil
}
```

### Implements Resolution

```go
// Implements returns interfaces this interface implements
func (d *iface) Implements() (defs []def.Definition, err error) {
    for _, imp := range d.data.Implements {
        def := d.index.Find(imp)
        if def == nil {
            return defs, fmt.Errorf("implements '%s' not found", imp)
        }
        defs = append(defs, def)
    }
    return defs, nil
}
```

## Method Generation

### Method Definition

```go
// internal/_dom/defs/method.go
type Method interface {
    Name() string
    Type(caller string) (string, error)
    SetPackage(pkg string)
    SetFile(file string)
    Generate() (string, error)
    GenerateAs(def Interface) (string, error)
    GenerateInterfaceAs(def Interface) (string, error)
    Dependencies() ([]def.Definition, error)
    def.Definition
}
```

### Method Generation

```go
func (m *method) GenerateAs(def Interface) (string, error) {
    data := struct {
        Package     string
        Name        string
        Params      []gen.Vartype
        Return      string
        Receiver    string
        IsStatic    bool
        Rewrite     string
    }{
        Package:  m.pkg,
        Name:     m.data.Name,
        IsStatic: m.data.IsStatic,
    }

    // Generate parameters
    for _, param := range m.data.Params {
        t, err := m.index.Coerce(m.pkg, param.Type)
        if err != nil {
            return "", errors.Wrapf(err, "param type")
        }
        data.Params = append(data.Params, gen.Vartype{
            Var:      gen.Lowercase(param.Name),
            Optional: param.Optional,
            Type:     t,
        })
    }

    // Generate return type
    if m.data.Returns != nil {
        t, err := m.index.Coerce(m.pkg, m.data.Returns.Type)
        if err != nil {
            return "", errors.Wrapf(err, "return type")
        }
        data.Return = t
    }

    // Generate receiver
    receiver := gen.Lowercase(def.Name())
    data.Receiver = receiver

    // Generate rewrite rule
    data.Rewrite = m.generateRewrite(def)

    return gen.Generate("method/"+def.Name()+"/"+m.data.Name, data, `
        {{- if .IsStatic -}}
        // {{ .Name }} fn
        func {{ .Receiver }}{{ .Name }}({{ joinvt .Params }}) {{ .Return }} {
            macro.Rewrite("{{ .Rewrite }}", {{ joinv .Params }})
        }
        {{- else -}}
        // {{ .Name }} fn
        func ({{ .Receiver }} *{{ capitalize $.Name }}) {{ .Name }}({{ joinvt .Params }}) {{ .Return }} {
            macro.Rewrite("{{ .Rewrite }}", {{ joinv .Params }})
        }
        {{- end -}}
    `)
}
```

## Property Generation

### Property Definition

```go
// internal/_dom/defs/property.go
type Property interface {
    Name() string
    Type(caller string) (string, error)
    Generate() (string, error)
    GenerateAs(def Interface) (string, error)
    Dependencies() ([]def.Definition, error)
    def.Definition
}
```

### Property Generation

```go
func (p *property) GenerateAs(def Interface) (string, error) {
    data := struct {
        Package    string
        Name       string
        Type       string
        Receiver   string
        IsStatic   bool
        IsReadOnly bool
        Getter     string
        Setter     string
    }{
        Package:    p.pkg,
        Name:       p.data.Name,
        IsStatic:   p.data.IsStatic,
        IsReadOnly: p.data.IsReadOnly,
    }

    // Get property type
    t, err := p.index.Coerce(p.pkg, p.data.Type)
    if err != nil {
        return "", errors.Wrapf(err, "property type")
    }
    data.Type = t

    // Generate receiver
    data.Receiver = gen.Lowercase(def.Name())

    // Generate getter
    data.Getter = p.data.Name

    // Generate setter (if not readonly)
    if !p.data.IsReadOnly {
        data.Setter = "set" + gen.Capitalize(p.data.Name)
    }

    return gen.Generate("property/"+def.Name()+"/"+p.data.Name, data, `
        {{- if .IsStatic -}}
        // {{ .Name }} getter
        func {{ .Name }}() {{ .Type }} {
            var result {{ .Type }}
            macro.Rewrite("{{ .Getter }}", &result)
            return result
        }

        {{- if .Setter -}}
        // {{ .Name }} setter
        func {{ .Setter }}(v {{ .Type }}) {
            macro.Rewrite("{{ .Getter }} = $0", v)
        }
        {{- end -}}

        {{- else -}}
        // {{ .Name }} getter
        func ({{ .Receiver }} *{{ capitalize $.Name }}) {{ .Name }}() {{ .Type }} {
            var result {{ .Type }}
            macro.Rewrite("this.{{ .Getter }}", &result)
            return result
        }

        {{- if .Setter -}}
        // {{ .Name }} setter
        func ({{ .Receiver }} *{{ capitalize $.Name }}) {{ .Setter }}(v {{ .Type }}) {
            macro.Rewrite("this.{{ .Getter }} = $0", v)
        }
        {{- end -}}
        {{- end -}}
    `)
}
```

## Example: HTMLElement Interface

### Web IDL Definition

```webidl
interface HTMLElement : Element {
    attribute DOMString id;
    attribute DOMString className;
    attribute DOMString innerHTML;
    attribute DOMString outerHTML;
    void click();
    void focus();
    void blur();
};
```

### Generated Go Code

```go
// Generated from HTMLElement Web IDL
package window

// HTMLElement interface
type HTMLElement interface {
    Element
    
    // Properties
    ID() string
    SetID(v string)
    ClassName() string
    SetClassName(v string)
    InnerHTML() string
    SetInnerHTML(v string)
    OuterHTML() string
    SetOuterHTML(v string)
    
    // Methods
    Click()
    Focus()
    Blur()
}

// HTMLElement struct
type HTMLElement struct {
    Element
}

// ID getter
func (h *HTMLElement) ID() string {
    var result string
    macro.Rewrite("this.id", &result)
    return result
}

// ID setter
func (h *HTMLElement) SetID(v string) {
    macro.Rewrite("this.id = $0", v)
}

// ClassName getter
func (h *HTMLElement) ClassName() string {
    var result string
    macro.Rewrite("this.className", &result)
    return result
}

// ClassName setter
func (h *HTMLElement) SetClassName(v string) {
    macro.Rewrite("this.className = $0", v)
}

// InnerHTML getter
func (h *HTMLElement) InnerHTML() string {
    var result string
    macro.Rewrite("this.innerHTML", &result)
    return result
}

// InnerHTML setter
func (h *HTMLElement) SetInnerHTML(v string) {
    macro.Rewrite("this.innerHTML = $0", v)
}

// Click method
func (h *HTMLElement) Click() {
    macro.Rewrite("this.click()")
}

// Focus method
func (h *HTMLElement) Focus() {
    macro.Rewrite("this.focus()")
}

// Blur method
func (h *HTMLElement) Blur() {
    macro.Rewrite("this.blur()")
}
```

## Event System

### Event Definition

```go
// internal/_dom/defs/event.go
type Event struct {
    Name string
    Type string  // Event type (e.g., "MouseEvent", "KeyboardEvent")
    IDL  string  // Original IDL definition
}
```

### Event Finding

```go
// FindEvent finds an event definition by name
func (d *iface) FindEvent(name string) (def.Definition, error) {
    // Search local events first
    for _, event := range d.data.Events {
        if event.Name == name {
            if e, isset := d.index[event.Type]; isset {
                return e, nil
            }
        }
    }

    // Traverse up the inheritance chain
    parents, err := d.Parents()
    if err != nil {
        return nil, err
    }

    for _, parent := range parents {
        if t, ok := parent.(*iface); ok {
            return t.FindEvent(name)
        }
    }

    // Return default Event type
    return d.index["Event"], nil
}
```

## Dictionary Generation

Dictionaries are data structures used for complex parameters:

```go
// internal/_dom/defs/dictionary.go
type Dictionary interface {
    def.Definition
    Properties() []Property
    Generate() (string, error)
}

// NewDictionary creates a dictionary definition
func NewDictionary(index index.Index, data *raw.Dictionary) Dictionary {
    return &dictionary{
        data:  data,
        index: index,
    }
}

type dictionary struct {
    data *raw.Dictionary
    pkg  string
    file string
    index index.Index
}

// Generate generates Go code for the dictionary
func (d *dictionary) Generate() (string, error) {
    data := struct {
        Package    string
        Name       string
        Extends    string
        Properties []Property
    }{
        Package: d.pkg,
        Name:    d.data.Name,
    }

    // Handle inheritance
    if d.data.Extends != "" {
        data.Extends = d.data.Extends
    }

    // Generate properties
    for _, prop := range d.data.Properties {
        // ... property generation
    }

    return gen.Generate("dictionary/"+d.data.Name, data, `
        // {{ .Name }} struct
        type {{ capitalize .Name }} struct {
            {{- if .Extends }}
            {{ .Extends }}
            {{- end }}
            {{ range .Properties }}
            {{ .Name }} {{ .Type }}
            {{- end }}
        }
    `)
}
```

## Type Coercion

The indexer handles type coercion between Go and JavaScript types:

```go
// internal/dom/index/index.go
func (idx Index) Coerce(pkg, typeName string) (string, error) {
    // Handle primitive types
    switch typeName {
    case "DOMString":
        return "string", nil
    case "unsigned long":
        return "uint32", nil
    case "long":
        return "int32", nil
    case "double":
        return "float64", nil
    case "boolean":
        return "bool", nil
    case "any":
        return "interface{}", nil
    }

    // Handle array types
    if strings.HasPrefix(typeName, "sequence<") {
        inner := strings.TrimSuffix(strings.TrimPrefix(typeName, "sequence<"), ">")
        coerced, err := idx.Coerce(pkg, inner)
        if err != nil {
            return "", err
        }
        return "[]" + coerced, nil
    }

    // Handle union types
    if strings.Contains(typeName, " or ") {
        parts := strings.Split(typeName, " or ")
        var types []string
        for _, part := range parts {
            t, err := idx.Coerce(pkg, strings.TrimSpace(part))
            if err != nil {
                return "", err
            }
            types = append(types, t)
        }
        return "interface{} // " + strings.Join(types, " | "), nil
    }

    // Handle interface types
    def := idx.Find(typeName)
    if def != nil {
        return def.Type(pkg)
    }

    return typeName, nil
}
```

## Using DOM Bindings in Go Code

### Basic Usage

```go
package main

import (
    "github.com/matthewmueller/joy/dom/window"
)

func main() {
    // Get element by ID
    el := window.Document.GetElementByID("app")
    
    // Type assert to HTMLElement
    if htmlEl, ok := el.(*window.HTMLElement); ok {
        htmlEl.SetInnerHTML("Hello, World!")
        htmlEl.SetClassName("active")
    }
    
    // Create new element
    newEl := window.Document.CreateElement("div")
    newEl.SetID("container")
    
    // Add event listener
    newEl.AddEventListener("click", func(event *window.Event) {
        event.PreventDefault()
    }, false)
}
```

### Event Handling

```go
// Event listener pattern
func setupListeners() {
    btn := window.Document.GetElementByID("button")
    
    // Click event
    btn.AddEventListener("click", func(e *window.MouseEvent) {
        console.Log("Button clicked at:", e.ClientX, e.ClientY)
    }, false)
    
    // Keyboard event
    window.Document.AddEventListener("keydown", func(e *window.KeyboardEvent) {
        if e.Key == "Enter" {
            console.Log("Enter key pressed")
        }
    }, false)
}
```

## Generated Code Structure

```
joy/dom/
├── window/
│   ├── window.go           # Window interface
│   ├── document.go         # Document interface
│   ├── element.go          # Element interface
│   ├── htmlelement.go      # HTMLElement interface
│   ├── event.go            # Event interface
│   ├── mouseevent.go       # MouseEvent interface
│   ├── keyboardevent.go    # KeyboardEvent interface
│   └── ...
├── navigator/
│   ├── navigator.go        # Navigator interface
│   └── ...
└── ...
```

## Summary

Joy's DOM binding system:

1. **Parses Web IDL** - Converts browser API definitions to Go interfaces
2. **Resolves inheritance** - Handles extends/implements relationships
3. **Generates type-safe code** - Go interfaces with proper method signatures
4. **Maps properties** - Getters/setters for DOM properties
5. **Handles events** - Type-safe event listeners and handlers
6. **Supports dictionaries** - Complex parameter objects
7. **Type coercion** - Automatic JavaScript to Go type mapping

This allows writing Go code that feels natural while maintaining full type safety and IDE support for browser APIs.
