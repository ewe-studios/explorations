# PyCap'n Proto (pycapnp) Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/pycapnp
repository: https://github.com/capnproto/python-capnp
explored_at: 2026-03-23

## Overview

PyCap'n Proto provides Python bindings for the Cap'n Proto C++ library. It offers both synchronous and asynchronous APIs for serialization and RPC.

## Project Structure

```
pycapnp/
├── capnp/
│   ├── __init__.py        # Package initialization
│   ├── _gen.py            # Code generation utilities
│   ├── lib/
│   │   └── capnp.pyx      # Cython bindings (main implementation)
│   ├── helpers/
│   │   ├── capabilityHelper.cpp
│   │   ├── rpcHelper.h
│   │   └── serialize.h
│   ├── includes/
│   │   ├── capnp_cpp.pxd  # Cython declarations
│   │   ├── schema_cpp.pxd
│   │   └── PyCustomMessageBuilder.h
│   └── templates/
│       └── module.pyx.tmpl
├── benchmark/
│   ├── addressbook.capnp
│   ├── carsales.capnp
│   ├── catrank.capnp
│   └── eval.capnp
├── examples/
│   ├── addressbook.py
│   ├── async_calculator_server.py
│   ├── async_calculator_client.py
│   └── calculator.capnp
├── test/                  # Test suite
├── buildutils/            # Build utilities
├── setup.py
├── pyproject.toml
└── CHANGELOG.md
```

## Installation

```bash
pip install pycapnp
```

Builds bundled libcapnp C++ library.

## Basic Usage

### Loading Schemas

```python
import capnp

# Load schema file
capnp.load_schema('schema.capnp')

# Import generated module
import schema_capnp as schema
```

### Creating Messages

```python
# Create new message
message = capnp._capnp._Message()

# Init root struct
point = message.init_root_as('Point')
point.x = 1.0
point.y = 2.0

# Or using context manager
with capnp._capnp._Message() as message:
    point = message.init_root_as('Point')
    point.x = 1.0
```

### Reading Messages

```python
# Deserialize from file
with open('message.bin', 'rb') as f:
    message = capnp.load(f, schema.Point)
    print(message.x, message.y)

# Deserialize from bytes
message = capnp.loads(data, schema.Point)
```

## Schema Types

### Structs

```capnp
struct Point {
    x @0 :Float32;
    y @1 :Float32;
}
```

```python
# Builder
point = message.init_root_as('Point')
point.x = 1.0
point.y = 2.0

# Reader
x = point.x
y = point.y
```

### Enums

```capnp
enum Color {
    red @0;
    green @1;
    blue @2;
}
```

```python
# Set enum
point.color = 'red'
# or
point.color = 0

# Read enum
color = point.color  # Returns string
```

### Unions

```capnp
struct Shape {
    union {
        rectangle @0 :Rectangle;
        circle @1 :Circle;
    }
}
```

```python
# Set union member
shape.rectangle.width = 10
shape.rectangle.height = 20

# Check which is set
if shape.which() == 'rectangle':
    print(shape.rectangle.width)
```

### Lists

```capnp
struct AddressBook {
    people @0 :List(Person);
}
```

```python
# Create list
book = message.init_root_as('AddressBook')
people = book.init_people(3)

# Set list elements
people[0].name = 'Alice'
people[1].name = 'Bob'

# Iterate
for person in people:
    print(person.name)
```

### Text and Data

```capnp
struct Record {
    name @0 :Text;
    data @1 :Data;
}
```

```python
# Text (UTF-8 strings)
record.name = 'Hello'

# Data (bytes)
record.data = b'\x00\x01\x02\x03'
```

## RPC System

### Async RPC (asyncio)

```python
import capnp
import asyncio

# Load schema with RPC
calculator_capnp = capnp.load('calculator.capnp')

async def main():
    # Connect to server
    client = await capnp.connect('localhost:5000', calculator_capnp.Calculator)

    # Make RPC call
    op = await client.op(operator=calculator_capnp.Operator.new_add())
    result = await op.evaluate(x=10.0)

    print(f"Result: {result.result}")

asyncio.run(main())
```

### Server Implementation

```python
class CalculatorServer(calculator_capnp.Calculator.Server):
    async def op(self, operator, **kwargs):
        impl = OperatorImpl(operator.operator)
        return impl

class OperatorImpl(calculator_capnp.Operation.Server):
    def __init__(self, operator):
        self.operator = operator

    async def evaluate(self, x, **kwargs):
        if self.operator == 'add':
            return x + 5
        # ...
```

### Two-Party RPC

```python
# Server
import capnp

async def run_server():
    server = CalculatorServer()
    await capnp.listen_forever('localhost:5000', server)

# Client
async def run_client():
    client = await capnp.connect('localhost:5000', Calculator)
```

## Advanced Features

### Custom Message Builders

```python
class MyMessageBuilder(capnp._capnp._PyCustomMessageBuilder):
    def build(self, schema):
        # Custom initialization
        pass

# Use custom builder
message = capnp._capnp._Message(builder=MyMessageBuilder())
```

### Serialization Options

```python
# With options
message = capnp._capnp._Message(
    traversal_limit_in_words=8 * 1024 * 1024,  # 8 MB
    nesting_limit=64,
)
```

### Packed Serialization

```python
# Packed (compressed) format
packed_data = capnp.pack(data)
unpacked_data = capnp.unpack(packed_data)

# Save packed to file
with open('message.packed', 'rb') as f:
    capnp.load_packed(f, schema.Point)
```

## Benchmark Suite

### Comparison Benchmarks

```bash
# Run benchmarks
cd benchmark/bin
./run_all.py
```

Compares:
- Cap'n Proto (pycapnp)
- Protocol Buffers (protobuf)
- Protobuf with C++ extension

### Benchmark Schemas

- **carsales**: Complex nested structures
- **catrank**: Text processing
- **eval**: Expression trees

## Examples

### Address Book

```python
# examples/addressbook.py
import capnp
import addressbook_capnp

def create_address_book():
    message = capnp._capnp._Message()
    book = message.init_root_as(addressbook_capnp.AddressBook)

    people = book.init_people(2)

    alice = people[0]
    alice.id = 1
    alice.name = 'Alice'
    alice.email = 'alice@example.com'

    return message

# Save to file
with open('addressbook.bin', 'wb') as f:
    message.write(f)
```

### Async Calculator

```python
# examples/async_calculator_server.py
class Calculator(calculator_capnp.Calculator.Server):
    async def op(self, operator, **kwargs):
        return Operator(self, operator.operator)

class Operator(calculator_capnp.Operation.Server):
    async def evaluate(self, x, **kwargs):
        # Perform calculation
        return x

async def main():
    server = Calculator()
    await capnp.listen_forever('localhost:5000', server)
```

### SSL/TLS

```python
# examples/async_ssl_server.py
import ssl

ctx = ssl.create_default_context(ssl.Purpose.CLIENT_AUTH)
ctx.load_cert_chain('server.crt', 'server.key')

await capnp.listen_forever(
    'localhost:5000',
    server,
    ssl=ctx
)
```

## Cython Bindings

### capnp.pyx Structure

```cython
# cython: language_level=3
import cython
from libcpp cimport bool
from libc.stdint cimport uint32_t, uint64_t

cdef extern from "<capnp/message.h>" namespace "capnp":
    cdef cppclass Message:
        Message()
        void* getRoot()

cdef class Message:
    cdef capnp.Message* thisptr

    def __init__(self):
        self.thisptr = new capnp.Message()

    def init_root_as(self, schema):
        # Initialize root struct
        pass
```

### C++ Helpers

```cpp
// capabilityHelper.h
#pragma once
#include <capnp/capability.h>

// Helper functions for capability handling
capnp::Response<Calculator::EvaluateResults>
evaluate_impl(double x);
```

## Testing

### Unit Tests

```python
# test/test_struct.py
def test_basic_struct():
    message = capnp._capnp._Message()
    point = message.init_root_as('Point')

    point.x = 1.0
    point.y = 2.0

    assert point.x == 1.0
    assert point.y == 2.0
```

### RPC Tests

```python
# test/test_rpc.py
async def test_rpc_call():
    server = CalculatorServer()
    client = await capnp.connect('localhost:5000', Calculator)

    result = await client.op(operator='add')
    assert result is not None
```

## Build System

### setup.py

```python
from setuptools import setup
from Cython.Build import cythonize

# Bundled libcapnp
capnp_prefix = 'bundled/libcapnp'

setup(
    name='pycapnp',
    ext_modules=cythonize('capnp/lib/capnp.pyx'),
    include_dirs=[capnp_prefix + '/src'],
    libraries=['capnp'],
)
```

### Build Requirements

- Python 3.8+
- Cython
- C++17 compiler
- libcapnp (bundled)

## Performance

### Zero-Copy Reads

```python
# Reading doesn't copy data
with open('large_message.bin', 'rb') as f:
    message = capnp.load(f)
    # Access fields without deserialization
    value = message.large_field
```

### Memory Mapping

```python
# Memory-map large files
import mmap

with open('data.bin', 'rb') as f:
    mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
    message = capnp.load(mm)
    # Zero-copy access to file contents
```

## Comparison with Other Python Serialization

| Feature | pycapnp | protobuf | msgpack |
|---------|---------|----------|---------|
| Schema | .capnp | .proto | None |
| Zero-copy | Yes | No | No |
| RPC | Yes | Yes | No |
| Async | Yes | Yes | N/A |
| Speed | Fast | Medium | Fast |

## Known Issues

1. **Windows support**: Limited
2. **Python 3.12+**: May require updates
3. **Memory leaks**: Some edge cases in async RPC

## Resources

- [PyPI Package](https://pypi.org/project/pycapnp/)
- [Documentation](https://pycapnp.readthedocs.io/)
- [GitHub Repository](https://github.com/capnproto/python-capnp)
