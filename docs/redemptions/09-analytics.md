# 09 - Dashboard de Analytics para Merchants

## Endpoint

`GET /api/v1/merchant/analytics?range=week`

## Query Parameters

- `range`: "today" | "week" | "month" | "custom"
- `start_date`: ISO 8601 (para custom)
- `end_date`: ISO 8601 (para custom)

## Respuesta

```json
{
  "summary": {
    "total_redemptions": 150,
    "confirmed_redemptions": 120,
    "pending_redemptions": 20,
    "expired_redemptions": 10,
    "total_lumis": 7500
  },
  "redemptions_by_day": [
    {"date": "2025-10-18", "count": 25, "lumis": 1250}
  ],
  "peak_hours": [
    {"hour": 14, "count": 30},
    {"hour": 18, "count": 45}
  ],
  "popular_offers": [
    {
      "offer_id": "uuid",
      "offer_name": "Café Americano",
      "redemption_count": 80,
      "total_lumis": 4400
    }
  ],
  "average_confirmation_time": 3.5,
  "expiration_rate": 6.67
}
```

**Siguiente**: [10-prometheus-metrics.md](./10-prometheus-metrics.md)
