# 11 - Scheduled Jobs (Cron)

## Jobs Configurados

### 1. Expirar Redenciones (Cada hora)
```
Cron: 0 0 * * * *
Función: expire_old_redemptions()
```

### 2. Limpieza de QR Codes (Diario 3 AM)
```
Cron: 0 0 3 * * *
Función: cleanup_old_qr_codes()
```

### 3. Recalcular Stats Merchants (Diario 4 AM)
```
Cron: 0 0 4 * * *
Función: recalculate_merchant_stats()
```

### 4. Alertas de Expiración (Cada 5 min)
```
Cron: 0 */5 * * * *
Función: send_expiration_alerts()
```

## Dependencia

```toml
tokio-cron-scheduler = "0.10"
```

**Siguiente**: [12-deployment.md](./12-deployment.md)
