# 📊 Ejemplos de Datos - Sistema de Ofertas

## 🎯 Ejemplos de Ofertas por Tipo

### 1. Gift Card - Netflix
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "merchant_id": "merchant_netflix",
  "offer_type": "gift_card",
  "title": "Netflix Gift Card $500 MXN",
  "subtitle": "3 meses de entretenimiento ilimitado",
  "description": "Disfruta del mejor contenido en streaming con esta gift card de Netflix...",
  "lumis_cost": 5500,
  "original_price": 500.00,
  "images": [
    {
      "url": "https://cdn.lumis.com/giftcards/netflix-main.jpg",
      "type": "main"
    }
  ],
  "configuration": {
    "provider": "netflix",
    "denomination": 500,
    "currency": "MXN",
    "delivery_method": "email",
    "validity_months": 12,
    "instant_delivery": true
  },
  "inventory": {
    "initial_stock": 1000,
    "current_stock": 743,
    "auto_replenish": true
  },
  "restrictions": {
    "max_per_user": 5,
    "requires_kyc": false,
    "geo_restricted": false
  }
}
```

### 2. Descuento - Starbucks
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440002",
  "merchant_id": "merchant_starbucks",
  "offer_type": "discount",
  "title": "30% OFF en bebidas calientes",
  "subtitle": "Válido lunes a viernes antes de las 11 AM",
  "lumis_cost": 300,
  "discount_percentage": 30,
  "configuration": {
    "discount_type": "percentage",
    "applicable_products": ["hot_beverages"],
    "min_purchase": 150.00,
    "max_discount": 100.00,
    "schedule": {
      "monday": ["07:00-11:00"],
      "tuesday": ["07:00-11:00"],
      "wednesday": ["07:00-11:00"],
      "thursday": ["07:00-11:00"],
      "friday": ["07:00-11:00"]
    }
  },
  "zones": ["cdmx_polanco", "cdmx_roma", "cdmx_condesa"],
  "validation": {
    "method": "qr",
    "requires_receipt": true,
    "auto_apply": false
  }
}
```

### 3. Cashback - Amazon
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440003",
  "merchant_id": "merchant_amazon",
  "offer_type": "cashback",
  "title": "5% Cashback en Electrónicos",
  "subtitle": "Recibe Lümis de vuelta en tus compras",
  "lumis_cost": 0,
  "cashback_percentage": 5,
  "configuration": {
    "categories": ["electronics", "computers", "smartphones"],
    "min_purchase": 1000.00,
    "max_cashback": 5000,
    "cashback_delay_days": 30,
    "accumulative": true
  },
  "tracking": {
    "method": "affiliate_link",
    "cookie_duration_days": 7,
    "attribution_window": 24
  }
}
```

### 4. Sorteo - iPhone 15
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440004",
  "offer_type": "raffle",
  "title": "Sorteo iPhone 15 Pro Max",
  "subtitle": "Participa por solo 100 Lümis",
  "lumis_cost": 100,
  "configuration": {
    "ticket_price": 100,
    "max_tickets_per_user": 10,
    "total_tickets": 10000,
    "tickets_sold": 4532,
    "draw_date": "2024-12-31T20:00:00Z",
    "prizes": [
      {
        "position": 1,
        "prize": "iPhone 15 Pro Max 256GB",
        "value": 30000.00
      },
      {
        "position": 2,
        "prize": "AirPods Pro",
        "value": 5000.00
      },
      {
        "position": 3,
        "prize": "Apple Watch SE",
        "value": 7000.00
      }
    ],
    "legal": {
      "permit_number": "SEGOB/2024/12345",
      "terms_url": "https://lumis.com/sorteos/terminos",
      "age_restriction": 18
    }
  }
}
```

### 5. Experiencia - Concierto VIP
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440005",
  "offer_type": "experience",
  "title": "Pase VIP - Coldplay México 2025",
  "subtitle": "Incluye Meet & Greet",
  "lumis_cost": 50000,
  "original_price": 15000.00,
  "configuration": {
    "event_date": "2025-03-15T21:00:00Z",
    "venue": "Foro Sol, CDMX",
    "includes": [
      "Entrada VIP",
      "Meet & Greet con la banda",
      "Mercancía exclusiva",
      "Catering premium",
      "Estacionamiento VIP"
    ],
    "capacity": 50,
    "available_spots": 12
  },
  "validation": {
    "method": "qr_plus_id",
    "requires_id": true,
    "transfer_allowed": false
  }
}
```

## 📱 Ejemplos de Respuestas API

### Redención Exitosa
```json
{
  "success": true,
  "data": {
    "redemption_id": "red_20241220_abc123",
    "status": "success",
    "offer": {
      "id": "550e8400-e29b-41d4-a716-446655440002",
      "title": "30% OFF en bebidas calientes",
      "merchant": "Starbucks"
    },
    "transaction": {
      "lumis_spent": 300,
      "lumis_balance_before": 2750,
      "lumis_balance_after": 2450,
      "savings": 45.00,
      "timestamp": "2024-12-20T09:30:45Z"
    },
    "validation": {
      "code": "LUMIS-STAR-1220-ABCD",
      "qr_url": "https://api.lumis.com/qr/red_20241220_abc123",
      "expires_at": "2024-12-20T09:45:45Z",
      "expires_in_minutes": 15
    },
    "next_steps": [
      "Muestra el código QR en caja",
      "El código expira en 15 minutos",
      "Guarda una captura por si acaso"
    ]
  }
}
```

### Error - Lümis Insuficientes
```json
{
  "success": false,
  "error": {
    "code": "INSUFFICIENT_LUMIS",
    "message": "No tienes suficientes Lümis para esta oferta",
    "details": {
      "required": 500,
      "available": 450,
      "shortage": 50
    },
    "suggestions": [
      {
        "action": "earn_more",
        "message": "Gana 50 Lümis más completando estas actividades",
        "url": "/earn-lumis"
      },
      {
        "action": "similar_offers",
        "message": "Ver ofertas similares que puedes canjear",
        "url": "/offers?max_lumis=450"
      }
    ]
  }
}
```

## 🔄 Flujos de Datos Complejos

### Flujo de Compra de Gift Card
```yaml
1. Selección:
   Request:
     GET /gift-cards/netflix/denominations
   Response:
     denominations: [100, 200, 500, 1000]
     lumis_cost: [1100, 2150, 5500, 10800]

2. Validación:
   Request:
     POST /gift-cards/validate
     {
       "card_id": "gc_netflix",
       "denomination": 500,
       "user_lumis": 5500
     }
   Response:
     {
       "can_purchase": true,
       "final_cost": 5500,
       "delivery_time": "instant"
     }

3. Compra:
   Request:
     POST /gift-cards/purchase
     {
       "card_id": "gc_netflix",
       "denomination": 500
     }
   Response:
     {
       "purchase_id": "gcp_123",
       "status": "processing"
     }

4. Entrega:
   Webhook/Push:
     {
       "event": "gift_card.delivered",
       "purchase_id": "gcp_123",
       "code": "NFLX-XXXX-XXXX-XXXX",
       "pin": "1234",
       "instructions_url": "https://..."
     }
```

### Flujo de Sorteo con Multiple Tickets
```yaml
1. Consulta disponibilidad:
   GET /raffles/iphone15/availability
   Response:
     available_tickets: 5468
     your_tickets: 3
     max_additional: 7
     
2. Compra múltiple:
   POST /raffles/iphone15/tickets
   {
     "quantity": 5,
     "lumis_to_spend": 500
   }
   
3. Confirmación:
   Response:
     {
       "tickets": [
         {"number": "0004533", "code": "RAF-4533-ABC"},
         {"number": "0004534", "code": "RAF-4534-DEF"},
         {"number": "0004535", "code": "RAF-4535-GHI"},
         {"number": "0004536", "code": "RAF-4536-JKL"},
         {"number": "0004537", "code": "RAF-4537-MNO"}
       ],
       "total_tickets": 8,
       "win_probability": "0.08%"
     }

4. Día del sorteo:
   Push Notification:
     {
       "title": "🎰 Sorteo en 1 hora",
       "body": "El sorteo del iPhone 15 es a las 8 PM",
       "data": {
         "raffle_id": "raffle_iphone15",
         "your_tickets": [4533, 4534, 4535, 4536, 4537],
         "live_stream_url": "https://..."
       }
     }

5. Resultado:
   POST /raffles/iphone15/check-results
   Response:
     {
       "winning_numbers": [1234, 5678, 9012],
       "your_numbers": [4533, 4534, 4535, 4536, 4537],
       "won": false,
       "winners": [
         {
           "position": 1,
           "ticket": "0001234",
           "prize": "iPhone 15 Pro Max",
           "user": "Mar***a G."
         }
       ]
     }
```

## 📊 Analytics Data Examples

### Dashboard Comercio - Resumen Diario
```json
{
  "date": "2024-12-20",
  "merchant_id": "merchant_starbucks",
  "summary": {
    "offers_active": 5,
    "total_views": 15420,
    "unique_visitors": 3856,
    "redemptions": 234,
    "conversion_rate": 1.52,
    "lumis_redeemed": 70200,
    "revenue_impact": 15640.50
  },
  "by_offer": [
    {
      "offer_id": "off_001",
      "title": "30% OFF bebidas calientes",
      "performance": {
        "views": 8500,
        "clicks": 1200,
        "redemptions": 156,
        "ctr": 14.12,
        "conversion": 13.0
      }
    }
  ],
  "by_hour": [
    {"hour": 7, "redemptions": 45},
    {"hour": 8, "redemptions": 62},
    {"hour": 9, "redemptions": 38},
    {"hour": 10, "redemptions": 28}
  ],
  "customer_insights": {
    "new_customers": 45,
    "returning_customers": 189,
    "average_ticket": 245.80,
    "top_products": [
      "Caramel Macchiato",
      "Flat White",
      "Cappuccino"
    ]
  }
}
```

### User Activity Pattern
```json
{
  "user_id": "user_789",
  "period": "2024-12",
  "activity": {
    "offers_viewed": 145,
    "offers_saved": 23,
    "offers_redeemed": 8,
    "lumis_spent": 3400,
    "lumis_earned": 1250,
    "categories_preferred": [
      {"category": "food", "percentage": 45},
      {"category": "entertainment", "percentage": 30},
      {"category": "shopping", "percentage": 25}
    ],
    "peak_activity": {
      "day_of_week": "friday",
      "hour_of_day": 19
    },
    "redemption_patterns": {
      "average_lumis_per_redemption": 425,
      "favorite_merchant": "Starbucks",
      "preferred_offer_type": "discount"
    }
  },
  "predictions": {
    "likely_to_redeem": ["off_234", "off_567"],
    "churn_risk": "low",
    "lifetime_value": 45000
  }
}
```

## 🔐 Security & Validation Examples

### QR Code Data Structure
```json
{
  "version": 2,
  "type": "redemption",
  "data": {
    "redemption_id": "red_20241220_abc123",
    "offer_id": "550e8400",
    "user_id": "user_789",
    "merchant_id": "merchant_123",
    "amount": 300,
    "created_at": "2024-12-20T09:30:45Z",
    "expires_at": "2024-12-20T09:45:45Z"
  },
  "signature": "RSA-SHA256:abcdef1234567890...",
  "validation_url": "https://api.lumis.com/v1/validate/red_20241220_abc123"
}
```

### Fraud Detection Signal
```json
{
  "user_id": "user_suspicious_001",
  "signals": [
    {
      "type": "velocity",
      "description": "15 redemptions in 1 hour",
      "severity": "high",
      "timestamp": "2024-12-20T10:00:00Z"
    },
    {
      "type": "location",
      "description": "Redemptions in 3 different cities in 2 hours",
      "severity": "critical",
      "locations": ["CDMX", "Guadalajara", "Monterrey"]
    },
    {
      "type": "device",
      "description": "5 different devices used",
      "severity": "medium",
      "device_ids": ["dev_1", "dev_2", "dev_3", "dev_4", "dev_5"]
    }
  ],
  "risk_score": 85,
  "recommended_action": "suspend_account",
  "manual_review_required": true
}
```

---

*Última actualización: Diciembre 2024*
