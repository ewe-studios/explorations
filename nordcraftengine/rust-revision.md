# Nordcraft Rust Revision

## Overview

This document translates Nordcraft's visual web development engine concepts to Rust. We examine how to build a similar system using Rust's ecosystem, focusing on component architecture, reactive data flow, styling, and server-side rendering.

## Core Architecture Translation

### TypeScript vs Rust Type System

```rust
// TypeScript Component interface
interface Component {
  name: string
  attributes: Record<string, ComponentAttribute>
  variables: Record<string, ComponentVariable>
  formulas: Record<string, ComponentFormula>
  workflows: Record<string, ComponentWorkflow>
  nodes: Record<string, NodeModel>
}

// Rust equivalent
#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub attributes: HashMap<String, ComponentAttribute>,
    pub variables: HashMap<String, ComponentVariable>,
    pub formulas: HashMap<String, ComponentFormula>,
    pub workflows: HashMap<String, ComponentWorkflow>,
    pub nodes: NodeTree,
}

// Node model - tagged enum for type safety
#[derive(Debug, Clone)]
pub enum NodeModel {
    Text(TextNode),
    Element(ElementNode),
    Component(ComponentInstance),
    Slot(SlotNode),
}

#[derive(Debug, Clone)]
pub struct ElementNode {
    pub id: String,
    pub tag: String,
    pub attrs: HashMap<String, Formula>,
    pub style: StyleMap,
    pub children: Vec<String>,
    pub condition: Option<Formula>,
    pub repeat: Option<Formula>,
}
```

### Formula System in Rust

```rust
// Formula AST with type-safe operations
#[derive(Debug, Clone)]
pub enum Formula {
    Static(Json),
    Variable { name: String },
    Argument { name: String },
    Api { name: String, path: Option<String> },
    Operation {
        op: Operation,
        args: Vec<Formula>,
    },
}

#[derive(Debug, Clone)]
pub enum Operation {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    
    // Comparison
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    
    // Logical
    And,
    Or,
    Not,
    
    // Conditional
    If,
    
    // String operations
    Concat,
    Upper,
    Lower,
    
    // Array operations
    Map,
    Filter,
    Reduce,
    Get,
    
    // Object operations
    ObjectGet,
    ObjectSet,
    ObjectKeys,
}

// Formula evaluation context
pub struct FormulaContext<'a> {
    pub variables: &'a HashMap<String, Json>,
    pub arguments: &'a HashMap<String, Json>,
    pub apis: &'a HashMap<String, ApiState>,
    pub list_item: Option<&'a ListItem>,
}

// Formula evaluator
pub struct FormulaEvaluator {
    handlers: HashMap<String, FormulaHandler>,
}

type FormulaHandler = fn(&[Json], &FormulaContext) -> Result<Json>;

impl FormulaEvaluator {
    pub fn new() -> Self {
        let mut evaluator = Self {
            handlers: HashMap::new(),
        };
        evaluator.register_builtins();
        evaluator
    }
    
    fn register_builtins(&mut self) {
        self.register("add", |args, _| {
            let sum: f64 = args.iter()
                .filter_map(|v| v.as_f64())
                .sum();
            Ok(Json::from(sum))
        });
        
        self.register("concat", |args, _| {
            let concatenated: String = args.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("");
            Ok(Json::from(concatenated))
        });
        
        self.register("eq", |args, _| {
            Ok(Json::from(args[0] == args[1]))
        });
        
        self.register("if", |args, _| {
            if args[0].as_bool().unwrap_or(false) {
                Ok(args[1].clone())
            } else {
                Ok(args[2].clone())
            }
        });
    }
    
    pub fn register(&mut self, name: &'static str, handler: FormulaHandler) {
        self.handlers.insert(name.to_string(), handler);
    }
    
    pub fn evaluate(&self, formula: &Formula, context: &FormulaContext) -> Result<Json> {
        match formula {
            Formula::Static(value) => Ok(value.clone()),
            
            Formula::Variable { name } => {
                Ok(context.variables.get(name).cloned().unwrap_or(Json::Null))
            }
            
            Formula::Argument { name } => {
                Ok(context.arguments.get(name).cloned().unwrap_or(Json::Null))
            }
            
            Formula::Api { name, path } => {
                let api_state = context.apis.get(name);
                match (api_state, path) {
                    (Some(ApiState::Success { response, .. }), Some(p)) => {
                        Ok(json_pointer_get(response, p))
                    }
                    (Some(ApiState::Success { response, .. }), None) => {
                        Ok(response.clone())
                    }
                    _ => Ok(Json::Null),
                }
            }
            
            Formula::Operation { op, args } => {
                let evaluated_args: Result<Vec<Json>> = args
                    .iter()
                    .map(|arg| self.evaluate(arg, context))
                    .collect();
                
                let handler = self.handlers.get(&format!("{:?}", op))
                    .ok_or_else(|| anyhow!("Unknown operation: {:?}", op))?;
                
                handler(&evaluated_args?, context)
            }
        }
    }
}
```

### Reactive Variables with Signals

```rust
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashSet;

// Signal for reactive values
pub struct Signal<T> {
    value: RefCell<T>,
    subscribers: RefCell<HashSet<usize>>,
    version: RefCell<u32>,
}

impl<T: Clone + PartialEq> Signal<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
            subscribers: RefCell::new(HashSet::new()),
            version: RefCell::new(0),
        }
    }
    
    pub fn get(&self) -> T {
        // Track dependency if in reactive context
        if let Some(effect_id) = CURRENT_EFFECT.with(|e| *e.borrow()) {
            self.subscribers.borrow_mut().insert(effect_id);
        }
        self.value.borrow().clone()
    }
    
    pub fn set(&self, new_value: T) {
        let mut value = self.value.borrow_mut();
        if *value != new_value {
            *value = new_value;
            *self.version.borrow_mut() += 1;
            // Notify subscribers would happen here
        }
    }
}

// Effect for reactive computations
thread_local! {
    static CURRENT_EFFECT: RefCell<Option<usize>> = RefCell::new(None);
}

pub struct Effect {
    id: usize,
    callback: Box<dyn Fn()>,
    dependencies: RefCell<HashSet<usize>>,
}

impl Effect {
    pub fn new<F: Fn() + 'static>(callback: F) -> Self {
        Self {
            id: rand::random(),
            callback: Box::new(callback),
            dependencies: RefCell::new(HashSet::new()),
        }
    }
    
    pub fn run(&self) {
        CURRENT_EFFECT.with(|e| {
            *e.borrow_mut() = Some(self.id);
            (self.callback)();
            *e.borrow_mut() = None;
        });
    }
}

// Variable store for component state
pub struct VariableStore {
    signals: HashMap<String, Rc<Signal<Json>>>,
    effects: Vec<Effect>,
}

impl VariableStore {
    pub fn new() -> Self {
        Self {
            signals: HashMap::new(),
            effects: Vec::new(),
        }
    }
    
    pub fn define(&mut self, name: &str, initial_value: Json) {
        self.signals.insert(
            name.to_string(),
            Rc::new(Signal::new(initial_value)),
        );
    }
    
    pub fn get(&self, name: &str) -> Option<Json> {
        self.signals.get(name).map(|s| s.get())
    }
    
    pub fn set(&self, name: &str, value: Json) -> Result<()> {
        let signal = self.signals.get(name)
            .ok_or_else(|| anyhow!("Variable not found: {}", name))?;
        signal.set(value);
        Ok(())
    }
    
    pub fn effect<F: Fn() + 'static>(&mut self, callback: F) {
        self.effects.push(Effect::new(callback));
    }
}
```

## Component System in Rust

### Component Definition

```rust
#[derive(Debug, Clone)]
pub struct ComponentDefinition {
    pub name: String,
    pub attributes: Vec<AttributeDefinition>,
    pub variables: Vec<VariableDefinition>,
    pub template: ComponentTemplate,
    pub event_handlers: HashMap<String, Workflow>,
}

#[derive(Debug, Clone)]
pub struct AttributeDefinition {
    pub name: String,
    pub default_value: Option<Json>,
}

#[derive(Debug, Clone)]
pub struct VariableDefinition {
    pub name: String,
    pub initial_value: Formula,
}

// Component template - similar to NodeModel tree
#[derive(Debug, Clone)]
pub struct ComponentTemplate {
    pub root: Rc<NodeModel>,
}

// Component instance (when used in parent)
pub struct ComponentInstance {
    pub component: Rc<ComponentDefinition>,
    pub attributes: HashMap<String, Json>,
    pub internal_state: VariableStore,
}

impl ComponentInstance {
    pub fn new(component: Rc<ComponentDefinition>) -> Self {
        let mut state = VariableStore::new();
        
        // Initialize variables
        for var_def in &component.variables {
            state.define(&var_def.name, Json::Null);
        }
        
        Self {
            component,
            attributes: HashMap::new(),
            internal_state: state,
        }
    }
    
    pub fn set_attribute(&mut self, name: &str, value: Json) {
        self.attributes.insert(name.to_string(), value);
    }
}
```

### Component Runtime

```rust
pub struct ComponentRuntime {
    components: HashMap<String, Rc<ComponentDefinition>>,
    instances: HashMap<String, ComponentInstance>,
    evaluator: FormulaEvaluator,
}

impl ComponentRuntime {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            instances: HashMap::new(),
            evaluator: FormulaEvaluator::new(),
        }
    }
    
    pub fn register_component(&mut self, component: ComponentDefinition) {
        self.components.insert(
            component.name.clone(),
            Rc::new(component),
        );
    }
    
    pub fn create_instance(&mut self, id: &str, component_name: &str) -> Result<()> {
        let component = self.components.get(component_name)
            .ok_or_else(|| anyhow!("Component not found: {}", component_name))?
            .clone();
        
        let instance = ComponentInstance::new(component);
        self.instances.insert(id.to_string(), instance);
        Ok(())
    }
    
    pub fn render(&self, instance_id: &str) -> Result<String> {
        let instance = self.instances.get(instance_id)
            .ok_or_else(|| anyhow!("Instance not found: {}", instance_id))?;
        
        self.render_node(&instance.component.template.root, instance)
    }
    
    fn render_node(&self, node: &NodeModel, instance: &ComponentInstance) -> Result<String> {
        match node {
            NodeModel::Element(el) => self.render_element(el, instance),
            NodeModel::Text(text) => self.render_text(text, instance),
            NodeModel::Component(comp) => self.render_component_instance(comp, instance),
            NodeModel::Slot(_) => Ok(String::new()),
        }
    }
    
    fn render_element(&self, el: &ElementNode, instance: &ComponentInstance) -> Result<String> {
        // Check condition
        if let Some(condition) = &el.condition {
            let ctx = self.create_formula_context(instance);
            let should_show = self.evaluator.evaluate(condition, &ctx)?
                .as_bool().unwrap_or(false);
            if !should_show {
                return Ok(String::new());
            }
        }
        
        // Handle repeat
        if let Some(repeat) = &el.repeat {
            return self.render_repeated_element(el, repeat, instance);
        }
        
        let mut html = format!("<{}", el.tag);
        
        // Add attributes
        for (name, formula) in &el.attrs {
            let ctx = self.create_formula_context(instance);
            let value = self.evaluator.evaluate(formula, &ctx)?;
            if let Some(s) = value.as_str() {
                html.push_str(&format!(" {}=\"{}\"", name, s));
            }
        }
        
        // Add styles
        let styles = self.render_styles(&el.style);
        if !styles.is_empty() {
            html.push_str(&format!(" style=\"{}\"", styles));
        }
        
        html.push('>');
        
        // Render children
        for child_id in &el.children {
            // Look up child node and render
        }
        
        html.push_str(&format!("</{}>", el.tag));
        Ok(html)
    }
    
    fn render_styles(&self, style: &StyleMap) -> String {
        style.iter()
            .map(|(k, v)| format!("{}: {}", to_kebab_case(k), v))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
```

## Workflow System in Rust

```rust
#[derive(Debug, Clone)]
pub struct Workflow {
    pub name: String,
    pub parameters: Vec<ParameterDefinition>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone)]
pub struct ParameterDefinition {
    pub name: String,
    pub default_value: Option<Json>,
}

#[derive(Debug, Clone)]
pub enum Action {
    SetVariable {
        variable: String,
        value: Formula,
    },
    TriggerEvent {
        event: String,
        data: Formula,
    },
    Fetch {
        api: String,
        inputs: HashMap<String, Formula>,
        on_success: Option<Box<Workflow>>,
        on_error: Option<Box<Workflow>>,
    },
    Switch {
        value: Formula,
        cases: Vec<SwitchCase>,
        default: Box<Workflow>,
    },
    TriggerWorkflow {
        workflow: String,
        parameters: HashMap<String, Formula>,
    },
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub condition: Formula,
    pub actions: Vec<Action>,
}

// Workflow executor
pub struct WorkflowExecutor<'a> {
    runtime: &'a mut ComponentRuntime,
    variables: &'a mut VariableStore,
}

impl<'a> WorkflowExecutor<'a> {
    pub async fn execute(
        &mut self,
        workflow: &Workflow,
        params: HashMap<String, Json>,
    ) -> Result<()> {
        for action in &workflow.actions {
            self.execute_action(action, &params).await?;
        }
        Ok(())
    }
    
    async fn execute_action(
        &mut self,
        action: &Action,
        params: &HashMap<String, Json>,
    ) -> Result<()> {
        match action {
            Action::SetVariable { variable, value } => {
                let ctx = self.create_context(params);
                let evaluated = self.runtime.evaluator.evaluate(value, &ctx)?;
                self.variables.set(variable, evaluated)?;
            }
            
            Action::Fetch { api, inputs, on_success, on_error } => {
                // Evaluate inputs
                let ctx = self.create_context(params);
                let evaluated_inputs: HashMap<String, Json> = inputs
                    .iter()
                    .map(|(k, v)| {
                        let value = self.runtime.evaluator.evaluate(v, &ctx)?;
                        Ok((k.clone(), value))
                    })
                    .collect::<Result<_>>()?;
                
                // Execute fetch
                match self.execute_fetch(api, &evaluated_inputs).await {
                    Ok(response) => {
                        if let Some(success_workflow) = on_success {
                            // Add response to context
                            self.execute(success_workflow, params).await?;
                        }
                    }
                    Err(e) => {
                        if let Some(error_workflow) = on_error {
                            self.execute(error_workflow, params).await?;
                        }
                    }
                }
            }
            
            Action::Switch { value, cases, default } => {
                let ctx = self.create_context(params);
                let switch_value = self.runtime.evaluator.evaluate(value, &ctx)?;
                
                // Find matching case
                let mut executed = false;
                for case in cases {
                    let condition_result = self.runtime.evaluator.evaluate(
                        &case.condition,
                        &ctx,
                    )?;
                    
                    if condition_result.as_bool().unwrap_or(false) {
                        // Execute case actions
                        for action in &case.actions {
                            self.execute_action(action, params).await?;
                        }
                        executed = true;
                        break;
                    }
                }
                
                // Execute default if no case matched
                if !executed {
                    self.execute(default, params).await?;
                }
            }
            
            Action::TriggerWorkflow { workflow, parameters } => {
                let ctx = self.create_context(params);
                let evaluated_params: HashMap<String, Json> = parameters
                    .iter()
                    .map(|(k, v)| {
                        let value = self.runtime.evaluator.evaluate(v, &ctx)?;
                        Ok((k.clone(), value))
                    })
                    .collect::<Result<_>>()?;
                
                let workflow_def = self.runtime.get_workflow(workflow)?;
                self.execute(workflow_def, evaluated_params).await?;
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    async fn execute_fetch(
        &self,
        api: &str,
        inputs: &HashMap<String, Json>,
    ) -> Result<Json> {
        // Use reqwest or similar for HTTP requests
        use reqwest::Client;
        
        let client = Client::new();
        let config = self.runtime.get_api_config(api)?;
        
        let response = match config.method.as_str() {
            "GET" => client.get(&config.url).send().await?,
            "POST" => client.post(&config.url).json(inputs).send().await?,
            "PUT" => client.put(&config.url).json(inputs).send().await?,
            "DELETE" => client.delete(&config.url).send().await?,
            _ => return Err(anyhow!("Unsupported HTTP method")),
        }
        .json::<Json>()
        .await?;
        
        Ok(response)
    }
    
    fn create_context(&self, params: &HashMap<String, Json>) -> FormulaContext {
        FormulaContext {
            variables: &self.variables.get_all(),
            arguments: params,
            apis: &self.runtime.api_states,
            list_item: None,
        }
    }
}
```

## Styling Engine in Rust

```rust
#[derive(Debug, Clone, Default)]
pub struct StyleMap {
    properties: HashMap<String, String>,
}

impl StyleMap {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }
    
    pub fn insert(&mut self, property: &str, value: String) {
        self.properties.insert(to_kebab_case(property), value);
    }
    
    pub fn to_css(&self) -> String {
        self.properties
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

// Style variant for conditional styles
#[derive(Debug, Clone)]
pub struct StyleVariant {
    pub id: String,
    pub condition: StyleCondition,
    pub styles: StyleMap,
}

#[derive(Debug, Clone)]
pub enum StyleCondition {
    PseudoClass(PseudoClass),
    MediaQuery(MediaQuery),
    Class(String),
    Combined(Vec<StyleCondition>),
}

#[derive(Debug, Clone)]
pub enum PseudoClass {
    Hover,
    Focus,
    Active,
    Disabled,
    FirstChild,
    LastChild,
}

#[derive(Debug, Clone)]
pub struct MediaQuery {
    pub min_width: Option<String>,
    pub max_width: Option<String>,
    pub min_height: Option<String>,
    pub max_height: Option<String>,
}

// CSS generator
pub struct CssGenerator {
    class_prefix: String,
}

impl CssGenerator {
    pub fn new(prefix: &str) -> Self {
        Self {
            class_prefix: prefix.to_string(),
        }
    }
    
    pub fn generate(&self, base_style: &StyleMap, variants: &[StyleVariant]) -> CssOutput {
        let base_class = self.generate_class_name("base");
        let mut css = format!(".{} {{ {} }}\n", base_class, base_style.to_css());
        
        let mut variant_classes = Vec::new();
        for variant in variants {
            let variant_class = self.generate_class_name(&variant.id);
            let selector = self.build_variant_selector(&variant_class, &variant.condition);
            css.push_str(&format!("{} {{ {} }}\n", selector, variant.styles.to_css()));
            variant_classes.push(variant_class);
        }
        
        CssOutput {
            base_class,
            variant_classes,
            css,
        }
    }
    
    fn generate_class_name(&self, suffix: &str) -> String {
        format!("{}_{}", self.class_prefix, suffix)
    }
    
    fn build_variant_selector(&self, class: &str, condition: &StyleCondition) -> String {
        match condition {
            StyleCondition::PseudoClass(pc) => {
                format!(".{}:{}", class, self.pseudo_class_to_string(pc))
            }
            StyleCondition::MediaQuery(mq) => {
                format!("@media {} {{ .{} }}", self.media_query_to_string(mq), class)
            }
            StyleCondition::Class(name) => {
                format!(".{}.{}", class, name)
            }
            StyleCondition::Combined(conditions) => {
                conditions.iter()
                    .map(|c| self.build_variant_selector(class, c))
                    .collect::<Vec<_>>()
                    .join("")
            }
        }
    }
    
    fn pseudo_class_to_string(&self, pc: &PseudoClass) -> &'static str {
        match pc {
            PseudoClass::Hover => "hover",
            PseudoClass::Focus => "focus",
            PseudoClass::Active => "active",
            PseudoClass::Disabled => "disabled",
            PseudoClass::FirstChild => "first-child",
            PseudoClass::LastChild => "last-child",
        }
    }
    
    fn media_query_to_string(&self, mq: &MediaQuery) -> String {
        let mut parts = Vec::new();
        if let Some(min) = &mq.min_width {
            parts.push(format!("(min-width: {})", min));
        }
        if let Some(max) = &mq.max_width {
            parts.push(format!("(max-width: {})", max));
        }
        if let Some(min) = &mq.min_height {
            parts.push(format!("(min-height: {})", min));
        }
        if let Some(max) = &mq.max_height {
            parts.push(format!("(max-height: {})", max));
        }
        parts.join(" and ")
    }
}

pub struct CssOutput {
    pub base_class: String,
    pub variant_classes: Vec<String>,
    pub css: String,
}
```

## Server-Side Rendering in Rust

```rust
use axum::{
    extract::Path,
    response::Html,
    routing::get,
    Router,
};
use tokio::sync::RwLock;

// SSR renderer
pub struct SsrRenderer {
    components: HashMap<String, Rc<ComponentDefinition>>,
}

impl SsrRenderer {
    pub fn render_page(
        &self,
        page: &str,
        params: &HashMap<String, String>,
    ) -> Result<String> {
        let component = self.components.get(page)
            .ok_or_else(|| anyhow!("Page not found: {}", page))?;
        
        let mut instance = ComponentInstance::new(component.clone());
        
        // Set URL parameters as variables
        for (key, value) in params {
            instance.internal_state.set(key, Json::from(value.clone()))?;
        }
        
        // Execute onLoad workflow if present
        // Fetch APIs with auto-fetch enabled
        
        // Render to HTML
        let html = self.render_to_html(&instance)?;
        
        Ok(html)
    }
    
    fn render_to_html(&self, instance: &ComponentInstance) -> Result<String> {
        // Similar to render_node in ComponentRuntime
        // but optimized for SSR (no event handlers, etc.)
        Ok(String::new())
    }
}

// Axum web server
pub async fn create_server() -> Router {
    let state = AppState {
        renderer: RwLock::new(SsrRenderer::new()),
    };
    
    Router::new()
        .route("/:page", get(render_page_handler))
        .route("/api/:endpoint", get(api_handler))
        .with_state(state)
}

#[derive(Clone)]
struct AppState {
    renderer: RwLock<SsrRenderer>,
}

async fn render_page_handler(
    Path(page): Path<String>,
) -> Html<String> {
    // Handle SSR page rendering
    Html("<html>...</html>".to_string())
}
```

## Full Stack Application with Leptos

```rust
use leptos::*;
use serde::{Deserialize, Serialize};

// Component props (attributes)
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonProps {
    pub label: String,
    pub variant: ButtonVariant,
    pub disabled: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
}

// Leptos component
#[component]
fn Button(
    cx: Scope,
    label: String,
    #[prop(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    #[prop(default = false)]
    disabled: bool,
) -> impl IntoView {
    // Internal state (like Nordcraft variables)
    let is_hovered = create_signal(cx, false);
    
    // Computed style (like Nordcraft style variables)
    let background_color = move || match variant {
        ButtonVariant::Primary => "#007bff",
        ButtonVariant::Secondary => "#6c757d",
        ButtonVariant::Danger => "#dc3545",
    };
    
    view! { cx,
        <button
            class="btn"
            class:btn-primary=move || variant == ButtonVariant::Primary
            class:btn-secondary=move || variant == ButtonVariant::Secondary
            class:btn-danger=move || variant == ButtonVariant::Danger
            class:btn-disabled=move || disabled
            style=move || format!("background-color: {}", background_color())
            disabled=disabled
            on:mouseenter=move |_| is_hovered.set(true)
            on:mouseleave=move |_| is_hovered.set(false)
        >
            {label}
        </button>
    }
}

// Page component with data fetching
#[component]
fn UserProfile(cx: Scope) -> impl IntoView {
    // URL parameter (like Nordcraft URL params)
    let params = use_params_map(cx);
    let user_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());
    
    // API fetch (like Nordcraft auto-fetch APIs)
    let user_data = create_resource(cx, user_id, |id| async move {
        fetch_user_data(id).await
    });
    
    // Loading state
    let loading = move || user_data.with(|data| data.is_loading());
    
    // Error state
    let error = move || user_data.with(|data| data.error().cloned());
    
    view! { cx,
        <Show
            when=move || error().is_some()
            fallback=|| view! { cx, <div>"Loading..."</div> }
        >
            <div class="error">
                {move || error().unwrap_or_default()}
            </div>
        </Show>
        
        <Suspense fallback=move || view! { cx, <div>"Loading..."</div> }>
            {move || {
                user_data.with(|data| {
                    data.data().map(|user| {
                        view! { cx,
                            <div class="user-profile">
                                <h1>{user.name.clone()}</h1>
                                <p>{user.email.clone()}</p>
                            </div>
                        }
                    })
                })
            }}
        </Suspense>
    }
}

async fn fetch_user_data(user_id: String) -> Result<User, ServerFnError> {
    let response = reqwest::get(&format!("/api/users/{}", user_id))
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .json::<User>()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    
    Ok(response)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}
```

## Workflow Actions with Server Functions

```rust
use leptos::*;
use serde::{Deserialize, Serialize};

// Server action (like Nordcraft workflow with Fetch action)
#[server(UpdateUserProfile, "/api")]
pub async fn update_user_profile(
    user_id: String,
    name: String,
    email: String,
) -> Result<User, ServerFnError> {
    // Database update
    let user = sqlx::query_as!(
        User,
        r#"UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING *"#,
        name,
        email,
        user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    
    Ok(user)
}

// Component using server action
#[component]
fn EditProfileForm(cx: Scope, initial_user: User) -> impl IntoView {
    // Form state (like Nordcraft variables)
    let (name, set_name) = create_signal(cx, initial_user.name.clone());
    let (email, set_email) = create_signal(cx, initial_user.email.clone());
    let (is_submitting, set_is_submitting) = create_signal(cx, false);
    let (error, set_error) = create_signal(cx, Option::<String>::None);
    
    // Submit handler (like Nordcraft workflow)
    let submit = ActionForm::new(cx, move |form_data: FormData| async move {
        set_is_submitting.set(true);
        
        let result = update_user_profile(
            initial_user.id.clone(),
            form_data.get("name").unwrap_or_default(),
            form_data.get("email").unwrap_or_default(),
        )
        .await;
        
        set_is_submitting.set(false);
        
        match result {
            Ok(_) => Ok("/profile".to_string()), // Redirect on success
            Err(e) => {
                set_error.set(Some(e.to_string()));
                Err(e)
            }
        }
    });
    
    view! { cx,
        <Form action=submit>
            <input
                type="text"
                name="name"
                prop:value=move || name.get()
                on:input=move |ev| set_name.set(event_target_value(&ev))
            />
            <input
                type="email"
                name="email"
                prop:value=move || email.get()
                on:input=move |ev| set_email.set(event_target_value(&ev))
            />
            
            <Show when=move || error.with(|e| e.is_some())>
                <div class="error">{move || error.get().unwrap_or_default()}</div>
            </Show>
            
            <button type="submit" disabled=move || is_submitting.get()>
                {move || if is_submitting.get() { "Saving..." } else { "Save" }}
            </button>
        </Form>
    }
}
```

## Project Structure

```
nordcraft-rust/
├── core/                    # Core types and formula system
│   ├── src/
│   │   ├── formula.rs       # Formula AST and evaluator
│   │   ├── variable.rs      # Reactive variables
│   │   ├── workflow.rs      # Workflow system
│   │   ├── component.rs     # Component definition
│   │   └── lib.rs
│   └── Cargo.toml
│
├── runtime/                 # Client-side runtime
│   ├── src/
│   │   ├── renderer.rs      # DOM renderer
│   │   ├── hydration.rs     # Hydration logic
│   │   └── lib.rs
│   └── Cargo.toml
│
├── ssr/                     # Server-side rendering
│   ├── src/
│   │   ├── renderer.rs      # SSR renderer
│   │   └── lib.rs
│   └── Cargo.toml
│
├── web/                     # Web application (Leptos)
│   ├── src/
│   │   ├── app.rs           # Main app component
│   │   ├── components/      # Reusable components
│   │   └── main.rs
│   └── Cargo.toml
│
├── server/                  # Backend server (Axum)
│   ├── src/
│   │   ├── api.rs           # API endpoints
│   │   └── main.rs
│   └── Cargo.toml
│
└── Cargo.toml               # Workspace root
```

## Summary

The Rust implementation provides:

1. **Type Safety**: Compile-time guarantees for formula operations and component structure
2. **Reactive Variables**: Signal-based reactivity similar to Nordcraft's system
3. **Formula Evaluator**: Extensible formula system with custom handlers
4. **Workflow Execution**: Async workflow execution with proper error handling
5. **Component System**: Runtime component instantiation and rendering
6. **Styling Engine**: CSS generation with variants and media queries
7. **SSR Support**: Server-side rendering with Axum or Leptos
8. **Full-Stack Integration**: Server functions for type-safe API calls
9. **Performance**: Compiled code with optimized execution
10. **Ecosystem**: Access to Rust's rich ecosystem of libraries

This translation demonstrates how Nordcraft's visual development concepts can be implemented in Rust, providing a type-safe, performant alternative to the TypeScript implementation.
