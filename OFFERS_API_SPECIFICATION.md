# 🔌 Especificación de APIs - Sistema de Ofertas

## 📋 Información General

### Base URL
```
Production: https://api.lumis.com/v1
Staging: https://api-staging.lumis.com/v1
Development: http://localhost:8080/v1
```

### Autenticación
```http
Authorization: Bearer {jwt_token}
X-API-Key: {api_key} # Para comercios
X-Device-ID: {device_id} # Para tracking
```

### Rate Limiting
```yaml
Usuarios:
  - 100 requests/minute
  - 10,000 requests/day

Comercios:
  - 500 requests/minute
  - 100,000 requests/day

Enterprise:
  - Custom limits
```

## 🎯 Endpoints de Ofertas

### 1. Listar Ofertas
```http
GET /offers
```

#### Query Parameters
```typescript
interface OfferFilters {
  // Paginación
  page?: number;          // Default: 1
  limit?: number;         // Default: 20, Max: 100
  
  // Filtros
  category?: string[];    // Array de categorías
  type?: OfferType[];    // Tipos de oferta
  merchant_id?: string;   // ID del comercio
  
  // Rango de precio
  min_lumis?: number;
  max_lumis?: number;
  
  // Ubicación
  lat?: number;
  lng?: number;
  radius_km?: number;     // Default: 10
  
  // Estado
  status?: 'active' | 'scheduled';
  featured?: boolean;
  
  // Ordenamiento
  sort_by?: 'relevance' | 'lumis_asc' | 'lumis_desc' | 
            'popularity' | 'newest' | 'ending_soon';
  
  // Búsqueda
  search?: string;        // Búsqueda de texto
}
```

#### Response
```json
{
  "success": true,
  "data": {
    "offers": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "merchant": {
          "id": "merchant_123",
          "name": "Starbucks",
          "logo_url": "https://cdn.lumis.com/merchants/starbucks.png",
          "category": "coffee"
        },
        "title": "2x1 en Frappuccinos",
        "subtitle": "Todos los viernes",
        "description": "Disfruta de 2x1 en todos los Frappuccinos...",
        "type": "bogo",
        "lumis_cost": 500,
        "original_price": 120.00,
        "discount_percentage": 50,
        "images": [
          {
            "url": "https://cdn.lumis.com/offers/123/main.jpg",
            "type": "main"
          }
        ],
        "availability": {
          "start_date": "2024-12-01T00:00:00Z",
          "end_date": "2024-12-31T23:59:59Z",
          "stock": {
            "available": 45,
            "total": 100
          },
          "schedule": {
            "friday": ["14:00-18:00"]
          }
        },
        "location": {
          "zones": ["CDMX", "Polanco"],
          "distance_km": 2.3
        },
        "stats": {
          "redemptions": 55,
          "saves": 234,
          "rating": 4.5,
          "reviews_count": 23
        },
        "tags": ["coffee", "drinks", "friday"],
        "is_saved": false,
        "can_redeem": true
      }
    ],
    "pagination": {
      "page": 1,
      "limit": 20,
      "total_pages": 10,
      "total_items": 198,
      "has_next": true,
      "has_prev": false
    },
    "filters_applied": {
      "category": ["coffee"],
      "radius_km": 10
    }
  }
}
```

### 2. Detalle de Oferta
```http
GET /offers/{offer_id}
```

#### Response
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "merchant": {
      "id": "merchant_123",
      "name": "Starbucks",
      "logo_url": "https://cdn.lumis.com/merchants/starbucks.png",
      "rating": 4.6,
      "total_offers": 15,
      "address": {
        "street": "Av. Presidente Masaryk 123",
        "city": "CDMX",
        "postal_code": "11560"
      },
      "contact": {
        "phone": "+52 55 1234 5678",
        "website": "https://starbucks.com.mx"
      }
    },
    "title": "2x1 en Frappuccinos",
    "full_description": "...",
    "terms_conditions": [
      {
        "type": "restriction",
        "text": "No acumulable con otras promociones"
      },
      {
        "type": "validity",
        "text": "Válido solo viernes de 2-6 PM"
      }
    ],
    "images": [
      {
        "url": "https://cdn.lumis.com/offers/123/1.jpg",
        "type": "main",
        "width": 1200,
        "height": 800
      }
    ],
    "redemption_instructions": [
      "Muestra el código QR en caja",
      "Menciona la promoción Lümis",
      "Aplica para Frappuccinos de cualquier tamaño"
    ],
    "similar_offers": [
      // Array de ofertas similares
    ],
    "reviews": {
      "average_rating": 4.5,
      "total_reviews": 23,
      "recent": [
        {
          "id": "review_123",
          "user": {
            "name": "Juan P.",
            "avatar": "https://..."
          },
          "rating": 5,
          "comment": "Excelente promoción, muy fácil de canjear",
          "date": "2024-12-15T10:30:00Z",
          "verified_purchase": true
        }
      ]
    }
  }
}
```

### 3. Crear Redención
```http
POST /offers/{offer_id}/redemptions
```

#### Request Body
```json
{
  "validation_method": "qr",
  "device_info": {
    "platform": "ios",
    "version": "17.0",
    "model": "iPhone 14"
  },
  "location": {
    "lat": 19.4326,
    "lng": -99.1332
  }
}
```

#### Response
```json
{
  "success": true,
  "data": {
    "redemption_id": "red_abc123",
    "code": "LUMIS-ABCD-1234-WXYZ",
    "qr_data": "lumis://redemption/red_abc123",
    "qr_image_url": "https://api.lumis.com/qr/red_abc123.png",
    "expires_at": "2024-12-20T18:00:00Z",
    "expires_in_minutes": 15,
    "offer": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "2x1 en Frappuccinos",
      "merchant": "Starbucks"
    },
    "lumis_deducted": 500,
    "lumis_balance": 2450,
    "instructions": [
      "Muestra este código QR en el establecimiento",
      "El código expira en 15 minutos",
      "Guarda una captura de pantalla por si acaso"
    ]
  }
}
```

### 4. Validar Redención (Comercios)
```http
POST /merchant/redemptions/validate
```

#### Request Body
```json
{
  "redemption_code": "LUMIS-ABCD-1234-WXYZ",
  "validation_method": "qr",
  "employee_id": "emp_123",
  "terminal_id": "term_456",
  "amount": 120.00
}
```

#### Response
```json
{
  "success": true,
  "data": {
    "redemption": {
      "id": "red_abc123",
      "status": "validated",
      "validated_at": "2024-12-20T15:30:00Z"
    },
    "user": {
      "id": "user_789",
      "name": "Juan Pérez",
      "membership_level": "gold"
    },
    "offer": {
      "title": "2x1 en Frappuccinos",
      "discount_amount": 60.00
    },
    "transaction": {
      "original_amount": 120.00,
      "discount_applied": 60.00,
      "final_amount": 60.00,
      "lumis_spent": 500
    }
  }
}
```

## 🎁 Endpoints de Gift Cards

### 1. Listar Gift Cards
```http
GET /gift-cards
```

#### Response
```json
{
  "success": true,
  "data": {
    "categories": [
      {
        "name": "Entretenimiento",
        "icon": "🎬",
        "cards": [
          {
            "id": "gc_netflix",
            "provider": "Netflix",
            "logo_url": "https://cdn.lumis.com/giftcards/netflix.png",
            "denominations": [
              {
                "value": 100,
                "currency": "MXN",
                "lumis_cost": 1100
              },
              {
                "value": 200,
                "currency": "MXN",
                "lumis_cost": 2100
              }
            ],
            "availability": "instant",
            "terms_url": "https://..."
          }
        ]
      }
    ]
  }
}
```

### 2. Comprar Gift Card
```http
POST /gift-cards/{card_id}/purchase
```

#### Request Body
```json
{
  "denomination": 100,
  "quantity": 1,
  "delivery_method": "email",
  "recipient": {
    "email": "regalo@example.com",
    "name": "María García",
    "message": "¡Feliz cumpleaños!"
  }
}
```

## 🎫 Endpoints de Sorteos

### 1. Listar Sorteos Activos
```http
GET /raffles
```

### 2. Comprar Tickets
```http
POST /raffles/{raffle_id}/tickets
```

#### Request Body
```json
{
  "quantity": 5
}
```

#### Response
```json
{
  "success": true,
  "data": {
    "purchase_id": "rp_123",
    "tickets": [
      {
        "number": "0001234",
        "code": "RAF-0001234-XYZ"
      }
    ],
    "total_lumis": 500,
    "draw_date": "2024-12-31T20:00:00Z"
  }
}
```

## 👤 Endpoints de Usuario

### 1. Balance de Lümis
```http
GET /users/me/balance
```

#### Response
```json
{
  "success": true,
  "data": {
    "balance": {
      "available": 2450,
      "pending": 150,
      "reserved": 0,
      "total": 2600
    },
    "breakdown": [
      {
        "type": "regular",
        "amount": 2000,
        "expires_at": "2025-12-01T00:00:00Z"
      },
      {
        "type": "promotional",
        "amount": 450,
        "expires_at": "2025-03-01T00:00:00Z"
      }
    ],
    "next_expiration": {
      "amount": 200,
      "date": "2025-01-15T00:00:00Z"
    }
  }
}
```

### 2. Historial de Redenciones
```http
GET /users/me/redemptions
```

### 3. Ofertas Guardadas
```http
GET /users/me/saved-offers
POST /users/me/saved-offers/{offer_id}
DELETE /users/me/saved-offers/{offer_id}
```

## 📊 Endpoints de Analytics (Comercios)

### 1. Dashboard Stats
```http
GET /merchant/analytics/dashboard
```

#### Query Parameters
```
?start_date=2024-12-01&end_date=2024-12-31
```

#### Response
```json
{
  "success": true,
  "data": {
    "period": {
      "start": "2024-12-01",
      "end": "2024-12-31"
    },
    "summary": {
      "total_redemptions": 450,
      "unique_users": 380,
      "lumis_redeemed": 225000,
      "revenue_generated": 45000.00,
      "conversion_rate": 0.235
    },
    "offers_performance": [
      {
        "offer_id": "550e8400",
        "title": "2x1 Frappuccinos",
        "views": 5000,
        "saves": 450,
        "redemptions": 230,
        "conversion_rate": 0.046,
        "avg_rating": 4.5
      }
    ],
    "charts": {
      "daily_redemptions": [
        {
          "date": "2024-12-01",
          "redemptions": 15,
          "lumis": 7500
        }
      ],
      "categories": [
        {
          "category": "drinks",
          "percentage": 0.65
        }
      ],
      "peak_hours": [
        {
          "hour": 14,
          "redemptions": 45
        }
      ]
    },
    "user_insights": {
      "new_customers": 120,
      "returning_customers": 260,
      "avg_frequency": 2.3,
      "top_zip_codes": ["11560", "11000", "06600"]
    }
  }
}
```

### 2. Exportar Reportes
```http
POST /merchant/analytics/export
```

#### Request Body
```json
{
  "type": "redemptions",
  "format": "csv",
  "start_date": "2024-12-01",
  "end_date": "2024-12-31",
  "email": "reporte@comercio.com"
}
```

## 🔔 Webhooks

### Configuración
```http
POST /merchant/webhooks
```

#### Request Body
```json
{
  "url": "https://comercio.com/webhooks/lumis",
  "events": [
    "redemption.created",
    "redemption.validated",
    "offer.stock_low",
    "review.created"
  ],
  "secret": "webhook_secret_key"
}
```

### Eventos Disponibles
```yaml
Ofertas:
  - offer.created
  - offer.approved
  - offer.rejected
  - offer.expired
  - offer.stock_low
  - offer.sold_out

Redenciones:
  - redemption.created
  - redemption.validated
  - redemption.expired
  - redemption.cancelled

Reviews:
  - review.created
  - review.updated

Analytics:
  - daily.summary
  - weekly.report
```

### Payload Example
```json
{
  "event": "redemption.validated",
  "timestamp": "2024-12-20T15:30:00Z",
  "data": {
    "redemption_id": "red_abc123",
    "offer_id": "550e8400",
    "user_id": "user_789",
    "merchant_id": "merchant_123",
    "lumis_amount": 500,
    "validated_at": "2024-12-20T15:30:00Z"
  },
  "signature": "sha256=abcdef123456..."
}
```

## 🔒 Códigos de Error

```json
{
  "success": false,
  "error": {
    "code": "INSUFFICIENT_LUMIS",
    "message": "No tienes suficientes Lümis para esta oferta",
    "details": {
      "required": 500,
      "available": 450
    }
  }
}
```

### Códigos Comunes
| Código | HTTP Status | Descripción |
|--------|------------|-------------|
| `UNAUTHORIZED` | 401 | Token inválido o expirado |
| `FORBIDDEN` | 403 | Sin permisos para esta acción |
| `NOT_FOUND` | 404 | Recurso no encontrado |
| `VALIDATION_ERROR` | 400 | Datos de entrada inválidos |
| `INSUFFICIENT_LUMIS` | 400 | Lümis insuficientes |
| `OFFER_EXPIRED` | 400 | Oferta expirada |
| `STOCK_UNAVAILABLE` | 400 | Sin stock disponible |
| `LIMIT_EXCEEDED` | 400 | Límite de redenciones excedido |
| `RATE_LIMITED` | 429 | Demasiadas solicitudes |
| `INTERNAL_ERROR` | 500 | Error interno del servidor |

## 📦 SDKs Disponibles

### Flutter/Dart
```dart
import 'package:lumis_sdk/lumis_sdk.dart';

final lumis = LumisSDK(
  apiKey: 'your_api_key',
  environment: Environment.production,
);

// Obtener ofertas
final offers = await lumis.offers.list(
  filters: OfferFilters(
    category: ['coffee'],
    maxLumis: 1000,
  ),
);

// Crear redención
final redemption = await lumis.offers.redeem(
  offerId: 'offer_123',
);
```

### JavaScript/TypeScript
```typescript
import { LumisClient } from '@lumis/sdk';

const client = new LumisClient({
  apiKey: process.env.LUMIS_API_KEY,
});

// Validar redención
const result = await client.merchant.validateRedemption({
  code: 'LUMIS-ABCD-1234',
});
```

---

*Última actualización: Diciembre 2024*
*Versión de API: 1.0.0*
