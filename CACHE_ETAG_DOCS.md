# 🔧 **CACHE & ETAG DOCUMENTATION**

## **Overview**
Este documento explica cómo utilizar los headers de cache (ETag, If-None-Match) y las mejoras de observabilidad implementadas en la API.

## **📋 ETag & Conditional Requests**

### **¿Qué es ETag?**
ETag (Entity Tag) es un identificador único que representa una versión específica de un recurso. Permite:
- **Reducir bandwidth** con respuestas 304 Not Modified
- **Mejorar performance** evitando transferencias innecesarias
- **Cache inteligente** en el cliente

### **Headers de Cache Implementados:**

```http
# Response Headers Automáticos
Cache-Control: private, max-age=300, must-revalidate  # Con ETag
Cache-Control: private, max-age=60, must-revalidate   # Sin ETag
ETag: "1234567890abcdef"                              # Hash único del contenido
X-Response-Time-Ms: 23                               # Tiempo de respuesta

# Request Headers del Cliente  
If-None-Match: "1234567890abcdef"                    # Para cache validation
```

---

## **🚀 EJEMPLOS DE USO**

### **1. Primera Request - Sin Cache**

```bash
curl -X POST "https://api.lum.pa/v4/invoices/details" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "from_date": "2024-01-01 00:00:00",
    "to_date": "2024-01-31 23:59:59",
    "limit": 50
  }'
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json
Cache-Control: private, max-age=300, must-revalidate
ETag: "d4f2c8e1a0b3f7d9"
X-Response-Time-Ms: 245
X-RateLimit-Limit-Hour: 1000
X-RateLimit-Remaining-Hour: 999

{
  "success": true,
  "data": [...],
  "page_info": {
    "current_page": 1,
    "page_size": 50,
    "total_pages": 12,
    "has_next": true,
    "cursor_pagination": {
      "next_cursor": "eyJpZCI6MTIzNDU...",
      "has_more": true
    }
  }
}
```

### **2. Request Subsecuente - Con Cache (304)**

```bash
curl -X POST "https://api.lum.pa/v4/invoices/details" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "If-None-Match: \"d4f2c8e1a0b3f7d9\"" \
  -d '{
    "from_date": "2024-01-01 00:00:00",
    "to_date": "2024-01-31 23:59:59",  
    "limit": 50
  }'
```

**Response:**
```http
HTTP/1.1 304 Not Modified
Cache-Control: private, max-age=300, must-revalidate
ETag: "d4f2c8e1a0b3f7d9"
X-Response-Time-Ms: 8

# ✅ No body - client usa datos cached
```

### **3. Request con Datos Nuevos (200)**

```bash
curl -X POST "https://api.lum.pa/v4/invoices/details" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "If-None-Match: \"old_etag_value\"" \
  -d '{
    "from_date": "2024-01-01 00:00:00", 
    "to_date": "2024-01-31 23:59:59",
    "limit": 50
  }'
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json
Cache-Control: private, max-age=300, must-revalidate
ETag: "new_etag_d4f2c8e1a0b3f7d9"  # ✅ Nuevo ETag
X-Response-Time-Ms: 198

{
  "success": true,
  "data": [...],  # ✅ Nuevos datos
  "page_info": {...}
}
```

---

## **📊 OBSERVABILIDAD & MÉTRICAS**

### **Endpoints de Monitoreo:**

| Endpoint | Propósito | Formato |
|----------|-----------|---------|
| `/health` | Health check básico | JSON |
| `/health/detailed` | Health check con dependencias | JSON |
| `/metrics` | Métricas Prometheus | Text |
| `/metrics/json` | Métricas JSON | JSON |
| `/ready` | Kubernetes readiness probe | Text |
| `/live` | Kubernetes liveness probe | Text |

### **Health Check Básico:**

```bash
curl https://api.lum.pa/health
```

```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "service": "lum_rust_ws"
}
```

### **Health Check Detallado:**

```bash
curl https://api.lum.pa/health/detailed
```

```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z", 
  "version": "0.1.0",
  "uptime_seconds": 86400,
  "database": {
    "status": "healthy",
    "connection_pool_size": 10,
    "active_connections": 3,
    "last_query_duration_ms": 23
  },
  "redis": {
    "status": "healthy",
    "connection_count": 5,
    "last_ping_duration_ms": 1
  },
  "memory_usage": {
    "allocated_bytes": 1048576,
    "heap_size_bytes": 2097152,
    "peak_allocated_bytes": 1572864
  }
}
```

### **Métricas Prometheus:**

```bash
curl https://api.lum.pa/metrics
```

```
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",status="200"} 1234
http_requests_total{method="POST",status="200"} 567

# HELP http_request_duration_seconds HTTP request duration in seconds  
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_sum 45.2
http_request_duration_seconds_count 1890

# HELP database_connections_active Active database connections
# TYPE database_connections_active gauge
database_connections_active 3
```

### **Métricas JSON:**

```bash
curl https://api.lum.pa/metrics/json
```

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "service": "lum_rust_ws",
  "version": "0.1.0",
  "metrics": {
    "http_requests": {
      "total": 1890,
      "success_rate": 0.953,
      "avg_duration_ms": 23.9,
      "p95_duration_ms": 87.2,
      "p99_duration_ms": 156.8
    },
    "database": {
      "pool_size": 10,
      "active_connections": 3,
      "query_count": 15420,
      "avg_query_duration_ms": 12.5
    },
    "business_metrics": {
      "invoices_processed_today": 1250,
      "qr_codes_detected_today": 890,
      "user_sessions_active": 45
    }
  }
}
```

---

## **🔍 Headers de Observabilidad**

Todos los endpoints incluyen headers automáticos:

```http
X-Response-Time-Ms: 156          # Tiempo de respuesta en milisegundos
X-RateLimit-Limit-Hour: 1000     # Límite por hora  
X-RateLimit-Remaining-Hour: 847  # Requests restantes en la hora
X-RateLimit-Limit-Day: 10000     # Límite por día
X-RateLimit-Remaining-Day: 8653  # Requests restantes en el día
X-Request-ID: req_1234567890     # ID único de request (si implementado)
```

---

## **⚡ PAGINACIÓN KEYSET (CURSOR)**

### **Beneficios:**
- **🚀 Performance constante** - O(1) independiente del offset
- **📊 Consistency** - Sin duplicados con datos dinámicos  
- **💾 Escalabilidad** - Funciona con millones de registros

### **Invoice Details con Cursor:**

```bash
# Primera página
curl -X POST "https://api.lum.pa/v4/invoices/details" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "from_date": "2024-01-01 00:00:00",
    "to_date": "2024-01-31 23:59:59",
    "limit": 20,
    "cursor": null,
    "direction": "next"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": [...],
  "page_info": {
    "cursor_pagination": {
      "next_cursor": "eyJpZCI6MTIzNDUsImRhdGUi...", 
      "previous_cursor": null,
      "has_more": true,
      "direction": "next"
    }
  }
}
```

```bash
# Página siguiente
curl -X POST "https://api.lum.pa/v4/invoices/details" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "from_date": "2024-01-01 00:00:00", 
    "to_date": "2024-01-31 23:59:59",
    "limit": 20,
    "cursor": "eyJpZCI6MTIzNDUsImRhdGUi...",
    "direction": "next"
  }'
```

### **Invoice Headers con Cursor:**

```bash
curl -X POST "https://api.lum.pa/v4/invoices/headers" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "from_date": "2024-01-01 00:00:00",
    "to_date": "2024-01-31 23:59:59", 
    "limit": 50,
    "cursor": "eyJyZWNlcHRpb25fZGF0ZSI6Ij...",
    "direction": "next"
  }'
```

---

## **🛡️ SEGURIDAD & RATE LIMITING**

### **Headers de Rate Limiting:**
```http
X-RateLimit-Limit-Hour: 1000     # Límite por hora según endpoint
X-RateLimit-Remaining-Hour: 847  # Requests restantes  
X-RateLimit-Limit-Day: 10000     # Límite diario
X-RateLimit-Remaining-Day: 8653  # Requests restantes hoy
```

### **Límites por Endpoint:**
```
/v4/invoices/details    - 100/hora, 1000/día
/v4/invoices/headers    - 200/hora, 2000/día  
/v4/qr/process         - 50/hora, 500/día
/v4/auth/*             - 20/hora, 100/día
```

### **Responses de Rate Limit:**
```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit-Hour: 100
X-RateLimit-Remaining-Hour: 0
Retry-After: 1800

{
  "error": "Rate limit exceeded",
  "message": "Too many requests. Try again in 30 minutes.",
  "retry_after": 1800
}
```

---

## **🔧 CONFIGURACIÓN DEL CLIENTE**

### **Client-Side Caching (JavaScript):**

```javascript
class LumAPIClient {
  constructor(baseURL, token) {
    this.baseURL = baseURL;
    this.token = token;
    this.cache = new Map(); // Simple in-memory cache
  }

  async request(endpoint, data) {
    const cacheKey = JSON.stringify({endpoint, data});
    const cached = this.cache.get(cacheKey);
    
    const headers = {
      'Authorization': `Bearer ${this.token}`,
      'Content-Type': 'application/json'
    };
    
    // Add If-None-Match if we have cached ETag
    if (cached?.etag) {
      headers['If-None-Match'] = cached.etag;
    }
    
    const response = await fetch(`${this.baseURL}${endpoint}`, {
      method: 'POST',
      headers,
      body: JSON.stringify(data)
    });
    
    // Handle 304 Not Modified
    if (response.status === 304) {
      console.log('Using cached data');
      return cached.data;
    }
    
    const responseData = await response.json();
    const etag = response.headers.get('ETag');
    
    // Cache the response with ETag
    if (etag) {
      this.cache.set(cacheKey, {
        data: responseData,
        etag: etag,
        timestamp: Date.now()
      });
    }
    
    return responseData;
  }
}

// Usage
const client = new LumAPIClient('https://api.lum.pa', 'your_jwt_token');
const invoices = await client.request('/v4/invoices/details', {
  from_date: '2024-01-01 00:00:00',
  limit: 50
});
```

---

## **📈 MONITORING & ALERTING**

### **Grafana Dashboard Queries:**

```promql
# Request Rate
rate(http_requests_total[5m])

# Response Time P95  
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Error Rate
rate(http_requests_total{status=~"4..|5.."}[5m]) / rate(http_requests_total[5m])

# Database Connection Pool
database_connections_active / database_connections_max

# Cache Hit Rate  
rate(cache_hits_total[5m]) / rate(cache_requests_total[5m])
```

### **Alerting Rules:**

```yaml
groups:
- name: lum_api_alerts
  rules:
  - alert: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
    for: 2m
    
  - alert: HighResponseTime
    expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1.0
    for: 5m
    
  - alert: DatabaseDown
    expr: database_connections_active == 0
    for: 1m
```

---

## **✅ BEST PRACTICES**

### **Cliente:**
1. **Siempre enviar If-None-Match** si tienes ETag cached
2. **Respetar Cache-Control headers** 
3. **Manejar respuestas 304** correctamente
4. **Implementar exponential backoff** en 429 responses
5. **Usar cursor pagination** para datasets grandes

### **Servidor:**
1. **ETags se generan automáticamente** - no requiere código adicional
2. **Rate limits son por usuario** y endpoint
3. **Métricas se colectan automáticamente**
4. **Health checks incluyen todas las dependencias**
5. **Logs estructurados** para mejor observabilidad

---

## **🐛 TROUBLESHOOTING**

### **Cache Issues:**
```bash
# Forzar refresh (ignore cache)
curl -H "Cache-Control: no-cache" ...

# Ver headers de debug
curl -v https://api.lum.pa/v4/invoices/details
```

### **Rate Limit Issues:**
```bash
# Check remaining limits
curl -I https://api.lum.pa/v4/invoices/details

# Ver cabeceras X-RateLimit-* en response
```

### **Performance Issues:**
```bash
# Check metrics
curl https://api.lum.pa/metrics/json

# Check health  
curl https://api.lum.pa/health/detailed
```
