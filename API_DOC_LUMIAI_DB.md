# LümAI Backend Data & Visualization Guide

Este documento describe la estructura de datos local (SQLite) y la biblioteca de visualización usada por la app para que el backend genere respuestas consistentes al trabajar con OpenRouter u otros modelos de IA.

## 1. Tablas y Relaciones (SQLite)

Todas las consultas deben considerar la coherencia entre facturas, detalles, emisores y productos. A continuación un resumen amigable para prompts de IA.

### 1.1 `invoices`
| Campo | Tipo | Descripción |
| --- | --- | --- |
| `cufe` (PK) | TEXT | Identificador fiscal único por factura |
| `issuer_name` | TEXT | Nombre legal del emisor tal como viene en la factura |
| `issuer_ruc` | TEXT | RUC del emisor (clave para joins con `issuers`) |
| `store_id` | TEXT | Identificador de sucursal |
| `date` | TEXT | Fecha ISO `YYYY-MM-DD` |
| `tot_amount` | REAL | Total pagado |
| `tot_itbms` | REAL | Impuesto ITBMS |
| `user_id` | INTEGER | Propietario local (normalmente 0) |

**Relaciones clave:**
- `invoices.cufe` <-> `invoice_details.cufe`
- `(issuer_ruc, store_id)` <-> `issuers`

### 1.2 `invoice_details`
| Campo | Tipo | Descripción |
| --- | --- | --- |
| `id` (PK) | INTEGER | Autoincremental |
| `cufe` | TEXT | FK a `invoices.cufe` |
| `description` | TEXT | Descripción del producto |
| `code` | TEXT | Código en la factura |
| `quantity` | REAL | Cantidad comprada |
| `unit_price` | REAL | Precio unitario |
| `total_amount` | REAL | Subtotal de la línea |
| `discount` | REAL | Descuento aplicado |

### 1.3 `issuers`
| Campo | Tipo | Descripción |
| --- | --- | --- |
| `issuer_ruc` (PK parcial) | TEXT | RUC del comercio |
| `store_id` (PK parcial) | TEXT | Sucursal |
| `brand_name` | TEXT | Nombre comercial limpio (usar para UI) |
| `l1` | TEXT | Categoría nivel 1 (Supermercado, Farmacia, etc.) |
| `l2` | TEXT | Categoría nivel 2 |

**Uso recomendado:** `COALESCE(iss.brand_name, i.issuer_name)` para mostrar nombres consistentes.

### 1.4 `products`
| Campo | Tipo | Descripción |
| --- | --- | --- |
| `code_cleaned` | TEXT | Código normalizado |
| `issuer_ruc` | TEXT | FK a `invoices`/`issuers` |
| `description` | TEXT | Descripción limpia |
| `l1` | TEXT | Categoría principal (Alimentos, Hogar, etc.) |
| `l2` | TEXT | Subcategoría |

**Relación sugerida:** `invoice_details.code = products.code_cleaned AND invoices.issuer_ruc = products.issuer_ruc`.

## 2. Patrones de Joins frecuentes
1. **Facturas + Detalles**
```sql
FROM invoice_details d
JOIN invoices i ON d.cufe = i.cufe
```
2. **Facturas + Comercios**
```sql
FROM invoices i
LEFT JOIN issuers iss
  ON i.issuer_ruc = iss.issuer_ruc AND i.store_id = iss.store_id
```
3. **Detalles + Productos**
```sql
FROM invoice_details d
JOIN invoices i ON d.cufe = i.cufe
LEFT JOIN products p
  ON d.code = p.code_cleaned AND i.issuer_ruc = p.issuer_ruc
```

## 3. Guía para el Prompt del Modelo
- Describe el esquema anterior en el System Prompt (puede copiar/pegar las tablas).
- Recordatorios:
  - Fechas en formato `YYYY-MM-DD`. Para agregar por mes usar `strftime('%Y-%m', date)`.
  - Preferir `COALESCE(iss.brand_name, i.issuer_name)`.
  - Evitar funciones fuera de SQLite (nada de `DATE_FORMAT`, etc.).
  - Siempre devolver JSON con campos `explanation`, `sqlQuery`, `chartType`, `chartConfig`.

## 4. Biblioteca de Gráficos en Flutter
- **Biblioteca:** `fl_chart` (Flutter)
- **Componentes disponibles:**
  - `BarChart` (vertical), `HorizontalBarChart`
  - `LineChart`, `AreaChart`, `MultiLine`
  - `PieChart`, `Donut` (Pie con hueco), `SemiPie`
  - `StackedBarChart`, `GroupedBarChart`
  - `HeatMapChart`, `KpiCards (custom)`, `DataTable`

### Tabla de referencia para `chartType`
| chartType | Uso recomendado |
| --- | --- |
| `barChart` | Comparar montos por categoría |
| `stackedBarChart` | Partes de un total segmentadas |
| `groupedBarChart` | Comparar dos periodos por categoría |
| `horizontalBarChart` | Etiquetas largas (Top comercios) |
| `lineChart` | Tendencias en el tiempo |
| `multiLineChart` | Comparar series (ej. 2024 vs 2023) |
| `areaChart` | Evolución acumulada |
| `pieChart` / `donutChart` | Distribución porcentual |
| `semiPieChart` | KPIs de progreso |
| `kpiCards` | Muestran solo valor clave |
| `table` | Listado detallado |
| `heatmap` | Intensidad por día/mes |

## 5. Formato de Respuesta Esperado
```json
{
  "explanation": "Texto amigable",
  "sqlQuery": "SELECT ...",
  "chartType": "barChart",
  "chartConfig": {
    "xAxis": "mes",
    "yAxis": "total",
    "label": "Gasto mensual",
    "value_field": "total",
    "label_field": "mes"
  }
}
```

> Entregar este documento como prompt base al equipo backend/IA para asegurar respuestas consistentes.