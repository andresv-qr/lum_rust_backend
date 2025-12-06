# 10 - Monitoreo con Prometheus

## Endpoint de Métricas

`GET /monitoring/metrics`

Formato: Prometheus text format

## Métricas Principales

### HTTP
- `http_requests_total{method,endpoint,status}`
- `http_request_duration_seconds{method,endpoint}`

### Redenciones
- `redemptions_created_total{offer_type,status}`
- `redemptions_confirmed_total{merchant_id,offer_type}`
- `redemptions_expired_total{offer_type}`
- `lumis_spent_total{offer_type}`

### Merchants
- `merchant_logins_total{merchant_id,status}`
- `merchant_validations_total{merchant_id,status}`

### Webhooks y Push
- `webhooks_sent_total{event_type,status}`
- `push_notifications_sent_total{notification_type,status}`

## Configuración Grafana

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'lumis-redemption'
    static_configs:
      - targets: ['localhost:8000']
    metrics_path: '/monitoring/metrics'
    scrape_interval: 15s
```

**Siguiente**: [11-scheduled-jobs.md](./11-scheduled-jobs.md)
