# Joy Compiler Internals Deep Dive

## Overview

This deep-dive examines the internals of the Joy compiler - a Go-to-JavaScript transpiler. We explore the compilation pipeline, AST translation, and code generation strategies.

## Compilation Pipeline

### High-Level Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     Go Source Files                           │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  Stage 1: LOAD - Parse Go packages with go/types             │
│  - Type checking                                             │
│  - Import resolution                                         │
│  - Package graph construction                                │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  Stage 2: INDEX - Catalog all definitions                    │
│  - Functions, methods, structs, interfaces                   │
│  - Track exports and omissions                               │
│  - Build definition lookup table                             │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  Stage 3: GRAPH - Build dependency graph                     │
│  - Topological sort from main()                              │
│  - Detect unreachable code                                   │
│  - Prune unused definitions                                  │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  Stage 4: TRANSLATE - Go AST to JavaScript AST               │
│  - Type-aware translation                                    │
│  - Standard library mapping                                  │
│  - DOM binding generation                                    │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  Stage 5: ASSEMBLE - Generate JavaScript code                │
│  - Module pattern wrapping                                   │
│  - Import linking                                            │
│  - Code minification (optional)                              │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                     JavaScript Output                         │
└──────────────────────────────────────────────────────────────┘
```

## Stage 1: Loading Go Packages

### Package Loader

```go
// internal/compiler/loader/loader.go
type Config struct {
    JoyPath  string
    Packages []string
}

func Load(cfg *Config) (*Program, error) {
    // Create type checker configuration
    conf := types.Config{
        Importer: defaultImporter(),
        Error:    func(err error) { /* collect errors */ },
    }
    
    // Load all packages
    program := &Program{
        Packages: make(map[string]*Package),
    }
    
    for _, pkgPath := range cfg.Packages {
        pkg, err := conf.Check(pkgPath, nil, []string{pkgPath}, nil)
        if err != nil {
            return nil, err
        }
        program.Packages[pkgPath] = pkg
    }
    
    return program, nil
}
```

### Type Information Extraction

```go
// Extract type information from Go package
type Package struct {
    Path       string
    Name       string
    Types      map[string]*types.TypeName
    Functions  map[string]*types.Func
    Variables  map[string]*types.Var
    Constants  map[string]*types.Const
    Interfaces map[string]*types.Interface
    Structs    map[string]*types.Struct
}

// Categorize definitions
func categorizeDefs(scope *types.Scope) *Package {
    pkg := &Package{
        Types:      make(map[string]*types.TypeName),
        Functions:  make(map[string]*types.Func),
        Variables:  make(map[string]*types.Var),
    }
    
    for _, name := range scope.Names() {
        obj := scope.Lookup(name)
        switch obj := obj.(type) {
        case *types.TypeName:
            pkg.Types[name] = obj
        case *types.Func:
            pkg.Functions[name] = obj
        case *types.Var:
            pkg.Variables[name] = obj
        }
    }
    
    return pkg
}
```

## Stage 2: Indexing Definitions

### Definition Interface

```go
// internal/compiler/def/def.go
type Definition interface {
    ID() string              // Unique identifier
    Path() string            // Package path
    Kind() string            // "function", "struct", "method", etc.
    Name() string            // Local name
    Exported() bool          // Is it exported (capitalized)?
    Omitted() bool           // Should it be omitted from output?
    Dependencies() ([]Definition, error)
    Imports() map[string]string
}
```

### Definition Types

```go
// Function definition
type Function struct {
    path     string
    name     string
    exported bool
    funcType *types.Func
    body     *ast.FuncDecl
}

func (f *Function) ID() string       { return f.path + "." + f.name }
func (f *Function) Path() string     { return f.path }
func (f *Function) Kind() string     { return "function" }
func (f *Function) Name() string     { return f.name }
func (f *Function) Exported() bool   { return f.exported }
func (f *Function) Omitted() bool    { return false }

// Method definition (function with receiver)
type Method struct {
    Function
    recv *types.Var  // Receiver
}

func (m *Method) Kind() string { return "method" }
func (m *Method) Recv() *types.Var { return m.recv }

// Struct definition
type Struct struct {
    path     string
    name     string
    exported bool
    structType *types.Struct
}

// Interface definition
type Interface struct {
    path     string
    name     string
    exported bool
    iface    *types.Interface
}
```

### Indexer Implementation

```go
// internal/compiler/indexer/indexer.go
type Index struct {
    programs  *Program
    defs      map[string]def.Definition
    mains     []def.Definition
    inits     []def.Definition
}

func New(program *Program) (*Index, error) {
    idx := &Index{
        program: program,
        defs:    make(map[string]def.Definition),
    }
    
    // Index all packages
    for path, pkg := range program.Packages {
        idx.indexPackage(path, pkg)
    }
    
    return idx, nil
}

func (idx *Index) indexPackage(path string, pkg *Package) {
    // Index functions
    for name, fn := range pkg.Functions {
        def := &Function{
            path:     path,
            name:     name,
            exported: ast.IsExported(name),
            funcType: fn,
        }
        idx.defs[def.ID()] = def
        
        if name == "main" {
            idx.mains = append(idx.mains, def)
        }
    }
    
    // Index types (structs, interfaces)
    for name, t := range pkg.Types {
        if structType, ok := t.Type().(*types.Struct); ok {
            def := &Struct{
                path:     path,
                name:     name,
                exported: ast.IsExported(name),
                structType: structType,
            }
            idx.defs[def.ID()] = def
        }
    }
}
```

## Stage 3: Dependency Graph

### Graph Construction

```go
// internal/compiler/graph/graph.go
type Graph struct {
    edges map[string][]string  // from -> [to]
}

func New() *Graph {
    return &Graph{
        edges: make(map[string][]string),
    }
}

// Add edge from -> to
func (g *Graph) Edge(from, to def.Definition) {
    fromID := from.ID()
    toID := to.ID()
    g.edges[fromID] = append(g.edges[fromID], toID)
}

// Topological sort starting from a root
func (g *Graph) Toposort(root def.Definition) []string {
    var sorted []string
    visited := make(map[string]bool)
    
    var visit func(id string)
    visit = func(id string) {
        if visited[id] {
            return
        }
        visited[id] = true
        
        // Visit dependencies first
        for _, depID := range g.edges[id] {
            visit(depID)
        }
        
        sorted = append(sorted, id)
    }
    
    visit(root.ID())
    return sorted
}
```

### Dependency Resolution

```go
// Extract dependencies from a function
func (f *Function) Dependencies() ([]def.Definition, error) {
    deps := []def.Definition{}
    
    // Walk the AST to find references
    ast.Inspect(f.body, func(n ast.Node) bool {
        switch node := n.(type) {
        case *ast.CallExpr:
            // Function call - depends on the function
            if ident, ok := node.Fun.(*ast.Ident); ok {
                if def := f.resolve(ident.Name); def != nil {
                    deps = append(deps, def)
                }
            }
        
        case *ast.SelectorExpr:
            // Method call or package reference
            if sel, ok := node.Sel.(*ast.Ident); ok {
                if def := f.resolve(sel.Name); def != nil {
                    deps = append(deps, def)
                }
            }
        
        case *ast.CompositeLit:
            // Struct literal - depends on the struct type
            if typ, ok := node.Type.(*ast.Ident); ok {
                if def := f.resolve(typ.Name); def != nil {
                    deps = append(deps, def)
                }
            }
        }
        return true
    })
    
    return deps, nil
}
```

## Stage 4: Translation

### Translator Structure

```go
// internal/compiler/translator/translator.go
type Translator struct {
    index  *index.Index
    scope  *Scope
}

func New(idx *index.Index) *Translator {
    return &Translator{
        index: idx,
        scope: NewScope(),
    }
}

// Translate a definition to JavaScript AST
func (t *Translator) Translate(def def.Definition) (jsast.INode, error) {
    switch d := def.(type) {
    case *Function:
        return t.translateFunction(d)
    case *Method:
        return t.translateMethod(d)
    case *Struct:
        return t.translateStruct(d)
    case *Interface:
        return t.translateInterface(d)
    default:
        return nil, fmt.Errorf("unknown definition type: %T", def)
    }
}
```

### Function Translation

```go
func (t *Translator) translateFunction(fn *Function) (*jsast.FunctionDeclaration, error) {
    // Translate parameters
    params := t.translateParams(fn.funcType.Type().(*types.Signature).Params())
    
    // Translate body
    body, err := t.translateStmts(fn.body.Body)
    if err != nil {
        return nil, err
    }
    
    return &jsast.FunctionDeclaration{
        ID:   &jsast.Identifier{Name: fn.name},
        Params: params,
        Body: jsast.FunctionBody{Body: body},
    }, nil
}

// Translate Go parameters to JS
func (t *Translator) translateParams(tuple *types.Tuple) []jsast.IPattern {
    var params []jsast.IPattern
    for i := 0; i < tuple.Len(); i++ {
        param := tuple.At(i)
        params = append(params, &jsast.Identifier{
            Name: t.scope.declare(param.Name()),
        })
    }
    return params
}
```

### Statement Translation

```go
func (t *Translator) translateStmts(stmts []ast.Stmt) ([]jsast.IStatement, error) {
    var result []jsast.IStatement
    
    for _, stmt := range stmts {
        jsStmt, err := t.translateStmt(stmt)
        if err != nil {
            return nil, err
        }
        result = append(result, jsStmt)
    }
    
    return result, nil
}

func (t *Translator) translateStmt(stmt ast.Stmt) (jsast.IStatement, error) {
    switch s := stmt.(type) {
    case *ast.ReturnStmt:
        return t.translateReturn(s)
    
    case *ast.AssignStmt:
        return t.translateAssign(s)
    
    case *ast.IfStmt:
        return t.translateIf(s)
    
    case *ast.ForStmt:
        return t.translateFor(s)
    
    case *ast.RangeStmt:
        return t.translateRange(s)
    
    case *ast.SwitchStmt:
        return t.translateSwitch(s)
    
    case *ast.ExprStmt:
        expr, err := t.translateExpr(s.X)
        if err != nil {
            return nil, err
        }
        return &jsast.ExpressionStatement{Expression: expr}, nil
    
    default:
        return nil, fmt.Errorf("unsupported statement: %T", stmt)
    }
}

func (t *Translator) translateReturn(s *ast.ReturnStmt) (*jsast.ReturnStatement, error) {
    if len(s.Results) == 0 {
        return &jsast.ReturnStatement{}, nil
    }
    
    // Go can return multiple values, JS cannot
    // Wrap in array or handle specially
    if len(s.Results) == 1 {
        expr, err := t.translateExpr(s.Results[0])
        return &jsast.ReturnStatement{Argument: expr}, err
    }
    
    // Multiple return values -> array
    elements := make([]jsast.IExpression, len(s.Results))
    for i, r := range s.Results {
        expr, err := t.translateExpr(r)
        if err != nil {
            return nil, err
        }
        elements[i] = expr
    }
    
    return &jsast.ReturnStatement{
        Argument: &jsast.ArrayExpression{Elements: elements},
    }, nil
}
```

### Expression Translation

```go
func (t *Translator) translateExpr(expr ast.Expr) (jsast.IExpression, error) {
    switch e := expr.(type) {
    case *ast.Ident:
        return t.translateIdent(e)
    
    case *ast.BinaryExpr:
        return t.translateBinary(e)
    
    case *ast.CallExpr:
        return t.translateCall(e)
    
    case *ast.SelectorExpr:
        return t.translateSelector(e)
    
    case *ast.BasicLit:
        return t.translateBasicLit(e)
    
    case *ast.CompositeLit:
        return t.translateComposite(e)
    
    case *ast.SliceExpr:
        return t.translateSlice(e)
    
    case *ast.IndexExpr:
        return t.translateIndex(e)
    
    default:
        return nil, fmt.Errorf("unsupported expression: %T", expr)
    }
}

func (t *Translator) translateBinary(e *ast.BinaryExpr) (*jsast.BinaryExpression, error) {
    left, err := t.translateExpr(e.X)
    if err != nil {
        return nil, err
    }
    
    right, err := t.translateExpr(e.Y)
    if err != nil {
        return nil, err
    }
    
    op := t.translateOp(e.Op)
    
    return &jsast.BinaryExpression{
        Operator: op,
        Left:     left,
        Right:    right,
    }, nil
}

func (t *Translator) translateOp(op token.Token) jsast.BinaryOperator {
    switch op {
    case token.ADD:
        return "+"
    case token.SUB:
        return "-"
    case token.MUL:
        return "*"
    case token.QUO:
        return "/"
    case token.REM:
        return "%"
    case token.EQL:
        return "==="
    case token.NEQ:
        return "!=="
    case token.LSS:
        return "<"
    case token.LEQ:
        return "<="
    case token.GTR:
        return ">"
    case token.GEQ:
        return ">="
    case token.LAND:
        return "&&"
    case token.LOR:
        return "||"
    default:
        return jsast.BinaryOperator(op.String())
    }
}
```

### Struct to Object Translation

```go
func (t *Translator) translateStruct(s *Struct) (*jsast.FunctionDeclaration, error) {
    // Generate constructor function
    fields := t.extractFields(s.structType)
    params := make([]jsast.IPattern, len(fields))
    
    for i, field := range fields {
        params[i] = &jsast.Identifier{Name: field.Name}
    }
    
    // Build constructor body
    var body []jsast.IStatement
    for _, field := range fields {
        body = append(body, &jsast.ExpressionStatement{
            Expression: &jsast.AssignmentExpression{
                Operator: "=",
                Left: &jsast.MemberExpression{
                    Object:   &jsast.Identifier{Name: "this"},
                    Property: &jsast.Identifier{Name: field.Name},
                    Computed: false,
                },
                Right: &jsast.Identifier{Name: field.Name},
            },
        })
    }
    
    return &jsast.FunctionDeclaration{
        ID:     &jsast.Identifier{Name: s.name},
        Params: params,
        Body:   jsast.FunctionBody{Body: body},
    }, nil
}
```

## Stage 5: Code Assembly

### Module Pattern Generation

```go
// Group definitions into modules (files)
func group(definitions []def.Definition) ([]*module, error) {
    moduleMap := map[string]*module{}
    
    for _, def := range definitions {
        path := def.Path()
        
        if moduleMap[path] == nil {
            moduleMap[path] = &module{
                path:    path,
                defs:    []def.Definition{},
                exports: []string{},
                imports: map[string]string{},
            }
        }
        
        moduleMap[path].defs = append(moduleMap[path].defs, def)
        
        if def.Exported() && !def.Omitted() {
            moduleMap[path].exports = append(moduleMap[path].exports, def.Name())
        }
        
        // Track imports
        for alias, importPath := range def.Imports() {
            if moduleMap[importPath] != nil {
                moduleMap[path].imports[alias] = importPath
            }
        }
    }
    
    return modules, nil
}
```

### Import Linking

```go
// Generate import linking code
func generateImports(module *module) []jsast.IStatement {
    var stmts []jsast.IStatement
    
    for alias, path := range module.imports {
        // var alias = pkg["path/to/module"]
        stmts = append(stmts, &jsast.VariableDeclaration{
            Kind: "var",
            Declarations: []jsast.VariableDeclarator{{
                ID:   &jsast.Identifier{Name: alias},
                Init: &jsast.MemberExpression{
                    Object:   &jsast.Identifier{Name: "pkg"},
                    Property: &jsast.StringLiteral{Value: path},
                    Computed: true,
                },
            }},
        })
    }
    
    return stmts
}
```

### Export Generation

```go
// Generate return statement with exports
func generateExports(exports []string) *jsast.ReturnStatement {
    props := make([]jsast.Property, len(exports))
    
    for i, name := range exports {
        props[i] = jsast.CreateProperty(
            jsast.CreateIdentifier(name),
            jsast.CreateIdentifier(name),
            "init",
        )
    }
    
    return &jsast.ReturnStatement{
        Argument: &jsast.ObjectExpression{Properties: props},
    }
}
```

### Final Assembly

```go
// Assemble complete JavaScript file
func assembleFile(file *file) (*script.Script, error) {
    var body []jsast.IStatement
    
    // Create pkg object: var pkg = {}
    body = append(body, jsast.CreateVariableDeclaration(
        "var",
        jsast.CreateVariableDeclarator(
            jsast.CreateIdentifier("pkg"),
            jsast.CreateObjectExpression([]jsast.Property{}),
        ),
    ))
    
    // Add modules
    for _, module := range file.modules {
        // Generate module body
        var modBody []jsast.IStatement
        
        // Import statements
        modBody = append(modBody, generateImports(module)...)
        
        // Definition translations
        for _, def := range module.defs {
            ast, _ := translator.Translate(def)
            modBody = append(modBody, ast)
        }
        
        // Export statement
        modBody = append(modBody, generateExports(module.exports))
        
        // Wrap in IIFE: pkg["path"] = (function() { ... })()
        body = append(body, jsast.CreateExpressionStatement(
            jsast.CreateAssignmentExpression(
                jsast.CreateMemberExpression(
                    jsast.CreateIdentifier("pkg"),
                    jsast.CreateString(module.path),
                    true,
                ),
                "=",
                jsast.CreateCallExpression(
                    jsast.CreateFunctionExpression(
                        nil,
                        []jsast.IPattern{},
                        jsast.CreateFunctionBody(modBody...),
                    ),
                    []jsast.IExpression{},
                ),
            ),
        ))
    }
    
    // Call main: pkg["main"].main()
    body = append(body, jsast.CreateReturnStatement(
        jsast.CreateCallExpression(
            jsast.CreateMemberExpression(
                jsast.CreateMemberExpression(
                    jsast.CreateIdentifier("pkg"),
                    jsast.CreateString(file.path),
                    true,
                ),
                jsast.CreateIdentifier("main"),
                false,
            ),
            []jsast.IExpression{},
        ),
    ))
    
    // Wrap everything in IIFE
    program := jsast.CreateProgram(
        jsast.CreateExpressionStatement(
            jsast.CreateCallExpression(
                jsast.CreateFunctionExpression(
                    nil,
                    []jsast.IPattern{},
                    jsast.CreateFunctionBody(body...),
                ),
                []jsast.IExpression{},
            ),
        ),
    )
    
    code, err := jsast.Assemble(program)
    return script.New(file.path, file.path, code), err
}
```

## DOM Binding Generation

Joy generates JavaScript bindings for Web APIs by parsing Web IDL:

```go
// Internal structure for DOM definitions
type DOMDef struct {
    Name       string
    Interface  bool
    Properties []Property
    Methods    []Method
    Events     []Event
}

// Parse Web IDL
func parseIDL(idlSource string) ([]*DOMDef, error) {
    // Parse IDL to AST
    ast := parseIDLString(idlSource)
    
    // Extract interface definitions
    var defs []*DOMDef
    for _, node := range ast.Nodes {
        if iface, ok := node.(*IDLInterface); ok {
            def := &DOMDef{
                Name:      iface.Name,
                Interface: true,
            }
            
            // Extract members
            for _, member := range iface.Members {
                switch m := member.(type) {
                case *IDLAttribute:
                    def.Properties = append(def.Properties, Property{
                        Name: m.Name,
                        Type: m.Type,
                    })
                case *IDLOperation:
                    def.Methods = append(def.Methods, Method{
                        Name:       m.Name,
                        Parameters: m.Parameters,
                        Return:     m.Return,
                    })
                }
            }
            
            defs = append(defs, def)
        }
    }
    
    return defs, nil
}

// Generate JavaScript bindings
func generateBinding(def *DOMDef) string {
    var buf strings.Builder
    
    fmt.Fprintf(&buf, "function %s() {}\n", def.Name)
    
    // Properties
    for _, prop := range def.Properties {
        fmt.Fprintf(&buf, "%s.prototype.%s = undefined;\n", def.Name, prop.Name)
    }
    
    // Methods
    for _, method := range def.Methods {
        params := strings.Join(method.Parameters, ", ")
        fmt.Fprintf(&buf, "%s.prototype.%s = function(%s) {};\n",
            def.Name, method.Name, params)
    }
    
    return buf.String()
}
```

## Standard Library Mapping

Joy maps Go standard library functions to JavaScript equivalents:

```go
// fmt package mapping
map[string]string{
    "fmt.Printf":  "console.log",
    "fmt.Println": "console.log",
    "fmt.Sprintf": "sprintf",  // External library
}

// strings package mapping
map[string]string{
    "strings.Replace":  "replace",
    "strings.Split":    "split",
    "strings.Join":     "join",
    "strings.ToUpper":  "toUpperCase",
    "strings.ToLower":  "toLowerCase",
    "strings.TrimSpace": "trim",
}

// math package - direct JavaScript Math mapping
map[string]string{
    "math.Abs":    "Math.abs",
    "math.Max":    "Math.max",
    "math.Min":    "Math.min",
    "math.Pow":    "Math.pow",
    "math.Sqrt":   "Math.sqrt",
    "math.Sin":    "Math.sin",
    "math.Cos":    "Math.cos",
    "math.Tan":    "Math.tan",
}
```

## Summary

The Joy compiler demonstrates:

1. **Multi-stage compilation** - Parse → Index → Graph → Translate → Assemble
2. **Go type system usage** - Leverages go/types for semantic analysis
3. **AST translation** - Systematic Go AST to JavaScript AST mapping
4. **Module pattern** - IIFE-based module wrapping for isolation
5. **Import linking** - Cross-package reference resolution
6. **DOM bindings** - Generated from Web IDL
7. **Standard library mapping** - Go stdlib to JavaScript equivalents
