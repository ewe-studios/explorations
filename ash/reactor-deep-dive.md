# Reactor Deep Dive

## Overview

This deep dive covers the Reactor ecosystem:
- **reactor**: Dynamic, concurrent, dependency-resolving saga orchestrator
- **reactor_file**: File-based reactor definitions
- **splode**: Error handling utilities

---

## Part 1: Reactor (`reactor` v0.15.6+)

### What is Reactor?

Reactor is a **dynamic, concurrent, dependency-resolving saga orchestrator**. Let's break down what that means:

- **Saga Orchestrator**: Coordinates multiple steps across different systems/resources with compensation on failure
- **Dependency Resolving**: Steps declare dependencies, Reactor computes execution order
- **Concurrent**: Independent steps run in parallel
- **Dynamic**: Workflows can be built at runtime, not just compile-time

### Use Cases

Reactor is ideal for:
- Multi-step business processes
- Data pipelines with dependencies
- Payment processing workflows
- User onboarding flows
- ETL operations
- Complex integrations

---

### Basic Example

```elixir
defmodule HelloWorldReactor do
  @moduledoc false
  use Reactor

  # Define inputs
  input :whom

  # Define steps
  step :greet, Greeter do
    argument :whom, input(:whom)
  end

  # Define output
  return :greet
end

# Run the reactor
{:ok, result} = Reactor.run(HelloWorldReactor, %{whom: "World"})
# => {:ok, "Hello, World!"}
```

---

### Core Concepts

#### 1. Steps

Steps are the atomic units of work in a Reactor:

```elixir
defmodule MyApp.WelcomeUser do
  use Reactor

  input :user_id
  input :send_email?, default: true

  # Step with dependencies
  step :fetch_user, GetUser do
    argument :user_id, input(:user_id)
  end

  # Step depends on fetch_user
  step :validate_user, ValidateUser do
    argument :user, step(:fetch_user)
  end

  # Step runs in parallel with validate_user (no dependencies)
  step :fetch_preferences, GetPreferences do
    argument :user_id, input(:user_id)
  end

  # Step depends on multiple previous steps
  step :send_welcome_email, SendEmail do
    argument :user, step(:fetch_user)
    argument :preferences, step(:fetch_preferences)
    condition input(:send_email?)
  end

  # Return multiple values
  return [:fetch_user, :send_welcome_email]
end
```

#### 2. Step Arguments

Steps can receive arguments from:

```elixir
# From reactor inputs
step :process, ProcessStep do
  argument :data, input(:data)
end

# From previous step results
step :transform, TransformStep do
  argument :result, step(:process)
end

# From context
step :log, LogStep do
  argument :correlation_id, context(:correlation_id)
end

# Literal values
step :notify, NotifyStep do
  argument :priority, literal(:high)
end

# Computed values
step :compute, ComputeStep do
  argument :value, fn reactor ->
    reactor.inputs[:base] * 2
  end
end
```

#### 3. Conditions

Steps can have conditions that determine if they run:

```elixir
step :send_email, SendEmail do
  argument :user, step(:fetch_user)

  # Only run if user is confirmed
  condition fn reactor ->
    reactor.results[:fetch_user].confirmed?
  end
end

step :premium_welcome, PremiumWelcome do
  argument :user, step(:fetch_user)

  # Only run for premium users
  condition step(:fetch_user), :premium?
end
```

#### 4. Error Handling

Reactor provides robust error handling:

```elixir
defmodule MyApp.PaymentReactor do
  use Reactor

  step :charge_card, ChargeCard do
    argument :amount, input(:amount)
    argument :card, input(:card)

    # Retry on failure
    retry max_attempts: 3, delay: 1000
  end

  step :refund, Refund do
    argument :charge_id, step(:charge_card)

    # Only run if charge_card failed (compensation)
    only_on_failure :charge_card
  end

  # Custom error handler
  on_error fn reactor, error ->
    # Log error, send notification, etc.
    MyApp.ErrorLogger.log(reactor, error)
  end
end
```

#### 5. Compensation (Rollback)

Reactor implements the Saga pattern with compensation:

```elixir
defmodule MyApp.BookTrip do
  use Reactor

  step :reserve_flight, ReserveFlight do
    argument :flight, input(:flight)
    argument :passenger, input(:passenger)
  end

  step :reserve_hotel, ReserveHotel do
    argument :hotel, input(:hotel)
    argument :guest, input(:passenger)
  end

  step :book_car, BookCar do
    argument :car_type, input(:car_type)
    argument :driver, input(:passenger)
  end

  # Compensation runs if any step fails
  compensate fn reactor ->
    # Undo in reverse order
    if reactor.results[:book_car], do: cancel_car(reactor.results[:book_car])
    if reactor.results[:reserve_hotel], do: cancel_hotel(reactor.results[:reserve_hotel])
    if reactor.results[:reserve_flight], do: cancel_flight(reactor.results[:reserve_flight])
  end
end
```

---

### Reactor API

#### Running Reactors

```elixir
# Basic run
{:ok, results} = Reactor.run(MyReactor, %{input: value})

# With context
{:ok, results} = Reactor.run(
  MyReactor,
  %{input: value},
  %{correlation_id: "abc-123"}
)

# With options
{:ok, results} = Reactor.run(
  MyReactor,
  inputs,
  context,
  max_concurrency: 10,
  timeout: :timer.minutes(5),
  async?: true
)

# Synchronous (no parallel execution)
{:ok, results} = Reactor.run(
  MyReactor,
  inputs,
  context,
  async?: false
)
```

#### Building Reactors Programmatically

```elixir
alias Reactor.Builder

reactor =
  Builder.new()
  |> Builder.add_input(:user_id)
  |> Builder.add_input(:options, default: %{})
  |> Builder.add_step(:fetch_user, GetUser, user_id: {:input, :user_id})
  |> Builder.add_step(:validate, ValidateUser, user: {:step, :fetch_user})
  |> Builder.add_step(:notify, NotifyUser,
    user: {:step, :fetch_user},
    options: {:input, :options}
  )
  |> Builder.return(:notify)

{:ok, results} = Reactor.run(reactor, %{user_id: "123"})
```

#### Dynamic Step Addition

```elixir
defmodule DynamicReactor do
  use Reactor

  step :analyze, AnalyzeData do
    argument :data, input(:data)
  end

  # Add steps based on analysis results
  step :route, RouteStep do
    argument :analysis, step(:analyze)

    after_run fn reactor, _result ->
      case reactor.results[:analysis].type do
        :type_a ->
          Builder.add_step(reactor, :process_a, ProcessA, data: {:step, :analyze})
        :type_b ->
          Builder.add_step(reactor, :process_b, ProcessB, data: {:step, :analyze})
        _ ->
          reactor
      end
    end
  end
end
```

---

### Reactor DSL

The Reactor DSL provides a declarative way to define workflows:

```elixir
defmodule MyWorkflow do
  use Reactor

  # Inputs with validation
  input :data, required: true
  input :options, default: %{}

  # Step configuration
  step :process, ProcessStep do
    argument :data, input(:data)
    timeout :timer.minutes(2)
    retry max_attempts: 3, delay: 1000
    condition fn reactor -> not reactor.inputs[:dry_run] end
  end

  # Parallel steps
  step :notify_email, NotifyEmail do
    argument :result, step(:process)
  end

  step :notify_slack, NotifySlack do
    argument :result, step(:process)
  end

  # Sequential step (waits for both notifications)
  step :log_completion, LogCompletion do
    argument :email_result, step(:notify_email)
    argument :slack_result, step(:notify_slack)
  end

  # Return specific step results
  return [:process, :log_completion]
end
```

---

### Concurrency Model

Reactor uses a DAG (Directed Acyclic Graph) to determine execution order:

```
         ┌─────────────┐
         │   Input     │
         └──────┬──────┘
                │
         ┌──────▼──────┐
         │   Step A    │
         └──────┬──────┘
                │
       ┌────────┴────────┐
       │                 │
┌──────▼──────┐   ┌──────▼──────┐
│   Step B    │   │   Step C    │  (Run in parallel)
└──────┬──────┘   └──────┬──────┘
       │                 │
       └────────┬────────┘
                │
         ┌──────▼──────┐
         │   Step D    │  (Waits for B and C)
         └──────┬──────┘
                │
         ┌──────▼──────┐
         │   Return    │
         └─────────────┘
```

#### Concurrency Control

```elixir
# Limit concurrent steps
Reactor.run(reactor, inputs, context,
  max_concurrency: 5  # Max 5 steps running in parallel
)

# Timeout entire reactor
Reactor.run(reactor, inputs, context,
  timeout: :timer.minutes(10)  # Fail after 10 minutes
)

# Limit iterations (for loops)
Reactor.run(reactor, inputs, context,
  max_iterations: 100  # Fail after 100 iterations
)
```

---

### Monitoring and Observability

#### Telemetry Events

Reactor emits telemetry events:

```elixir
# Attach handler
:telemetry.attach(
  "my-reactor-handler",
  [:reactor, :step, :stop],
  &MyApp.Metrics.handle_step/3,
  nil
)

# Handler function
def handle_step([:reactor, :step, :stop], measurements, metadata, _) do
  # measurements.duration is in native time
  # metadata contains step_name, reactor_id, etc.
end
```

#### Context Propagation

```elixir
# Context is passed through all steps
Reactor.run(reactor, inputs, %{
  correlation_id: "abc-123",
  tracer: MyApp.Tracer
})

# Access context in steps
defmodule MyStep do
  def run(arguments, context) do
    correlation_id = context.correlation_id
    # Use for tracing, logging, etc.
  end
end
```

---

### Testing Reactors

```elixir
defmodule MyWorkflowTest do
  use ExUnit.Case

  test "runs successfully" do
    inputs = %{data: "test", options: %{}}
    context = %{}

    assert {:ok, results} = Reactor.run(MyWorkflow, inputs, context)
    assert Map.has_key?(results, :process)
  end

  test "compensates on failure" do
    # Mock step to fail
    Mox.expect(MyApp.MockStep, :run, fn _, _ -> {:error, :test_error} end)

    assert {:error, _} = Reactor.run(MyWorkflow, %{data: "fail"}, %{})

    # Verify compensation was called
    assert_called(MyApp.MockCompensation, :compensate, 1)
  end

  test "respects conditions" do
    inputs = %{data: "test", dry_run: true}

    assert {:ok, _} = Reactor.run(MyWorkflow, inputs, %{})

    # Step with condition should not run
    refute_called(MyApp.MockConditionalStep, :run, 1)
  end
end
```

---

## Part 2: ReactorFile (`reactor_file`)

### Overview

ReactorFile allows defining reactors in YAML/JSON files instead of code.

### Example Configuration

```yaml
# workflows/welcome_user.yaml
name: WelcomeUser
inputs:
  - user_id
  - send_email:
      default: true

steps:
  - name: fetch_user
    module: GetUser
    arguments:
      user_id: "{{inputs.user_id}}"

  - name: validate_user
    module: ValidateUser
    arguments:
      user: "{{steps.fetch_user}}"
    depends_on:
      - fetch_user

  - name: send_welcome_email
    module: SendEmail
    arguments:
      user: "{{steps.fetch_user}}"
    condition: "{{inputs.send_email}}"
    depends_on:
      - fetch_user

return:
  - fetch_user
  - send_welcome_email
```

### Loading File-Based Reactors

```elixir
# Load reactor from file
{:ok, reactor} = ReactorFile.load("workflows/welcome_user.yaml")

# Run the reactor
{:ok, results} = Reactor.run(reactor, %{user_id: "123"})
```

---

## Part 3: Splode (`splode`)

### Overview

Splode provides utilities for error handling, particularly for aggregating and formatting errors.

### Key Features

#### 1. Error Aggregation

```elixir
defmodule MyApp.Error do
  use Splode.Error

  defexception [:errors]

  def message(exception) do
    "Multiple errors occurred: #{inspect(exception.errors)}"
  end
end

# Aggregate errors
errors = [
  {:error, ErrorOne.new(message: "First error")},
  {:error, ErrorTwo.new(message: "Second error")}
]

# Combine into single error
combined_error = Splode.Error.combine(errors)
```

#### 2. Error Classes

```elixir
# Ash uses Splode for error classification
defmodule Ash.Error.Invalid do
  use Splode.Error

  defexception [:field, :message]

  def exception(opts) do
    %__MODULE__{field: opts[:field], message: opts[:message]}
  end
end
```

#### 3. Error Formatting

```elixir
# Format errors for display
Splode.Error.format(error)
# => "Field 'email' is invalid: must be a valid email address"

# Get short message
Splode.Error.short_message(error)
# => "Invalid email"
```

---

## Real-World Example: E-Commerce Order Processing

```elixir
defmodule MyApp.Ecommerce.ProcessOrder do
  use Reactor

  input :order_id
  input :payment_token
  input :notify_customer, default: true

  # Step 1: Fetch order
  step :fetch_order, FetchOrder do
    argument :order_id, input(:order_id)
  end

  # Step 2: Validate inventory (parallel with payment)
  step :check_inventory, CheckInventory do
    argument :items, step(:fetch_order), :items
  end

  # Step 3: Process payment (parallel with inventory)
  step :process_payment, ProcessPayment do
    argument :amount, step(:fetch_order), :total
    argument :token, input(:payment_token)
    argument :customer, step(:fetch_order), :customer

    retry max_attempts: 3, delay: 1000
  end

  # Step 4: Reserve inventory (depends on check and payment)
  step :reserve_inventory, ReserveInventory do
    argument :items, step(:check_inventory)
    argument :payment, step(:process_payment)
  end

  # Step 5: Create shipment
  step :create_shipment, CreateShipment do
    argument :order, step(:fetch_order)
    argument :reservation, step(:reserve_inventory)
  end

  # Step 6: Send confirmation (conditional)
  step :send_confirmation, SendConfirmation do
    argument :order, step(:fetch_order)
    argument :shipment, step(:create_shipment)
    condition input(:notify_customer)
  end

  # Compensation on failure
  compensate fn reactor ->
    # Reverse operations in opposite order
    if reactor.results[:create_shipment] do
      cancel_shipment(reactor.results[:create_shipment])
    end

    if reactor.results[:reserve_inventory] do
      release_inventory(reactor.results[:reserve_inventory])
    end

    if reactor.results[:process_payment] do
      refund_payment(reactor.results[:process_payment])
    end
  end

  # Error handling
  on_error fn reactor, error ->
    # Log error with full context
    Logger.error("Order processing failed",
      order_id: reactor.inputs[:order_id],
      error: inspect(error),
      completed_steps: Map.keys(reactor.results)
    )

    # Notify operations team
    MyApp.OpsAlert.send(:order_processing_failed, reactor, error)
  end

  # Return key results
  return [:fetch_order, :process_payment, :create_shipment, :send_confirmation]
end

# Usage
{:ok, results} = Reactor.run(
  MyApp.Ecommerce.ProcessOrder,
  %{
    order_id: "order-123",
    payment_token: "tok_visa_4242"
  },
  %{correlation_id: "corr-abc-123"}
)
```

---

## Reactor vs Other Workflow Systems

| Feature | Reactor | Temporal | AWS Step Functions |
|---------|---------|----------|-------------------|
| Language | Elixir | Multi-lang | Multi-lang |
| Deployment | In-process | Service | Cloud |
| State Persistence | Memory (ephemeral) | Database | AWS |
| Concurrency | Native BEAM | Limited | Limited |
| Cost | Free | Service cost | Pay-per-use |
| Learning Curve | Low | High | Medium |

---

## Best Practices

### 1. Keep Steps Atomic

```elixir
# Good: Single responsibility
step :validate_email, ValidateEmail do
  argument :email, input(:email)
end

step :send_verification, SendVerification do
  argument :email, step(:validate_email)
end

# Avoid: Multiple responsibilities
step :validate_and_send, ValidateAndSend do
  # Don't do this
end
```

### 2. Use Conditions for Optional Steps

```elixir
step :send_sms, SendSms do
  argument :phone, step(:fetch_user)
  condition fn reactor ->
    not is_nil(reactor.results[:fetch_user].phone)
  end
end
```

### 3. Handle Failures Gracefully

```elixir
step :charge_card, ChargeCard do
  retry max_attempts: 3, delay: fn attempt -> attempt * 1000 end
end

step :fallback_payment, FallbackPayment do
  only_on_failure :charge_card
end
```

### 4. Use Context for Cross-Cutting Concerns

```elixir
# Pass correlation ID through context
Reactor.run(reactor, inputs, %{
  correlation_id: Correlation.id(),
  tracer: OpenTelemetry.get_tracer()
})
```

### 5. Test Compensation Logic

```elixir
test "compensates on payment failure" do
  Mox.stub(MockPayment, :charge, fn _ -> {:error, :declined} end)

  assert {:error, _} = Reactor.run(ProcessOrder, valid_inputs(), %{})

  assert_called(MockCompensation, :release_inventory, 1)
end
```

---

## Conclusion

Reactor provides:

1. **Declarative Workflows**: Define complex processes with simple DSL
2. **Automatic Concurrency**: Independent steps run in parallel
3. **Saga Pattern**: Built-in compensation for rollbacks
4. **Dynamic Execution**: Build workflows at runtime
5. **Error Handling**: Robust error handling and retry logic
6. **Observability**: Telemetry integration for monitoring

The Reactor ecosystem (Reactor + ReactorFile + Splode) provides a complete solution for orchestrating complex, multi-step processes in Elixir applications.
