# 14 - Testing

## Unit Tests

```bash
cargo test
```

## Integration Tests

```bash
cargo test --test redemption_system_tests
```

## Load Testing (k6)

```javascript
import http from 'k6/http';

export default function() {
  http.post('http://localhost:8000/api/v1/rewards/redeem', 
    JSON.stringify({
      offer_id: 'uuid',
      user_id: 12345
    }), 
    { headers: { 'Content-Type': 'application/json' }}
  );
}
```

## Coverage

```bash
cargo tarpaulin --out Html
```

**Siguiente**: [15-contributing.md](./15-contributing.md)
