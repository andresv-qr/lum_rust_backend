# 18 - SDKs y Ejemplos

## Python SDK (Conceptual)

```python
from lumis_sdk import LumisClient

client = LumisClient(
    base_url="https://api.lumapp.org",
    token="your-jwt-token"
)

# Get offers
offers = client.rewards.get_offers()

# Create redemption
redemption = client.rewards.redeem(
    offer_id="uuid",
    user_id=12345
)

print(f"Code: {redemption.redemption_code}")
print(f"QR: {redemption.qr_image_url}")
```

## PHP SDK (Conceptual)

```php
<?php
use Lumis\SDK\Client;

$client = new Client([
    'base_url' => 'https://api.lumapp.org',
    'token' => 'your-jwt-token'
]);

$offers = $client->rewards()->getOffers();

$redemption = $client->rewards()->redeem([
    'offer_id' => 'uuid',
    'user_id' => 12345
]);
```

## cURL Examples

```bash
# Get offers
curl -X GET https://api.lumapp.org/api/v1/rewards/offers \
  -H "Authorization: Bearer $TOKEN"

# Create redemption
curl -X POST https://api.lumapp.org/api/v1/rewards/redeem \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"offer_id":"uuid","user_id":12345}'
```

**Fin de documentación**
