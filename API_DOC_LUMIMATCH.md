# API Documentation: Lumimatch (Tinder Questions Module)

## Descripción General

Lumimatch es un módulo de preguntas estilo "Tinder" que permite mostrar preguntas segmentadas a usuarios específicos. Las preguntas pueden tener 2 o 4 opciones de respuesta, cada una con texto, icono o imagen.

## Endpoints

### 1. Obtener Preguntas Pendientes

```
GET /api/v4/lumimatch/questions?limit=20
```

**Autenticación:** JWT requerido

**Query Parameters:**
| Parámetro | Tipo | Default | Rango | Descripción |
|-----------|------|---------|-------|-------------|
| `limit` | integer | 20 | 1-50 | Número de preguntas a retornar |

**Descripción:** Retorna las preguntas pendientes de responder para el usuario autenticado, filtradas según las reglas de segmentación.

**Ejemplo de uso:**
```bash
# Obtener 10 preguntas
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.example.com/api/v4/lumimatch/questions?limit=10"

# Obtener el default (20 preguntas)
curl -H "Authorization: Bearer $TOKEN" \
  "https://api.example.com/api/v4/lumimatch/questions"
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "¿Prefieres café o té?",
      "image_url": "https://cdn.example.com/questions/coffee_tea.jpg",
      "priority": 10,
      "targeting_rules": {},
      "specific_date": null,
      "created_at": "2025-12-05T10:00:00Z",
      "options": [
        {
          "id": "660e8400-e29b-41d4-a716-446655440001",
          "question_id": "550e8400-e29b-41d4-a716-446655440000",
          "label": "Café",
          "image_url": null,
          "icon_url": "https://cdn.example.com/icons/coffee.png",
          "display_order": 0
        },
        {
          "id": "660e8400-e29b-41d4-a716-446655440002",
          "question_id": "550e8400-e29b-41d4-a716-446655440000",
          "label": "Té",
          "image_url": null,
          "icon_url": "https://cdn.example.com/icons/tea.png",
          "display_order": 1
        }
      ]
    }
  ]
}
```

**Response (401 Unauthorized):**
```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Token inválido o expirado"
  }
}
```

---

### 2. Enviar Respuesta

```
POST /api/v4/lumimatch/answers
```

**Autenticación:** JWT requerido

**Body:**
```json
{
  "question_id": "550e8400-e29b-41d4-a716-446655440000",
  "option_id": "660e8400-e29b-41d4-a716-446655440001"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": "Answer received"
}
```

**Notas:**
- El endpoint es **idempotente**: si el usuario ya respondió la pregunta, retorna éxito sin crear duplicados.
- La respuesta queda registrada con timestamp en `answered_at`.

---

## Esquema de Base de Datos

**Requisitos:** PostgreSQL 18+ (para soporte nativo de UUIDv7)

### Tablas

| Tabla | Descripción |
|-------|-------------|
| `lumimatch.questions` | Preguntas con configuración y reglas de segmentación |
| `lumimatch.options` | Opciones de respuesta para cada pregunta |
| `lumimatch.user_answers` | Respuestas de usuarios |
| `lumimatch.user_tags` | Etiquetas de usuario para segmentación |

### Estructura Detallada

#### `lumimatch.questions`
| Columna | Tipo | Default | Descripción |
|---------|------|---------|-------------|
| `id` | UUID | `uuidv7()` | Identificador único (ordenable cronológicamente) |
| `title` | TEXT | NOT NULL | Texto de la pregunta |
| `image_url` | TEXT | NULL | URL de imagen principal |
| `valid_from` | TIMESTAMPTZ | NULL | Inicio de vigencia |
| `valid_to` | TIMESTAMPTZ | NULL | Fin de vigencia |
| `specific_date` | DATE | NULL | Fecha específica (solo ese día) |
| `is_active` | BOOLEAN | true | Estado de la pregunta |
| `priority` | INTEGER | 0 | Orden de prioridad (mayor = primero) |
| `targeting_rules` | JSONB | `{}` | Reglas de segmentación |
| `created_at` | TIMESTAMPTZ | `NOW()` | Fecha de creación |
| `updated_at` | TIMESTAMPTZ | `NOW()` | Fecha de actualización |

#### `lumimatch.options`
| Columna | Tipo | Default | Descripción |
|---------|------|---------|-------------|
| `id` | UUID | `uuidv7()` | Identificador único |
| `question_id` | UUID | FK | Referencia a pregunta |
| `label` | TEXT | NULL | Texto de la opción |
| `image_url` | TEXT | NULL | URL de imagen |
| `icon_url` | TEXT | NULL | URL/nombre de icono |
| `display_order` | INTEGER | 0 | Orden de visualización |

#### `lumimatch.user_answers`
| Columna | Tipo | Default | Descripción |
|---------|------|---------|-------------|
| `id` | UUID | `uuidv7()` | Identificador único |
| `user_id` | INTEGER | NOT NULL | ID del usuario |
| `question_id` | UUID | FK | Pregunta respondida |
| `option_id` | UUID | FK | Opción seleccionada |
| `answered_at` | TIMESTAMPTZ | `NOW()` | Timestamp de respuesta |

**Restricción:** `UNIQUE(user_id, question_id)` - Un usuario solo puede responder una vez cada pregunta.

#### `lumimatch.user_tags`
| Columna | Tipo | Default | Descripción |
|---------|------|---------|-------------|
| `user_id` | INTEGER | NOT NULL | ID del usuario |
| `tag` | TEXT | NOT NULL | Etiqueta de segmentación |
| `created_at` | TIMESTAMPTZ | `NOW()` | Fecha de asignación |

**Primary Key:** `(user_id, tag)`

### Nota sobre UUIDv7

Las tablas utilizan **UUIDv7** (PostgreSQL 18+) en lugar de UUIDv4 por las siguientes ventajas:
- **~20-30% más rápido** en INSERTs (menor fragmentación de índices B-tree)
- **Ordenable cronológicamente** (los UUIDs más recientes son lexicográficamente mayores)
- **Compatible** con UUIDv4 existentes (mismo tipo de dato, 128 bits)

---

## Documentación de `targeting_rules` (JSONB)

El campo `targeting_rules` en `lumimatch.questions` permite definir reglas de segmentación flexibles. **Todas las condiciones son opcionales** y se evalúan con lógica AND entre grupos.

### Estructura Completa

```json
{
  // ═══════════════════════════════════════════════════════════════════
  // DEMOGRAFÍA
  // ═══════════════════════════════════════════════════════════════════
  "min_age": 18,                          // Edad mínima del usuario
  "max_age": 65,                          // Edad máxima del usuario
  "countries": ["PA", "CO", "MX"],        // Países permitidos (ISO 3166-1 alpha-2)

  // ═══════════════════════════════════════════════════════════════════
  // USUARIOS ESPECÍFICOS
  // ═══════════════════════════════════════════════════════════════════
  "user_ids": [1, 2, 3, 100],             // Solo estos user_ids verán la pregunta

  // ═══════════════════════════════════════════════════════════════════
  // TAGS (Sistema Flexible de Segmentación)
  // ═══════════════════════════════════════════════════════════════════
  "required_tags": ["vip", "early_adopter"],  // Usuario DEBE tener TODOS (AND)
  "any_tags": ["coffee_lover", "tea_lover"],  // Usuario DEBE tener AL MENOS UNO (OR)
  "excluded_tags": ["churned", "banned"],     // Usuario NO DEBE tener NINGUNO (NOT)

  // ═══════════════════════════════════════════════════════════════════
  // PRODUCTOS (Segmentación por consumo de productos)
  // ═══════════════════════════════════════════════════════════════════
  "product_codes": ["ABC123", "XYZ789"],      // Códigos de producto específicos
  "product_l1": ["alimentos", "bebidas"],     // Categoría nivel 1
  "product_l2": ["lacteos", "gaseosas"],      // Categoría nivel 2
  "product_l3": ["leche_entera"],             // Categoría nivel 3
  "product_l4": ["leche_entera_1L"],          // Categoría nivel 4
  "product_brands": ["cocacola", "pepsi"],    // Marcas de productos

  // ═══════════════════════════════════════════════════════════════════
  // COMERCIOS/EMISORES (Segmentación por lugar de compra)
  // ═══════════════════════════════════════════════════════════════════
  "issuer_rucs": ["12345678-1-2024"],         // RUC específico del comercio
  "issuer_brand_names": ["mcdonalds", "kfc"], // Marca del comercio
  "issuer_store_names": ["mcdonalds_via_espana"], // Tienda específica
  "issuer_l1": ["restaurantes", "supermercados"], // Tipo de comercio nivel 1
  "issuer_l2": ["comida_rapida"],             // Tipo de comercio nivel 2
  "issuer_l3": ["hamburguesas"],              // Tipo de comercio nivel 3
  "issuer_l4": ["hamburguesas_premium"]       // Tipo de comercio nivel 4
}
```

---

### Descripción de Campos

#### Demografía

| Campo | Tipo | Descripción | Ejemplo |
|-------|------|-------------|---------|
| `min_age` | integer | Edad mínima requerida | `18` |
| `max_age` | integer | Edad máxima permitida | `65` |
| `countries` | array[string] | Lista de países permitidos (ISO 3166-1 alpha-2) | `["PA", "CO"]` |

#### Usuarios Específicos

| Campo | Tipo | Descripción | Ejemplo |
|-------|------|-------------|---------|
| `user_ids` | array[integer] | IDs de usuarios específicos que verán la pregunta | `[1, 2, 100]` |

#### Tags (Etiquetas)

| Campo | Tipo | Lógica | Descripción |
|-------|------|--------|-------------|
| `required_tags` | array[string] | **AND** | Usuario debe tener **TODOS** estos tags |
| `any_tags` | array[string] | **OR** | Usuario debe tener **AL MENOS UNO** de estos tags |
| `excluded_tags` | array[string] | **NOT** | Usuario **NO DEBE** tener ninguno de estos tags |

#### Productos

| Campo | Tipo | Tag Generado | Descripción |
|-------|------|--------------|-------------|
| `product_codes` | array[string] | `product_code:{valor}` | Códigos de producto específicos |
| `product_l1` | array[string] | `product_l1:{valor}` | Categoría de producto nivel 1 |
| `product_l2` | array[string] | `product_l2:{valor}` | Categoría de producto nivel 2 |
| `product_l3` | array[string] | `product_l3:{valor}` | Categoría de producto nivel 3 |
| `product_l4` | array[string] | `product_l4:{valor}` | Categoría de producto nivel 4 |
| `product_brands` | array[string] | `product_brand:{valor}` | Marcas de productos |

#### Comercios/Emisores

| Campo | Tipo | Tag Generado | Descripción |
|-------|------|--------------|-------------|
| `issuer_rucs` | array[string] | `issuer_ruc:{valor}` | RUC del comercio |
| `issuer_brand_names` | array[string] | `issuer_brand_name:{valor}` | Marca del comercio |
| `issuer_store_names` | array[string] | `issuer_store_name:{valor}` | Nombre de tienda específica |
| `issuer_l1` | array[string] | `issuer_l1:{valor}` | Tipo de comercio nivel 1 |
| `issuer_l2` | array[string] | `issuer_l2:{valor}` | Tipo de comercio nivel 2 |
| `issuer_l3` | array[string] | `issuer_l3:{valor}` | Tipo de comercio nivel 3 |
| `issuer_l4` | array[string] | `issuer_l4:{valor}` | Tipo de comercio nivel 4 |

---

### Lógica de Evaluación

1. **Entre grupos diferentes**: Se usa lógica **AND**
   - Si defines `min_age: 18` Y `countries: ["PA"]`, el usuario debe cumplir **AMBAS** condiciones.

2. **Dentro de arrays de productos/comercios**: Se usa lógica **OR**
   - Si defines `product_brands: ["cocacola", "pepsi"]`, el usuario debe haber comprado **AL MENOS UNA** de las marcas.

3. **Campos vacíos o ausentes**: Se ignoran (no filtran)
   - Si `targeting_rules: {}`, la pregunta se muestra a **TODOS** los usuarios.

---

### Ejemplos de Uso

#### 1. Pregunta para mayores de edad en Panamá

```json
{
  "min_age": 18,
  "countries": ["PA"]
}
```

#### 2. Pregunta para compradores de Coca-Cola o Pepsi

```json
{
  "product_brands": ["cocacola", "pepsi"]
}
```
*Requiere que el usuario tenga tag `product_brand:cocacola` O `product_brand:pepsi`*

#### 3. Pregunta para clientes VIP que compraron en McDonald's

```json
{
  "required_tags": ["vip"],
  "issuer_brand_names": ["mcdonalds"]
}
```

#### 4. Pregunta para usuarios específicos (campaña privada)

```json
{
  "user_ids": [1, 5, 10, 15]
}
```

#### 5. Pregunta para amantes del café que NO son churned

```json
{
  "any_tags": ["coffee_lover", "tea_lover"],
  "excluded_tags": ["churned"]
}
```

#### 6. Pregunta para compradores de lácteos en supermercados

```json
{
  "product_l2": ["lacteos"],
  "issuer_l1": ["supermercados"]
}
```

---

## Campo `specific_date`

Además de `targeting_rules`, la tabla `lumimatch.questions` tiene un campo `specific_date` (DATE) que permite mostrar una pregunta **solo en una fecha específica**.

**Ejemplo:** Pregunta que solo aparece el 25 de diciembre:
```sql
INSERT INTO lumimatch.questions (title, specific_date)
VALUES ('¿Cómo celebraste Navidad?', '2025-12-25');
```

---

## Sistema de Tags

### ¿Cómo se generan los tags?

Los tags en `lumimatch.user_tags` se generan **asincrónicamente** cuando el usuario realiza acciones en la plataforma:

1. **Al procesar una factura (vía OCR o Web Scraping):**
   - **Endpoint:** `POST /api/v4/invoices/upload-ocr`
   - **Documentación completa:** [api_endpoints_ocr.md](./api_endpoints_ocr.md)
   - Se extraen productos y se crean tags como:
     - `product_code:ABC123`
     - `product_l1:alimentos`
     - `product_brand:cocacola`
   - Se extrae información del emisor:
     - `issuer_ruc:12345678`
     - `issuer_brand_name:mcdonalds`
     - `issuer_l1:restaurantes`

2. **Tags manuales/calculados:**
   - `vip` - Asignado por lógica de negocio
   - `early_adopter` - Primeros usuarios
   - `churned` - Usuarios inactivos

### Insertar Tags Manualmente

```sql
-- Marcar usuario como VIP
INSERT INTO lumimatch.user_tags (user_id, tag)
VALUES (123, 'vip')
ON CONFLICT DO NOTHING;

-- Marcar que compró Coca-Cola
INSERT INTO lumimatch.user_tags (user_id, tag)
VALUES (123, 'product_brand:cocacola')
ON CONFLICT DO NOTHING;
```

---

## Índices para Analítica

La base de datos incluye índices optimizados para consultas de análisis:

```sql
-- Por pregunta (¿cuántos respondieron cada pregunta?)
SELECT question_id, COUNT(*) FROM lumimatch.user_answers GROUP BY question_id;

-- Por opción (¿qué opción es más popular?)
SELECT option_id, COUNT(*) FROM lumimatch.user_answers GROUP BY option_id;

-- Por fecha (tendencias temporales)
SELECT DATE(answered_at), COUNT(*) FROM lumimatch.user_answers GROUP BY DATE(answered_at);
```

---

## Arquitectura y Performance

### Flujo del Algoritmo de Selección

```
┌─────────────────────────────────────────────────────────────────────┐
│  GET /api/v4/lumimatch/questions?limit=20                          │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│  1. PARALELO: Obtener perfil de usuario + Tags del usuario         │
│     - Query A: SELECT age, country FROM dim_users                  │
│     - Query B: SELECT tag FROM lumimatch.user_tags                 │
│     Ejecutadas con tokio::join! (~3-5ms total vs 6-10ms secuencial)│
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│  2. Obtener preguntas candidatas (Over-fetch 3x)                   │
│     - LEFT JOIN con user_answers para excluir respondidas          │
│     - LIMIT = requested_limit * 3 (máx 150)                        │
│     - (~3-10ms)                                                     │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│  3. Filtrar por targeting_rules (In-Memory, Rust)                  │
│     - Evalúa: age, country, user_ids, tags, products, issuers      │
│     - Detiene al alcanzar requested_limit                          │
│     - (~0.1-0.5ms)                                                  │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│  4. Obtener opciones de preguntas filtradas                        │
│     - WHERE question_id = ANY($1) -- Una sola query                │
│     - (~2-5ms)                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Métricas de Latencia

| Escenario | Latencia Esperada |
|-----------|-------------------|
| Mejor caso | ~8ms |
| Caso típico | ~12ms |
| Peor caso | ~18ms |

### Consideraciones de Race Conditions

| Escenario | Mitigación | Resultado |
|-----------|------------|-----------|
| **Doble submit de respuesta** | `UNIQUE(user_id, question_id)` + `ON CONFLICT DO NOTHING` | ✅ Idempotente |
| **GET durante INSERT no-committed** | PostgreSQL READ COMMITTED isolation | ⚠️ Puede mostrar pregunta extra (se ignora en siguiente submit) |
| **Refresh mientras ve preguntas** | Flutter debe filtrar localmente | ✅ No hay duplicados en UI |

### Escalabilidad

- **Para <100K usuarios:** Configuración actual es óptima
- **Para 100K-1M usuarios:** Considerar cache de targeting_rules en Redis
- **Para >1M usuarios:** Considerar pre-computar question_candidates por segmento

---
## Notas de Implementación

- **Performance:** El filtrado de `targeting_rules` se hace en memoria (Rust) después de traer candidatos con over-fetch 3x.
- **Queries Paralelas:** Profile y tags se obtienen con `tokio::join!` (~30% menor latencia).
- **Escalabilidad:** El sistema de tags permite agregar nuevos criterios de segmentación sin modificar el código.
- **Idempotencia:** El endpoint de respuestas usa `ON CONFLICT DO NOTHING` para evitar duplicados.
- **UUIDs:** Se utiliza UUIDv7 (PostgreSQL 18+) para mejor performance en índices B-tree y ordenamiento cronológico nativo.
- **Generación de Tags:** Ver [api_endpoints_ocr.md](./api_endpoints_ocr.md) para detalles del procesamiento OCR que genera los tags.

---

## Versión

- **Módulo:** Lumimatch v1.2
- **Fecha:** 2025-12-05
- **Esquema:** `lumimatch`
- **Requisitos:** PostgreSQL 18+, Rust (Axum)
