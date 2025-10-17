# Comparación de Campos: OCR vs Web Scraping

## Resumen Ejecutivo

Este documento compara los campos extraídos y guardados en la base de datos por dos métodos diferentes:
1. **OCR (API `/invoices/upload-ocr`)**: Extrae datos de imágenes de facturas usando Gemini 2.0-flash
2. **Web Scraping (API `/invoices/process-from-url`)**: Extrae datos de URLs de la DGI de Panamá

---

## 📊 Tabla de Comparación de Campos

### TABLA: `invoice_header` / `invoice_headers`

| Campo Base de Datos | OCR (`upload-ocr`) | Web Scraping (`process-from-url`) | Notas |
|---------------------|-------------------|-----------------------------------|-------|
| **CUFE** | ✅ `OCR-{RUC+DV}-{FECHA}-{NUMERO}` | ✅ Extraído de URL `chFE` | **DIFERENCIA**: Formato diferente |
| **issuer_name** / proveedor_nombre | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **no** / numero_factura | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **date** / fecha_emision | ✅ Extraído por Gemini (YYYY-MM-DD) | ✅ Extraído del HTML (DD/MM/YYYY HH:MM:SS) | **DIFERENCIA**: Formato diferente |
| **issuer_ruc** / proveedor_ruc | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **issuer_dv** | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **issuer_address** | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **issuer_phone** | ❌ NO extraído | ✅ Extraído del HTML | **FALTA EN OCR** |
| **tot_amount** / total | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **tot_itbms** / impuestos | ✅ Siempre 0.0 (no calculado) | ✅ Extraído del HTML | **DIFERENCIA**: OCR no calcula |
| **receptor_name** / cliente_nombre | ❌ NO extraído | ✅ Extraído del HTML | **FALTA EN OCR** |
| **receptor_id** / cliente_ruc | ❌ NO extraído | ✅ Extraído del HTML | **FALTA EN OCR** |
| **receptor_dv** | ❌ NO extraído | ✅ Extraído del HTML | **FALTA EN OCR** |
| **receptor_address** | ❌ NO extraído | ✅ Extraído del HTML | **FALTA EN OCR** |
| **receptor_phone** | ❌ NO extraído | ✅ Extraído del HTML | **FALTA EN OCR** |
| **type** / type_field | ✅ `"ocr_pending"` | ✅ `"QR"` | **DIFERENCIA**: Valores diferentes |
| **origin** | ✅ `"api"` | ✅ `"app"` | **DIFERENCIA**: Valores diferentes |
| **user_id** | ✅ Del JWT | ✅ Hardcoded a 1 | **DIFERENCIA**: Fuente diferente |
| **user_email** | ✅ Del JWT | ❌ None | **DIFERENCIA**: Solo OCR |
| **user_ws** | ✅ None (opcional) | ❌ None | Igual |
| **url** | ✅ Data URL (imagen base64) | ✅ URL de la DGI | **DIFERENCIA**: Tipo de URL |
| **time** | ✅ HHMMSS format | ❌ None | **DIFERENCIA**: Solo OCR |
| **process_date** | ✅ UTC timestamp | ✅ UTC timestamp | Igual |
| **reception_date** | ✅ UTC timestamp | ✅ UTC timestamp | Igual |
| **auth_date** | ❌ NO extraído | ❌ NO extraído | Igual (ambos vacío) |

### TABLA: `invoice_detail` / `invoice_details`

| Campo Base de Datos | OCR (`upload-ocr`) | Web Scraping (`process-from-url`) | Notas |
|---------------------|-------------------|-----------------------------------|-------|
| **cufe** | ✅ Mismo CUFE | ✅ Mismo CUFE | Igual |
| **partkey** | ✅ `{cufe}\|{index}` | ❌ Numérico `1, 2, 3...` | **DIFERENCIA**: Formato diferente |
| **code** | ✅ `"OCR-{index}"` | ❌ NO extraído | **DIFERENCIA**: Solo OCR |
| **description** / descripcion | ✅ Nombre del producto | ✅ Descripción del producto | Mismo concepto |
| **quantity** / cantidad | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **unit_price** / precio_unitario | ✅ Extraído por Gemini | ✅ Extraído del HTML | Mismo concepto |
| **unit_discount** | ✅ Siempre "0" | ❌ NO extraído | **DIFERENCIA**: Solo OCR |
| **amount** | ✅ total_price del producto | ❌ NO extraído | **DIFERENCIA**: Solo OCR |
| **itbms** | ✅ Siempre "0" | ✅ impuesto_monto del HTML | **DIFERENCIA**: OCR no calcula |
| **total** | ✅ total_price del producto | ✅ Total del HTML | Mismo concepto |
| **date** | ✅ Fecha de la factura | ❌ NO extraído | **DIFERENCIA**: Solo OCR |
| **information_of_interest** | ✅ "Extraído por OCR" | ❌ NO extraído | **DIFERENCIA**: Solo OCR |
| **item_numero** | ❌ NO se mapea | ✅ Número del ítem | **DIFERENCIA**: Solo Scraping |
| **impuesto_porcentaje** | ❌ NO se mapea | ✅ Porcentaje del HTML | **DIFERENCIA**: Solo Scraping |
| **subtotal** | ❌ NO se mapea | ✅ Subtotal del HTML | **DIFERENCIA**: Solo Scraping |

### TABLA: `invoice_payment` / `invoice_payments`

| Campo Base de Datos | OCR (`upload-ocr`) | Web Scraping (`process-from-url`) | Notas |
|---------------------|-------------------|-----------------------------------|-------|
| **cufe** | ✅ Mismo CUFE | ✅ Mismo CUFE | Igual |
| **total_pagado** | ✅ Total de la factura | ❌ NO extraído | **DIFERENCIA**: Solo OCR |
| **forma_de_pago** / metodo_pago | ✅ "Efectivo" (default) | ✅ Método del HTML | **DIFERENCIA**: OCR es hardcoded |
| **efectivo** | ✅ Total de la factura | ❌ NO existe en Scraping | **DIFERENCIA**: Solo OCR |
| **valor_pago** | ✅ Total de la factura | ❌ NO existe en Scraping | **DIFERENCIA**: Solo OCR |
| **monto** | ❌ NO existe en OCR | ✅ Monto del HTML | **DIFERENCIA**: Solo Scraping |
| **referencia** | ❌ NO existe en OCR | ✅ Referencia del HTML | **DIFERENCIA**: Solo Scraping |

---

## 🚨 Inconsistencias Críticas Detectadas

### 1. **Esquemas de Base de Datos Diferentes**
- **OCR usa**: `invoice_header`, `invoice_detail`, `invoice_payment` (singular)
- **Scraping usa**: `invoice_headers`, `invoice_details`, `invoice_payments` (plural)
- **Impacto**: Son tablas DIFERENTES en la base de datos

### 2. **Formato de CUFE Diferente**
- **OCR**: `OCR-{RUC+DV}-{FECHA}-{NUMERO}` (ej: `OCR-123456712-20240115-00001`)
- **Scraping**: `chFE` extraído de la URL de la DGI (ej: `FE-001-00000001-20231130-123456789-001`)
- **Impacto**: No son compatibles, no se pueden relacionar facturas entre sistemas

### 3. **Campos de Receptor Faltantes en OCR**
El OCR NO extrae información del receptor (cliente):
- `receptor_name`
- `receptor_id` (RUC)
- `receptor_dv`
- `receptor_address`
- `receptor_phone`

**Impacto**: No se puede identificar quién recibió la factura en OCR

### 4. **ITBMS No Calculado en OCR**
- **OCR**: Siempre guarda `tot_itbms = 0.0`
- **Scraping**: Extrae el ITBMS del HTML
- **Impacto**: Datos fiscales incompletos en OCR

### 5. **Campos de Detalle Incompatibles**
- **partkey**: OCR usa formato `{cufe}|{index}`, Scraping usa número simple
- **code**: Solo OCR lo genera
- **impuesto_porcentaje**, **subtotal**: Solo Scraping los extrae
- **Impacto**: Dificulta consultas unificadas

### 6. **Campos de Pago Incompatibles**
- **OCR**: Usa `total_pagado`, `efectivo`, `valor_pago`, `forma_de_pago`
- **Scraping**: Usa `monto`, `referencia`, `metodo_pago`
- **Impacto**: Esquemas diferentes para la misma tabla

---

## 📋 Recomendaciones

### Corto Plazo (Crítico)
1. **Unificar esquemas de tablas**: Decidir si usar singular o plural
2. **Agregar extracción de receptor en OCR**: Modificar prompt de Gemini
3. **Calcular ITBMS en OCR**: Sumar impuestos de productos
4. **Estandarizar formato de partkey**: Usar el mismo formato en ambos
5. **Unificar campos de pago**: Mapear correctamente los campos

### Mediano Plazo
1. **Crear un servicio unificado de guardado**: Una función que maneje ambos casos
2. **Normalizar formato de fechas**: Usar ISO 8601 en ambos
3. **Agregar validación de esquema**: Asegurar que ambos métodos guarden los mismos campos
4. **Documentar mapeo de campos**: Mantener esta documentación actualizada

### Largo Plazo
1. **Migración de datos**: Unificar tablas existentes
2. **API unificada de consulta**: Que funcione independiente de la fuente
3. **Tests de integración**: Verificar compatibilidad entre métodos

---

## 🔍 Campos Mapeados Correctamente

Estos campos SÍ están consistentes entre ambos métodos:
- ✅ `cufe` (aunque con formatos diferentes)
- ✅ `issuer_name` / `proveedor_nombre`
- ✅ `issuer_ruc` / `proveedor_ruc`
- ✅ `issuer_dv`
- ✅ `no` / `numero_factura`
- ✅ `tot_amount` / `total`
- ✅ Productos con `descripcion`, `cantidad`, `precio_unitario`, `total`

---

## 📝 Conclusión

Existen **inconsistencias significativas** entre los dos métodos de extracción:

1. **Tablas diferentes**: OCR usa tablas singulares, Scraping usa plurales
2. **Campos faltantes en OCR**: No extrae información del receptor
3. **Cálculos incompletos**: OCR no calcula ITBMS
4. **Formatos diferentes**: Partkeys, fechas, y estructuras de pago

**Prioridad Alta**: Unificar los esquemas y agregar campos faltantes en OCR para garantizar compatibilidad.

---

**Fecha de análisis**: 2025-10-01  
**Autor**: GitHub Copilot  
**Archivos analizados**:
- `/home/client_1099_1/scripts/lum_rust_ws/src/services/ocr_service.rs`
- `/home/client_1099_1/scripts/lum_rust_ws/src/api/webscraping/mod.rs`
- `/home/client_1099_1/scripts/lum_rust_ws/src/api/database_persistence.rs`
