# 02 - Conceptos del Sistema

## ¿Qué son los Lümis?

Los **Lümis** son la moneda virtual del ecosistema Lümis App. Los usuarios acumulan Lümis al registrar facturas de sus compras y pueden canjearlos por productos o servicios en comercios aliados.

**Características**:
- 1 Lümis = 1 punto
- No tienen valor monetario directo
- No son transferibles entre usuarios
- No tienen fecha de expiración
- Se pueden usar para redenciones

## ¿Qué es una Oferta de Redención?

Una **Oferta de Redención** es un producto o servicio que un comercio aliado pone a disposición de los usuarios a cambio de Lümis.

**Ejemplos**:
- Café Americano - 55 Lümis
- Descuento 10% - 100 Lümis
- Producto gratis - 200 Lümis

## ¿Qué es una Redención?

Una **Redención** es la instancia específica cuando un usuario canjea sus Lümis por una oferta.

**Flujo**:
1. Usuario selecciona oferta
2. Sistema valida balance y crea redención
3. Se genera código QR único
4. Usuario presenta código al merchant
5. Merchant valida y confirma
6. Redención completada

## Estados de una Redención

| Estado | Descripción | Puede Confirmar |
|--------|-------------|-----------------|
| `pending` | Recién creada, esperando validación | ✅ Sí |
| `confirmed` | Confirmada por merchant | ❌ No |
| `expired` | Código expiró sin uso | ❌ No |
| `cancelled` | Cancelada por usuario o sistema | ❌ No |

## Códigos de Redención

Formato: `LUMS-XXXX-XXXX-XXXX`

**Características**:
- Único por redención
- Alfanumérico (A-Z, 0-9)
- 19 caracteres totales
- Expira en 15 minutos
- Se genera automáticamente

## QR Codes

Cada redención tiene un código QR que contiene:
- Código de redención
- URL de landing page
- Timestamp de creación

## Merchants (Comercios Aliados)

Los **Merchants** son los comercios que aceptan Lümis como forma de pago.

**Capacidades**:
- Validar códigos de redención
- Confirmar redenciones
- Ver estadísticas
- Recibir webhooks
- Acceder a analytics

**Siguiente**: [03-modelo-datos.md](./03-modelo-datos.md)
