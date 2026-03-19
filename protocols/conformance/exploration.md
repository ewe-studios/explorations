---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.protocols/conformance
repository: https://github.com/Universal-Commerce-Protocol/conformance
explored_at: 2026-03-20T00:00:00Z
language: Python
---

# Project Exploration: UCP Conformance Tests

## Overview

UCP Conformance Tests is a comprehensive test suite for validating UCP (Universal Commerce Protocol) implementations. It ensures that different UCP implementations interoperate correctly and adhere to the specification.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.protocols/conformance`
- **Remote:** `git@github.com:Universal-Commerce-Protocol/conformance.git`
- **Primary Language:** Python
- **License:** Apache License 2.0

## Directory Structure

```
conformance/
├── business_logic_test.py       # Business logic tests
├── checkout_flow_test.py        # Checkout flow tests
├── idempotency_test.py          # Idempotency tests
├── fulfillment_test.py          # Fulfillment tests
├── order_test.py                # Order lifecycle tests
├── card_credential_test.py      # Credential tests
├── ap2_test.py                  # AP2 authentication tests
├── binding_test.py              # Transport binding tests
│
├── fixtures/                    # Test fixtures
│   ├── profiles/                # UCP profile fixtures
│   ├── schemas/                 # Schema fixtures
│   └── mocks/                   # Mock server fixtures
│
├── utils/                       # Test utilities
│   ├── client.py                # Test client
│   ├── assertions.py            # Custom assertions
│   └── reporters.py             # Test reporters
│
├── .cspell/                     # Spell check config
├── .cspell.json
├── .github/
├── .gitignore
├── LICENSE
└── README.md
```

## Test Categories

### Business Logic Tests

Tests core UCP business logic:

```python
def test_checkout_flow():
    """Test complete checkout flow"""
    session = client.create_checkout()
    session = client.add_item(session.id, "product-123", 2)
    session = client.apply_discount(session.id, "SAVE20")
    confirmation = client.complete(session.id, payment_method)
    assert confirmation.status == "confirmed"
    assert confirmation.total > 0
```

### Idempotency Tests

Tests idempotency guarantees:

```python
def test_idempotent_create():
    """Test that duplicate creates return same result"""
    idempotency_key = str(uuid4())

    result1 = client.create_checkout(
        idempotency_key=idempotency_key
    )
    result2 = client.create_checkout(
        idempotency_key=idempotency_key
    )

    assert result1.id == result2.id
    assert result1.created_at == result2.created_at
```

### Fulfillment Tests

Tests fulfillment capability:

```python
def test_fulfillment_options():
    """Test fulfillment option retrieval"""
    options = client.get_fulfillment_options(session_id)
    assert len(options) > 0

    selected = client.select_fulfillment(
        session_id, options[0].id
    )
    assert selected.fulfillment_option is not None
```

### AP2 Tests

Tests AP2 authentication:

```python
def test_ap2_handshake():
    """Test AP2 authentication handshake"""
    challenge = server.get_challenge()
    response = client.sign_challenge(challenge)
    token = server.verify_challenge(response)
    assert token is not None
```

## Running Tests

### Prerequisites

```bash
pip install pytest pytest-asyncio httpx
```

### Test Commands

```bash
# Run all tests
pytest

# Run specific test file
pytest checkout_flow_test.py -v

# Run with coverage
pytest --cov=.

# Run against specific implementation
UCP_TEST_BASE_URL=https://store.example.com pytest
```

## Test Structure

```python
class TestCheckoutFlow:
    """Checkout capability tests"""

    @pytest.fixture
    def client(self):
        return UCPTestClient(BASE_URL)

    def test_create_session(self, client):
        session = client.create_checkout()
        assert session.id is not None
        assert session.status == "active"

    def test_add_item(self, client):
        session = client.create_checkout()
        updated = client.add_item(session.id, "item-1", 1)
        assert len(updated.items) == 1
        assert updated.items[0].quantity == 1

    def test_apply_discount(self, client):
        session = client.create_checkout()
        updated = client.apply_discount(session.id, "VALID_CODE")
        assert updated.discount is not None
        assert updated.total < updated.subtotal

    def test_complete_checkout(self, client):
        session = client.create_checkout()
        client.add_item(session.id, "item-1", 1)
        confirmation = client.complete(session.id, TEST_CARD)
        assert confirmation.status == "confirmed"
        assert confirmation.order_number is not None
```

## Integration with CI

```yaml
# .github/workflows/conformance.yml
name: Conformance Tests

on: [push, pull_request]

jobs:
  conformance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - name: Install dependencies
        run: pip install -r requirements.txt
      - name: Run tests
        run: pytest --junitxml=report.xml
      - name: Upload report
        uses: actions/upload-artifact@v4
        with:
          name: conformance-report
          path: report.xml
```

## Key Insights

1. **Implementation Agnostic:** Tests work against any UCP-compliant implementation
2. **Contract Testing:** Ensures different implementations interoperate
3. **Regression Prevention:** Catches breaking changes early
4. **Documentation:** Tests serve as executable specification

## Open Questions

1. **Certification:** Is there a certification program for passing implementations?
2. **Versioning:** How are test suites versioned with protocol?
3. **Performance:** Should there be performance benchmarks?
