# User Products API v4 - Documentación Completa

## 🎯 **Descripción General**

API para consultar productos únicos que un usuario ha comprado según sus facturas registradas en el sistema. Esta API es parte del ecosistema v4 de Lüm y sigue todas las mejores prácticas de seguridad, performance y estructuración.

## 📋 **Especificaciones Técnicas**

- **Endpoint:** `GET /api/v4/invoices/products`
- **Método:** GET
- **Autenticación:** JWT requerida
- **Formato de respuesta:** JSON (ApiResponse estándar)
- **Base de datos:** `fact_productos_factura`
- **Puerto:** 8000 (desarrollo)

## 🔐 **Autenticación y Seguridad**

### JWT Token Requerido
```bash
Authorization: Bearer <jwt_token>
```

### Características de Seguridad
- ✅ **Aislamiento por Usuario:** Solo productos del usuario autenticado
- ✅ **Validación de Token:** JWT verificado en cada request
- ✅ **Rate Limiting:** Límites de requests configurables
- ✅ **Sanitización:** Parámetros validados antes del query
- ✅ **Logging:** Auditoría completa de accesos

## 📊 **Parámetros de Consulta**

| Parámetro | Tipo | Requerido | Descripción | Ejemplo |
|-----------|------|-----------|-------------|---------|
| `update_date` | string | No | Filtrar productos por fecha de actualización (>=) | `2024-01-15` |

### Formatos de Fecha Aceptados
```bash
# Solo fecha
2024-01-15

# Fecha completa UTC
2024-01-15T10:00:00Z

# Con timezone
2024-01-15T10:00:00-05:00

# Con milisegundos
2024-01-15T10:00:00.123Z
```

## 🗄️ **Estructura de Datos**

### Query SQL Ejecutada
```sql
-- Sin filtro de fecha
SELECT 
    p.code,
    p.issuer_ruc,
    p.issuer_name,
    p.description,
    p.l1,
    p.l2,
    p.l3,
    p.l4,
    p.update_date
FROM public.dim_product p
JOIN (
    SELECT DISTINCT d.code, h.issuer_ruc, h.issuer_name, d.description
    FROM public.invoice_detail d
    JOIN public.invoice_header h
      ON d.cufe = h.cufe
    WHERE h.user_id = $1
) u
  ON p.code = u.code
 AND p.issuer_ruc = u.issuer_ruc
 AND p.issuer_name = u.issuer_name
 AND p.description = u.description
ORDER BY p.description ASC;

-- Con filtro de fecha
SELECT 
    p.code,
    p.issuer_ruc,
    p.issuer_name,
    p.description,
    p.l1,
    p.l2,
    p.l3,
    p.l4,
    p.update_date
FROM public.dim_product p
JOIN (
    SELECT DISTINCT d.code, h.issuer_ruc, h.issuer_name, d.description
    FROM public.invoice_detail d
    JOIN public.invoice_header h
      ON d.cufe = h.cufe
    WHERE h.user_id = $1
) u
  ON p.code = u.code
 AND p.issuer_ruc = u.issuer_ruc
 AND p.issuer_name = u.issuer_name
 AND p.description = u.description
WHERE p.update_date >= $2
ORDER BY p.description ASC;
```

### Estructura de Respuesta
```json
{
  "success": true,
  "message": "Successfully retrieved user products",
  "data": [
    {
      "code": "PROD001",
      "issuer_ruc": "155112341-2-DV",
      "issuer_name": "Super 99",
      "description": "Laptop Dell Inspiron 15",
      "l1": "Tecnología",
      "l2": "Computadoras", 
      "l3": "Laptops",
      "l4": "Laptops Personales",
      "update_date": "2024-08-20T10:30:00Z"
    }
  ],
  "timestamp": "2024-08-26T15:30:45Z",
  "user_id": 123
}
```

## 📝 **Ejemplos de Uso**

### 1. Obtener Todos los Productos
```bash
curl -H "Authorization: Bearer your_jwt_token" \
     "http://localhost:8000/api/v4/invoices/products"
```

### 2. Filtrar por Fecha de Actualización
```bash
curl -H "Authorization: Bearer your_jwt_token" \
     "http://localhost:8000/api/v4/invoices/products?update_date=2024-01-15"
```

### 3. Filtro con Fecha Completa
```bash
curl -H "Authorization: Bearer your_jwt_token" \
     "http://localhost:8000/api/v4/invoices/products?update_date=2024-09-01T00:00:00Z"
```

## 🎯 **Casos de Uso Empresariales**

### 1. **Historial de Compras**
- Mostrar todos los productos que el usuario ha comprado
- Ideal para secciones "Mis Compras" o "Historial"

### 2. **Sistema de Recomendaciones**
- Analizar patrones de compra del usuario
- Generar recomendaciones basadas en historial

### 3. **Actualizaciones Incrementales**
- Sincronizar solo productos nuevos o actualizados
- Optimizar transferencia de datos

### 4. **Personalización de Ofertas**
- Mostrar descuentos en productos relacionados
- Crear ofertas personalizadas basadas en compras previas

### 5. **Analytics y Reporting**
- Análisis de productos más populares por usuario
- Métricas de diversidad de compras

## ⚡ **Performance y Optimización**

### Características de Performance
- 🚀 **Query Optimizado:** SELECT DISTINCT eficiente
- 📊 **Indexado:** Índices en user_id y update_date
- 🔄 **Cacheable:** Respuestas compatibles con cache Redis
- 📝 **Métricas:** Tracking automático de response time

### Consideraciones de Escalabilidad
- **Paginación:** Futura implementación si datasets crecen
- **Índices de DB:** Optimizados para queries frecuentes
- **Cache Strategy:** TTL configurables por endpoint
- **Rate Limiting:** Protección contra abuse

## 🛠️ **Testing y Validación**

### Script de Testing
```bash
# Ejecutar test automatizado
./test_user_products_api.sh
```

### Tests Incluidos
- ✅ Acceso sin autenticación (401)
- ✅ Acceso autorizado válido (200)
- ✅ Filtros de fecha válidos
- ✅ Filtros de fecha inválidos (400)
- ✅ Estructura de respuesta
- ✅ Performance básico

## 🚨 **Manejo de Errores**

### Error 401 - No Autorizado
```json
{
  "success": false,
  "message": "Invalid or missing token",
  "data": null,
  "timestamp": "2024-08-26T15:30:45Z"
}
```

### Error 400 - Parámetros Inválidos
```json
{
  "success": false,
  "message": "Invalid date format. Use ISO 8601 format (e.g., 2024-01-15T10:00:00Z)",
  "data": null,
  "timestamp": "2024-08-26T15:30:45Z"
}
```

### Error 500 - Error del Servidor
```json
{
  "success": false,
  "message": "Database error",
  "data": null,
  "timestamp": "2024-08-26T15:30:45Z"
}
```

## 🔧 **Configuración y Despliegue**

### Variables de Entorno
```bash
DATABASE_URL=postgresql://user:pass@localhost/db
JWT_SECRET=your_secret_key
RUST_LOG=info
```

### Compilación
```bash
# Desarrollo
cargo run

# Producción
cargo build --release
./target/release/lum_rust_ws
```

## 📋 **Roadmap y Mejoras Futuras**

### V4.1 Planeado
- [ ] **Paginación:** limit/offset para datasets grandes
- [ ] **Cache Redis:** Implementación completa de cache
- [ ] **Filtros Adicionales:** Por categoría, precio, etc.
- [ ] **Agregaciones:** Count, totales por período

### V4.2 Considerado
- [ ] **GraphQL:** Endpoint alternativo para queries complejas
- [ ] **Streaming:** Para datasets muy grandes
- [ ] **ML Integration:** Recomendaciones inteligentes

## 🤝 **Contribución y Mantenimiento**

### Estructura de Archivos
```
src/api/
├── templates/
│   └── user_products_templates.rs    # SQL y tipos
├── user_products_v4.rs               # Handler y router
└── mod.rs                           # Registro de módulos
```

### Testing
```bash
# Tests unitarios
cargo test user_products

# Tests de integración  
./test_user_products_api.sh

# Benchmark
cargo bench user_products
```

## 📞 **Soporte y Contacto**

- **Documentación Principal:** `/API_ENDPOINTS.md`
- **Tests:** `./test_user_products_api.sh`
- **Logs:** Check `/var/log/lum_rust_ws.log`
- **Métricas:** `http://localhost:8000/metrics`

## 🏷️ **Versioning y Changelog**

### v4.0.0 - Inicial
- ✅ Implementación base del endpoint
- ✅ Autenticación JWT
- ✅ Filtros por fecha
- ✅ Documentación completa
- ✅ Tests automatizados

---

**Última actualización:** 26 de Agosto, 2024  
**Versión API:** v4.0.0  
**Status:** Production Ready ✅
