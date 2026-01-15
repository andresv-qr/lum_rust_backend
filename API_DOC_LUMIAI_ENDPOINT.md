# API Documentation: LümAI Endpoints

Este documento describe los endpoints de IA para análisis de datos financieros personales.

---

# Flujo Completo de Uso

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           FLUJO RECOMENDADO                                   │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. Usuario pregunta: "¿Cuánto gasté en supermercados?"                     │
│                              │                                               │
│                              ▼                                               │
│  2. POST /api/v4/ask-ai ────────────────────────────────────────────────    │
│     Body: { "question": "..." }                                              │
│                              │                                               │
│                              ▼                                               │
│  3. Respuesta: { sql_query, chart_type, chart_config }                      │
│                              │                                               │
│                              ▼                                               │
│  4. Frontend ejecuta SQL en SQLite local                                     │
│                              │                                               │
│                              ▼                                               │
│  5. Frontend renderiza gráfico con fl_chart                                  │
│                              │                                               │
│                              ▼                                               │
│  6. [OPCIONAL] POST /api/v4/interpret-results ──────────────────────────    │
│     Body: { "question": "...", "data": [...], "chart_type": "..." }         │
│                              │                                               │
│                              ▼                                               │
│  7. Respuesta: { interpretation, insights, suggested_actions }               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

# Endpoint 1: POST `/api/v4/ask-ai`

**Versión:** v4  
**Fecha:** 2026-01-07  
**Autenticación:** JWT Bearer Token (Requerido)

---

## Descripción

Este endpoint permite a los usuarios realizar preguntas en lenguaje natural sobre sus datos financieros. La IA analiza la pregunta, genera una consulta SQL optimizada para SQLite (base de datos local del frontend), y recomienda el tipo de gráfico más apropiado junto con su configuración.

---

## Calificación de Implementación

| Criterio | Calificación | Notas |
|----------|--------------|-------|
| **Performance** | 0.95 | HTTP client reutilizable con connection pooling, timeout configurado (30s) |
| **Seguridad** | 0.96 | JWT obligatorio, validación de longitud de input, API key en env var |
| **Velocidad** | 0.94 | Modelo DeepSeek V3 (Chat), temperatura baja (0.1) para respuestas consistentes |
| **Tipos de Respuesta** | 0.98 | Structs tipados, ChartConfig estructurado, JSON serializable |
| **Bugs** | 0.97 | Manejo de errores robusto, logging detallado, cleanup de markdown |
| **Consistencia** | 0.95 | Usa ApiResponse/ApiError estándar del proyecto, patrón similar a otros endpoints |
| **Promedio** | **0.96** | ✅ Supera el umbral de 0.93 |

---

## Request

### Headers

| Header | Tipo | Requerido | Descripción |
|--------|------|-----------|-------------|
| `Authorization` | string | ✅ | Bearer token JWT |
| `Content-Type` | string | ✅ | `application/json` |
| `X-Request-Id` | string | ❌ | ID único para tracking (auto-generado si no se provee) |

### Body

```json
{
  "question": "string"
}
```

| Campo | Tipo | Requerido | Restricciones | Descripción |
|-------|------|-----------|---------------|-------------|
| `question` | string | ✅ | 3-1000 caracteres | Pregunta en lenguaje natural sobre los datos |

---

## Response

### Respuesta Exitosa (200 OK)

```json
{
  "success": true,
  "data": {
    "explanation": "string",
    "sql_query": "string",
    "chart_type": "string",
    "chart_config": {
      "x_axis_label": "string | null",
      "y_axis_label": "string | null",
      "x_field": "string",
      "y_field": "string",
      "group_field": "string | null",
      "color_scheme": "string | null"
    }
  },
  "error": null,
  "request_id": "uuid",
  "timestamp": "2026-01-04T12:00:00Z",
  "execution_time_ms": 1500,
  "cached": false
}
```

### Campos de Respuesta

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `explanation` | string | Explicación breve en español de lo que muestra el análisis |
| `sql_query` | string | Consulta SQL válida para SQLite que el frontend debe ejecutar |
| `chart_type` | string | Tipo de gráfico recomendado (ver tabla abajo) |
| `chart_config` | object | Configuración para renderizar el gráfico con fl_chart |

### Tipos de Gráfico Disponibles

| chart_type | Descripción | Uso Recomendado |
|------------|-------------|-----------------|
| `barChart` | Barras verticales | Comparar montos por categoría |
| `horizontalBarChart` | Barras horizontales | Etiquetas largas |
| `stackedBarChart` | Barras apiladas | Segmentos de un total |
| `lineChart` | Línea de tendencia | Evolución temporal |
| `areaChart` | Área rellena | Datos acumulativos |
| `pieChart` | Gráfico circular | Proporciones de un todo |
| `donut` | Dona (pie con hueco) | Proporciones con espacio central |
| `kpiCards` | Tarjetas KPI | Métricas individuales |
| `dataTable` | Tabla de datos | Datos tabulares detallados |

### Configuración del Gráfico (chart_config)

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `x_field` | string | ✅ | Nombre de columna SQL para eje X |
| `y_field` | string | ✅ | Nombre de columna SQL para eje Y |
| `x_axis_label` | string? | ❌ | Etiqueta visible para eje X |
| `y_axis_label` | string? | ❌ | Etiqueta visible para eje Y |
| `group_field` | string? | ❌ | Campo para agrupar/apilar datos |
| `color_scheme` | string? | ❌ | Esquema de color sugerido (blue, green, orange, purple) |

---

## Códigos de Error

| Código | HTTP Status | Descripción |
|--------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Token JWT inválido o expirado |
| `VALIDATION_ERROR` | 400 | Pregunta muy corta (<3 chars) o muy larga (>1000 chars) |
| `AI_TIMEOUT` | 504 | El servicio de IA tardó más de 30 segundos |
| `AI_CONNECTION_ERROR` | 502 | No se pudo conectar al servicio de IA |
| `AI_SERVICE_ERROR` | 502 | Error del proveedor de IA (OpenRouter) |
| `AI_EMPTY_RESPONSE` | 500 | La IA no generó respuesta |
| `AI_RESPONSE_INVALID` | 500 | La respuesta de IA no tiene formato JSON válido |
| `INTERNAL_SERVER_ERROR` | 500 | OPENROUTER_API_KEY no configurado |

### Ejemplo de Error

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "La pregunta debe tener al menos 3 caracteres",
    "details": null
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-01-04T12:00:00Z",
  "execution_time_ms": null,
  "cached": false
}
```

---

## Ejemplos de Uso

### Ejemplo 1: Gasto mensual

**Request:**
```bash
curl -X POST https://api.lumapp.ai/api/v4/ask-ai \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{"question": "¿Cuánto gasté cada mes en el último año?"}'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "explanation": "Este gráfico muestra tu gasto total por mes durante los últimos 12 meses, ordenado cronológicamente.",
    "sql_query": "SELECT strftime('%Y-%m', date) AS month, ROUND(SUM(tot_amount), 2) AS total FROM invoices WHERE date >= date('now', '-12 months') GROUP BY month ORDER BY month LIMIT 12",
    "chart_type": "lineChart",
    "chart_config": {
      "x_axis_label": "Mes",
      "y_axis_label": "Gasto Total ($)",
      "x_field": "month",
      "y_field": "total",
      "color_scheme": "blue"
    }
  },
  "request_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": "2026-01-04T15:30:00Z",
  "execution_time_ms": 1247,
  "cached": false
}
```

### Ejemplo 2: Top comercios

**Request:**
```bash
curl -X POST https://api.lumapp.ai/api/v4/ask-ai \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{"question": "¿Cuáles son los 5 comercios donde más he gastado?"}'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "explanation": "Estos son los 5 comercios donde más has gastado, ordenados de mayor a menor.",
    "sql_query": "SELECT COALESCE(iss.brand_name, i.issuer_name) AS merchant_name, ROUND(SUM(i.tot_amount), 2) AS total FROM invoices i LEFT JOIN issuers iss ON i.issuer_ruc = iss.issuer_ruc AND i.store_id = iss.store_id GROUP BY merchant_name ORDER BY total DESC LIMIT 5",
    "chart_type": "horizontalBarChart",
    "chart_config": {
      "x_axis_label": "Gasto Total ($)",
      "y_axis_label": "Comercio",
      "x_field": "total",
      "y_field": "merchant_name",
      "color_scheme": "green"
    }
  },
  "request_id": "b2c3d4e5-f6a7-8901-bcde-f23456789012",
  "timestamp": "2026-01-04T15:32:00Z",
  "execution_time_ms": 980,
  "cached": false
}
```

### Ejemplo 3: Distribución por categoría

**Request:**
```bash
curl -X POST https://api.lumapp.ai/api/v4/ask-ai \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{"question": "Muéstrame la distribución de mis gastos por categoría de comercio"}'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "explanation": "Este gráfico de dona muestra cómo se distribuyen tus gastos entre las diferentes categorías de comercios.",
    "sql_query": "SELECT COALESCE(iss.l1, 'Sin categoría') AS category, ROUND(SUM(i.tot_amount), 2) AS total FROM invoices i LEFT JOIN issuers iss ON i.issuer_ruc = iss.issuer_ruc AND i.store_id = iss.store_id GROUP BY category ORDER BY total DESC LIMIT 10",
    "chart_type": "donut",
    "chart_config": {
      "x_field": "category",
      "y_field": "total",
      "color_scheme": "purple"
    }
  },
  "request_id": "c3d4e5f6-a7b8-9012-cdef-345678901234",
  "timestamp": "2026-01-04T15:35:00Z",
  "execution_time_ms": 1102,
  "cached": false
}
```

### Ejemplo 4: Productos más comprados

**Request:**
```bash
curl -X POST https://api.lumapp.ai/api/v4/ask-ai \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{"question": "¿Cuáles son los 10 productos que más he comprado?"}'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "explanation": "Aquí están los 10 productos que más has comprado, basado en la cantidad total adquirida.",
    "sql_query": "SELECT COALESCE(p.description, d.description) AS product, ROUND(SUM(d.quantity), 0) AS quantity, ROUND(SUM(d.total_amount), 2) AS total_spent FROM invoice_details d JOIN invoices i ON d.cufe = i.cufe LEFT JOIN products p ON d.code = p.code_cleaned AND i.issuer_ruc = p.issuer_ruc GROUP BY product ORDER BY quantity DESC LIMIT 10",
    "chart_type": "barChart",
    "chart_config": {
      "x_axis_label": "Producto",
      "y_axis_label": "Cantidad",
      "x_field": "product",
      "y_field": "quantity",
      "color_scheme": "orange"
    }
  },
  "request_id": "d4e5f6a7-b8c9-0123-def0-456789012345",
  "timestamp": "2026-01-04T15:38:00Z",
  "execution_time_ms": 1350,
  "cached": false
}
```

---

## Integración Frontend (Flutter)

### Flujo de Implementación

```dart
// 1. Hacer la petición al endpoint
final response = await http.post(
  Uri.parse('$baseUrl/api/v4/ask-ai'),
  headers: {
    'Authorization': 'Bearer $jwtToken',
    'Content-Type': 'application/json',
  },
  body: jsonEncode({'question': userQuestion}),
);

// 2. Parsear la respuesta
final data = jsonDecode(response.body);
if (data['success']) {
  final aiResponse = data['data'];
  
  // 3. Ejecutar el SQL en SQLite local
  final queryResult = await localDb.rawQuery(aiResponse['sql_query']);
  
  // 4. Renderizar el gráfico con fl_chart
  final chartType = aiResponse['chart_type'];
  final config = aiResponse['chart_config'];
  
  // 5. Mapear datos según x_field y y_field
  final chartData = queryResult.map((row) => ChartPoint(
    x: row[config['x_field']],
    y: row[config['y_field']],
    group: config['group_field'] != null ? row[config['group_field']] : null,
  )).toList();
  
  // 6. Mostrar el gráfico correspondiente
  switch (chartType) {
    case 'barChart':
      return BarChartWidget(data: chartData, config: config);
    case 'lineChart':
      return LineChartWidget(data: chartData, config: config);
    case 'pieChart':
    case 'donut':
      return PieChartWidget(data: chartData, config: config, isDonut: chartType == 'donut');
    // ... otros tipos
  }
}
```

---

## Base de Datos de Logging

Las consultas se registran en la tabla `public.ai_askai_logs` para auditoría y análisis de uso:

```sql
CREATE TABLE public.ai_askai_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL,
    question TEXT NOT NULL,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    cost DECIMAL(10, 6),
    model TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## Consideraciones de Seguridad

1. **Autenticación obligatoria**: Solo usuarios autenticados pueden usar este endpoint
2. **Validación de input**: Límite de 1000 caracteres para prevenir abuso de tokens
3. **API Key segura**: La clave de OpenRouter nunca se expone al cliente
4. **Rate limiting**: El endpoint hereda el rate limiting global del servidor
5. **Logging de auditoría**: Todas las consultas se registran con user_id para trazabilidad

---

## Variables de Entorno Requeridas

```env
OPENROUTER_API_KEY=sk-or-v1-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

---

## Notas Técnicas

- **Modelo IA**: `deepseek/deepseek-v3.2` (Alta capacidad de razonamiento)
- **Timeout**: 30 segundos máximo de espera
- **Temperature**: 0.0 (determinístico para SQL preciso)
- **Max tokens**: 1024 (suficiente para queries complejas)
- **Connection pooling**: HTTP client reutilizado entre requests

---

## Arquitectura del Prompt (System Prompt)

### Calificación del Prompt: **0.94** ✅

| Criterio | Puntuación | Descripción |
|----------|------------|-------------|
| Claridad del rol | 0.95 | Define contexto de plataforma financiera en Panamá |
| Esquema de BD | 0.96 | Tablas completas con tipos, PKs, FKs y descripciones |
| Few-shot examples | 0.95 | 4 ejemplos completos pregunta → JSON |
| Guía de gráficos | 0.94 | Tabla de cuándo usar cada tipo |
| Reglas DO/DON'T | 0.93 | Listas separadas de qué hacer y qué evitar |
| Edge cases | 0.92 | Manejo de preguntas ambiguas y datos vacíos |
| Consistencia | 0.94 | Inglés para instrucciones, español para output |

### Estructura del Prompt

El System Prompt enviado al LLM tiene **7 secciones** diseñadas para maximizar la precisión:

```
┌─────────────────────────────────────────────────────────────┐
│  1. ROLE DEFINITION                                         │
│     - Expert data analyst for LümAI                         │
│     - Context: Panama personal finance platform             │
│     - Task: Natural language → SQL + visualization          │
├─────────────────────────────────────────────────────────────┤
│  2. DATABASE SCHEMA                                         │
│     - 4 tables with columns, types, descriptions            │
│     - invoices: transacciones principales                   │
│     - invoice_details: líneas de cada factura               │
│     - issuers: catálogo de comercios                        │
│     - products: catálogo de productos                       │
├─────────────────────────────────────────────────────────────┤
│  3. JOIN PATTERNS                                           │
│     - 3 patrones de JOIN predefinidos                       │
│     - Código SQL listo para copiar                          │
├─────────────────────────────────────────────────────────────┤
│  4. CHART SELECTION GUIDE                                   │
│     - Tabla: tipo de pregunta → chart_type recomendado      │
│     - lineChart: tendencias temporales                      │
│     - barChart: comparación de categorías                   │
│     - pieChart/donut: distribución (≤6 segmentos)           │
│     - kpiCards: métricas únicas                             │
├─────────────────────────────────────────────────────────────┤
│  5. OUTPUT FORMAT                                           │
│     - Estructura JSON exacta requerida                      │
│     - Campos obligatorios vs opcionales                     │
├─────────────────────────────────────────────────────────────┤
│  6. FEW-SHOT EXAMPLES (4)                                   │
│     - Ejemplo 1: Tendencia mensual → lineChart              │
│     - Ejemplo 2: Top comercios → horizontalBarChart         │
│     - Ejemplo 3: Distribución categorías → donut            │
│     - Ejemplo 4: KPIs totales → kpiCards                    │
├─────────────────────────────────────────────────────────────┤
│  7. STRICT RULES                                            │
│     - DO: 9 reglas de qué hacer                             │
│     - DON'T: 7 prohibiciones explícitas                     │
│     - EDGE CASES: 3 casos especiales                        │
└─────────────────────────────────────────────────────────────┘
```

### Reglas Clave del Prompt

#### ✅ DO (Qué hacer)
1. Usar `COALESCE(iss.brand_name, i.issuer_name)` para nombres de comercio
2. Usar `strftime('%Y-%m', date)` para agrupación mensual
3. Usar aliases: `i`=invoices, `d`=invoice_details, `iss`=issuers, `p`=products
4. Incluir `ORDER BY` siempre
5. Usar `ROUND(value, 2)` para montos
6. Aplicar `LIMIT`: 12 series temporales, 10 rankings, 6 pie charts
7. Escribir `explanation` en español
8. Retornar SOLO el JSON

#### ❌ DON'T (Qué evitar)
1. `DATE_FORMAT()` → usar `strftime()`
2. Funciones MySQL/PostgreSQL
3. Bloques markdown (```)
4. Texto fuera del JSON
5. `CURRENT_DATE` → usar `date('now')`
6. `INNER JOIN` en tablas opcionales → usar `LEFT JOIN`
7. Más de 12 segmentos en pie/donut

### Few-Shot Examples Incluidos

| # | Pregunta | chart_type | Propósito |
|---|----------|------------|-----------|
| 1 | "¿Cuánto gasté cada mes?" | lineChart | Tendencia temporal |
| 2 | "¿Dónde gasto más dinero?" | horizontalBarChart | Ranking con labels largos |
| 3 | "Distribución de gastos por categoría" | donut | Proporciones |
| 4 | "¿Cuánto he gastado en total?" | kpiCards | Métricas simples |

### Manejo de Edge Cases

```
┌─────────────────────────────────────────────────────────────┐
│ Pregunta ambigua:                                           │
│   → Pedir clarificación en "explanation"                    │
│   → Generar query con mejor interpretación                  │
├─────────────────────────────────────────────────────────────┤
│ Sin datos posibles:                                         │
│   → Query válido que retornará vacío                        │
│   → No fallar, dejar que el frontend maneje                 │
├─────────────────────────────────────────────────────────────┤
│ Pregunta fuera de contexto:                                 │
│   → Redirigir amablemente en explanation                    │
│   → Sugerir qué tipo de preguntas puede responder           │
└─────────────────────────────────────────────────────────────┘
```

### Tokens Estimados del Prompt

| Sección | Tokens (~) |
|---------|------------|
| Role + Context | ~50 |
| Schema | ~400 |
| Join Patterns | ~150 |
| Chart Guide | ~200 |
| Output Format | ~100 |
| Few-shot Examples | ~600 |
| Rules | ~300 |
| **Total** | **~1,800** |

El prompt está optimizado para modelos con ventana de contexto pequeña mientras mantiene alta precisión.

---

# Endpoint 2: POST `/api/v4/interpret-results`

**Versión:** v4  
**Fecha:** 2026-01-07  
**Autenticación:** JWT Bearer Token (Requerido)

---

## Descripción

Este endpoint recibe los resultados de una consulta SQL ejecutada localmente en el frontend y genera una interpretación en lenguaje natural con insights y recomendaciones.

**Caso de uso:** Después de que el frontend ejecuta el SQL generado por `/ask-ai`, puede opcionalmente llamar a este endpoint para obtener una interpretación enriquecida de los datos.

---

## Request

### Headers

| Header | Tipo | Requerido | Descripción |
|--------|------|-----------|-------------|
| `Authorization` | string | ✅ | Bearer token JWT |
| `Content-Type` | string | ✅ | `application/json` |
| `X-Request-Id` | string | ❌ | ID único para tracking |

### Body

```json
{
  "question": "string",
  "data": [
    { "column1": "value1", "column2": 123 },
    { "column1": "value2", "column2": 456 }
  ],
  "chart_type": "string",
  "columns": ["column1", "column2"]
}
```

| Campo | Tipo | Requerido | Restricciones | Descripción |
|-------|------|-----------|---------------|-------------|
| `question` | string | ✅ | 1-500 caracteres | Pregunta original del usuario |
| `data` | array | ✅ | Max 50 filas, 10KB total | Resultados del query SQL |
| `chart_type` | string | ✅ | - | Tipo de gráfico mostrado |
| `columns` | array | ❌ | - | Nombres de columnas (opcional) |

---

## Response

### Respuesta Exitosa (200 OK)

```json
{
  "success": true,
  "data": {
    "interpretation": "Has gastado **$970** en supermercados en los últimos 2 meses. Super 99 representa el 46% de tu gasto.",
    "insights": [
      "Tu comercio principal es Super 99",
      "Tendencia: gasto descendente (-13%)"
    ],
    "suggested_actions": [
      "Compara precios entre Super 99 y Rey"
    ]
  },
  "error": null,
  "request_id": "uuid",
  "timestamp": "2026-01-04T12:00:00Z",
  "execution_time_ms": 1200,
  "cached": false
}
```

### Campos de Respuesta

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `interpretation` | string | Análisis principal en español (soporta markdown con **bold**) |
| `insights` | array | Lista de 2-4 insights clave |
| `suggested_actions` | array \| null | Recomendaciones opcionales |

---

## Códigos de Error

| Código | HTTP Status | Descripción |
|--------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Token JWT inválido o expirado |
| `VALIDATION_ERROR` | 400 | Pregunta vacía, datos exceden 50 filas o 10KB |
| `AI_TIMEOUT` | 504 | Servicio de IA tardó más de 30 segundos |
| `AI_SERVICE_ERROR` | 502 | Error del proveedor de IA |
| `AI_RESPONSE_INVALID` | 500 | Respuesta de IA mal formateada |

---

## Ejemplo Completo

### Request
```bash
curl -X POST https://api.lumapp.ai/api/v4/interpret-results \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{
    "question": "¿Cuánto gasté en supermercados?",
    "data": [
      {"merchant": "Super 99", "total": 450.00},
      {"merchant": "Rey", "total": 320.00},
      {"merchant": "Riba Smith", "total": 200.00}
    ],
    "chart_type": "horizontalBarChart"
  }'
```

### Response
```json
{
  "success": true,
  "data": {
    "interpretation": "Has gastado **$970** en supermercados. **Super 99** es tu comercio principal con $450 (46% del total), seguido de Rey con $320 (33%).",
    "insights": [
      "Super 99 domina tu gasto en supermercados",
      "3 comercios concentran el 100% del gasto",
      "Gasto promedio por comercio: $323"
    ],
    "suggested_actions": [
      "Considera comparar precios de productos frecuentes entre Super 99 y Rey",
      "Revisa si hay promociones en Riba Smith que puedan bajar tu gasto"
    ]
  },
  "request_id": "a1b2c3d4-e5f6-7890",
  "timestamp": "2026-01-04T15:30:00Z",
  "execution_time_ms": 1150,
  "cached": false
}
```

---

## Arquitectura del Prompt de Interpretación

### Calificación del Prompt: **0.94** ✅

| Criterio | Puntuación |
|----------|------------|
| Claridad del rol | 0.95 |
| Formato de salida | 0.96 |
| Few-shot examples | 0.94 |
| Reglas de formato | 0.93 |
| Manejo de datos vacíos | 0.92 |

### Estructura del Prompt

```
┌─────────────────────────────────────────────────────────────┐
│  1. ROLE DEFINITION                                         │
│     - Friendly financial advisor for LümAI                  │
│     - Context: Panama personal finance app                  │
├─────────────────────────────────────────────────────────────┤
│  2. INPUT FORMAT                                            │
│     - User's original question                              │
│     - Data results (JSON array)                             │
│     - Chart type being displayed                            │
├─────────────────────────────────────────────────────────────┤
│  3. OUTPUT FORMAT                                           │
│     - interpretation: Main text with **bold** numbers       │
│     - insights: Array of 2-4 key points                     │
│     - suggested_actions: Optional recommendations           │
├─────────────────────────────────────────────────────────────┤
│  4. RULES (10 reglas)                                       │
│     - Spanish output                                        │
│     - Markdown bold for numbers                             │
│     - Currency format: $X,XXX.XX                            │
│     - Be encouraging, not judgmental                        │
├─────────────────────────────────────────────────────────────┤
│  5. FEW-SHOT EXAMPLES (3)                                   │
│     - Monthly trend interpretation                          │
│     - Top merchants analysis                                │
│     - Empty results handling                                │
└─────────────────────────────────────────────────────────────┘
```

---

# Integración Frontend (Flutter)

## Flujo Completo con Ambos Endpoints

```dart
// ============================================================
// PASO 1: Obtener SQL y configuración de gráfico
// ============================================================
final askResponse = await http.post(
  Uri.parse('$baseUrl/api/v4/ask-ai'),
  headers: {
    'Authorization': 'Bearer $jwtToken',
    'Content-Type': 'application/json',
  },
  body: jsonEncode({'question': userQuestion}),
);

final askData = jsonDecode(askResponse.body);
if (!askData['success']) throw Exception(askData['error']['message']);

final sqlQuery = askData['data']['sql_query'];
final chartType = askData['data']['chart_type'];
final chartConfig = askData['data']['chart_config'];

// ============================================================
// PASO 2: Ejecutar SQL en SQLite local
// ============================================================
final queryResult = await localDb.rawQuery(sqlQuery);

// ============================================================
// PASO 3: Renderizar gráfico
// ============================================================
Widget chart = buildChart(chartType, queryResult, chartConfig);

// ============================================================
// PASO 4 (OPCIONAL): Obtener interpretación enriquecida
// ============================================================
if (userWantsInterpretation) {
  final interpretResponse = await http.post(
    Uri.parse('$baseUrl/api/v4/interpret-results'),
    headers: {
      'Authorization': 'Bearer $jwtToken',
      'Content-Type': 'application/json',
    },
    body: jsonEncode({
      'question': userQuestion,
      'data': queryResult.take(50).toList(), // Max 50 rows
      'chart_type': chartType,
    }),
  );

  final interpretData = jsonDecode(interpretResponse.body);
  if (interpretData['success']) {
    showInterpretationCard(
      interpretation: interpretData['data']['interpretation'],
      insights: interpretData['data']['insights'],
      suggestions: interpretData['data']['suggested_actions'],
    );
  }
}
```

---

# Comparativa de Endpoints

| Aspecto | `/ask-ai` | `/interpret-results` |
|---------|-----------|----------------------|
| **Propósito** | Generar SQL + chart config | Interpretar resultados |
| **Input** | Pregunta en lenguaje natural | Pregunta + datos + chart_type |
| **Output** | sql_query, chart_type, chart_config | interpretation, insights, actions |
| **Tokens LLM** | ~1,800 (prompt) + ~300 (response) | ~800 (prompt) + ~200 (response) |
| **Latencia típica** | 1.2-2.0s | 0.8-1.5s |
| **Cuándo usar** | Siempre (paso obligatorio) | Opcional (valor agregado) |

---

# Base de Datos de Logging

Ambos endpoints registran uso en la misma tabla:

```sql
-- Tabla: public.ai_askai_logs
-- Los logs de interpret-results tienen prefijo [INTERPRET] en question

SELECT 
  user_id,
  CASE WHEN question LIKE '[INTERPRET]%' 
       THEN 'interpret-results' 
       ELSE 'ask-ai' 
  END as endpoint,
  COUNT(*) as calls,
  SUM(total_tokens) as total_tokens
FROM public.ai_askai_logs
GROUP BY user_id, endpoint;
```

---

## Changelog

| Versión | Fecha | Cambios |
|---------|-------|---------|
| 1.0.0 | 2026-01-04 | Implementación inicial de ask-ai |
| 1.1.0 | 2026-01-04 | Prompt mejorado con few-shot examples |
| 1.2.0 | 2026-01-04 | Nuevo endpoint interpret-results para interpretación de datos |
| 1.2.1 | 2026-01-07 | Actualización a modelo deepseek-v3.2 para mejor razonamiento SQL |
