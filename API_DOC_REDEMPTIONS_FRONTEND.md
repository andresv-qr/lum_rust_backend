# Documentación de Integración Frontend - Sistema de Redención Lümis

Esta guía detalla los endpoints y flujos necesarios para integrar el sistema de redención de Lümis en la aplicación móvil/web del usuario ("El Cliente").

## 🔐 Autenticación
Todos los endpoints requieren el header `Authorization: Bearer <JWT_TOKEN>` del usuario logueado.

---

## 1. Catálogo de Ofertas
Muestra las ofertas disponibles para que el usuario pueda canjear sus Lümis.

### Listar Ofertas
`GET /api/v1/rewards/offers`

**Parámetros (Query Params):**
- `category`: (Opcional) Filtrar por categoría (ej. "Alimentos", "Tecnología").
- `sort`: (Opcional) Ordenar por: `cost_asc`, `cost_desc`, `newest`.
- `limit`: (Opcional) Cantidad de resultados (default: 50).
- `offset`: (Opcional) Paginación.

**Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "total_count": 15,
  "offers": [
    {
      "offer_id": "b1ffcd00-0d1c-5ff9-cc7e-7cc0ce491b22",
      "name_friendly": "Café Gratis",
      "description_friendly": "Disfruta de un delicioso café americano de 12oz.",
      "lumis_cost": 10,
      "category": "Alimentos",
      "merchant_name": "Demo Store",
      "image_url": "https://placehold.co/400x400/6B46C1/white?text=Cafe",
      "is_available": true,
      "stock_remaining": 998,
      "max_redemptions_per_user": 5,
      "user_redemptions_count": 0,
      "expires_at": "2026-12-11T10:00:00Z"
    }
  ]
}
```

### Detalle de Oferta
`GET /api/v1/rewards/offers/:id`

**Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "offer": {
    "offer_id": "b1ffcd00-0d1c-5ff9-cc7e-7cc0ce491b22",
    "name_friendly": "Café Gratis",
    "description_friendly": "Disfruta de un delicioso café americano de 12oz.",
    "lumis_cost": 10,
    "terms_and_conditions": "Válido solo en sucursales participantes. No acumulable.",
    "merchant_name": "Demo Store",
    "valid_from": "2024-01-01T00:00:00Z",
    "valid_to": "2025-12-31T23:59:59Z",
    "is_available": true
  }
}
```

---

## 2. Realizar Redención (Canje)
Cuando el usuario presiona el botón "Redimir" o "Canjear".

`POST /api/v1/rewards/redeem`

**Body (JSON):**
```json
{
  "offer_id": "b1ffcd00-0d1c-5ff9-cc7e-7cc0ce491b22"
}
```

**Respuesta Exitosa (201 Created):**
```json
{
  "success": true,
  "redemption": {
    "redemption_id": "550e8400-e29b-41d4-a716-446655440000",
    "redemption_code": "LUMS-A1B2-C3D4",
    "qr_image_url": "https://api.lumapp.org/static/qr/LUMS-A1B2-C3D4.png",
    "qr_landing_url": "https://comercios.lumapp.org/validate/LUMS-A1B2-C3D4",
    "expires_at": "2025-12-11T10:15:00Z",
    "lumis_spent": 10,
    "new_balance": 4990
  }
}
```

**Errores Comunes:**
- `400 Bad Request`: "Saldo insuficiente" o "Stock agotado".
- `429 Too Many Requests`: "Límite de redenciones alcanzado".

---

## 3. Historial y "Mis Cupones"
Muestra los cupones activos (pendientes) y el historial de canjes pasados.

### Listar Redenciones
`GET /api/v1/rewards/history`

**Parámetros (Query Params):**
- `status`: (Opcional) `pending` (activos), `confirmed` (usados), `cancelled`, `expired`.
- `limit`: (Opcional) Default 50.

**Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "total_count": 5,
  "stats": {
    "total_redeemed": 5,
    "total_spent": 50
  },
  "redemptions": [
    {
      "redemption_id": "550e8400-e29b-41d4-a716-446655440000",
      "offer_name": "Café Gratis",
      "merchant_name": "Demo Store",
      "lumis_spent": 10,
      "redemption_code": "LUMS-A1B2-C3D4", // Solo visible si status=pending
      "qr_landing_url": "...", // Solo visible si status=pending
      "redemption_status": "pending",
      "code_expires_at": "2025-12-11T10:15:00Z",
      "created_at": "2025-12-11T10:00:00Z",
      "qr_visible": true,
      "status_message": "Listo para usar"
    }
  ]
}
```

### Detalle de Redención (Pantalla del QR)
`GET /api/v1/rewards/history/:id`

**Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "redemption": {
    "redemption_id": "...",
    "offer_name": "Café Gratis",
    "redemption_code": "LUMS-A1B2-C3D4",
    "qr_landing_url": "...",
    "redemption_status": "pending",
    "qr_visible": true,
    // ... resto de campos
  }
}
```

### Cancelar Redención (Reembolso)
Permite al usuario arrepentirse si no ha usado el cupón (solo si está `pending`).

`DELETE /api/v1/rewards/history/:id`

**Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "message": "Redención cancelada exitosamente",
  "lumis_refunded": 10
}
```

---

## 4. Estadísticas del Usuario
`GET /api/v1/rewards/stats`

**Respuesta Exitosa (200 OK):**
```json
{
  "success": true,
  "stats": {
    "current_balance": 4990,
    "total_redemptions": 5,
    "total_lumis_spent": 50,
    "favorite_category": "Alimentos"
  }
}
```

---

## 📱 Flujo Recomendado en App (UX)

1.  **Catálogo**: Mostrar lista de ofertas (`GET /offers`).
2.  **Detalle**: Al tocar una oferta, mostrar detalle y botón "Canjear por X Lumis".
3.  **Confirmación**: Al confirmar, llamar a `POST /redeem`.
4.  **Éxito**: Mostrar pantalla de "¡Canje Exitoso!" con el QR generado.
5.  **Pantalla QR (Mi Cupón)**:
    *   Mostrar imagen del QR (usar librería de QR nativa o cargar imagen desde URL).
    *   **IMPORTANTE**: Mostrar el código `LUMS-XXXX` en texto grande debajo del QR.
    *   Instrucción: "Muestra este código al cajero".
6.  **Validación**: El usuario presenta el QR. El comercio lo escanea.
7.  **Post-Validación**: Si el usuario refresca la pantalla, el estado cambiará a `confirmed` y el QR desaparecerá.

## ⚠️ Notas de Seguridad
- Las URLs de los QRs (`qr_image_url`) tienen tiempo de vida limitado y validación de estado. Si el cupón ya fue usado, la URL devolverá 404.
- Se recomienda implementar "Pull to Refresh" en la pantalla de detalle del cupón para actualizar el estado si el comercio ya lo validó.
