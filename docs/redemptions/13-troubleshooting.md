# 13 - Troubleshooting

## Problema: JWT Inválido

**Síntoma**: 401 Unauthorized  
**Causa**: Token expirado o JWT_SECRET incorrecto  
**Solución**: Regenerar token o verificar JWT_SECRET

## Problema: Balance no se actualiza

**Síntoma**: Balance incorrecto después de redención  
**Causa**: Trigger no ejecutándose  
**Solución**: Verificar triggers en DB

```sql
SELECT * FROM pg_trigger WHERE tgname LIKE '%balance%';
```

## Problema: Webhook no llega

**Síntoma**: Merchant no recibe notificaciones  
**Causa**: webhook_enabled = false o URL incorrecta  
**Solución**: 

```sql
SELECT webhook_url, webhook_enabled 
FROM rewards.merchants 
WHERE merchant_id = 'uuid';
```

## Logs

```bash
# Ver logs en tiempo real
tail -f /var/log/lumis/app.log

# Filtrar errores
grep ERROR /var/log/lumis/app.log
```

**Siguiente**: [14-testing.md](./14-testing.md)
