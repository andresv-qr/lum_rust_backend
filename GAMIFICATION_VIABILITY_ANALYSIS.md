# Análisis de Viabilidad de Gamificación (Backend Rust + Frontend Flutter)

Este documento detalla la viabilidad técnica de implementar diversas mecánicas de gamificación en el ecosistema actual (Backend Rust + App Flutter), considerando las limitaciones actuales (sin integración POS en tiempo real, OCR asíncrono).

**Escala de Viabilidad:**
*   **1.0 - 0.9:** Inmediato / Muy Viable (Quick Win)
*   **0.8 - 0.7:** Viable (Requiere esfuerzo moderado)
*   **0.6 - 0.5:** Medio (Complejidad técnica o dependencia de datos sucios)
*   **0.4 - 0.0:** Difícil / Inviable (Requiere cambios arquitectónicos mayores o datos inexistentes)

---

## 📊 Tabla de Análisis Detallado

| Familia | Mecánica | Backend (Rust) | Frontend (Flutter) | Justificación Técnica | Estado |
| :--- | :--- | :---: | :---: | :--- | :---: |
| **ACELERADORES** | **El Rompe-Hielo (Flash Hour)** | **0.9** | **0.9** | **Muy Viable.** Usando `reception_date` (hora de escaneo) en lugar de hora de emisión (que el OCR a veces no trae). Fácil de validar en Rust. | 🟢 VERDE |
| | **LÜM Drop (El Rescate)** | 0.1 | 0.5 | **Inviable.** No tenemos inventario en tiempo real. Requiere integración profunda con el POS del comercio. | ⚫ NEGRO |
| | **Rescate Nocturno** | **0.9** | **0.9** | **Muy Viable.** Igual que "El Rompe-Hielo". Regla simple: `IF hour(now) BETWEEN 21 AND 23`. | 🟢 VERDE |
| | **La Chispa Oculta** | **1.0** | **1.0** | **Inmediato.** Lógica de RNG (`rand::thread_rng`) al procesar la factura. Frontend solo muestra un popup. | 🟢 VERDE |
| | **Ruleta del Ticket** | **1.0** | **0.8** | **Inmediato.** Ya tenemos el `total` validado. Frontend puede usar una librería de ruleta o animación simple. | 🟢 VERDE |
| | **Cuenta Regresiva** | **0.9** | **0.8** | **Muy Viable.** Basado en hora de escaneo. Fórmula de degradación lineal simple en Backend. | 🟢 VERDE |
| | **Stock de Premios** | 0.8 | 0.8 | **Viable.** Contador global en Redis o SQL (`UPDATE rewards SET stock = stock - 1`). | 🟢 VERDE |
| | **Ventana Sorprendente** | 0.6 | 0.8 | **Medio.** Depende de la calidad de `dim_product`. Si el producto no está categorizado (`l1`..`l4`), la regla falla. | ⚠️ AMARILLO |
| | **Drop Programado** | **1.0** | **0.9** | **Inmediato.** Cron job en Rust (`tokio-cron-scheduler`) que activa un flag global. | 🟢 VERDE |
| **ARQUITECTOS** | **Constelación (Streak)** | **1.0** | **1.0** | **Ya Implementado.** Vi tablas `fact_user_streaks` en tu esquema. Solo es exponerlo al frontend. | 🟢 VERDE |
| | **Escalera al Zénit** | **1.0** | **0.9** | **Muy Viable.** Sumatoria simple `SELECT SUM(total)`. UI de barra de progreso. | 🟢 VERDE |
| | **Re-Encendido (Win-Back)** | **0.9** | **0.8** | **Muy Viable.** Job diario que detecta `last_login > X days` y envía push/email. | 🟢 VERDE |
| | **Pasaporte de Sabores** | 0.3 | 0.7 | **Difícil.** El OCR lee "REFRESCO 2L", no siempre detecta la marca "Coca-Cola" explícitamente sin un catálogo maestro robusto. | 🔴 ROJO |
| | **Rueda de Compras** | 0.5 | 0.8 | **Medio.** Requiere que el usuario compre en 3 categorías distintas. Depende de la calidad de categorización de productos. | ⚠️ AMARILLO |
| | **Escala Progresiva** | **1.0** | **0.9** | **Muy Viable.** `COUNT(invoices) WHERE date > week_start`. Lógica robusta y simple. | 🟢 VERDE |
| | **Checkpoints** | 0.7 | 0.6 | **Viable.** Requiere una tabla nueva `user_quest_progress` para guardar el estado de los pasos. | ⚠️ AMARILLO |
| | **Objetivos Rotativos** | **0.9** | **0.8** | **Muy Viable.** Configuración en DB (`dim_engagement_mechanics`) que cambia diariamente. | 🟢 VERDE |
| | **Misión Asignada (IA)** | 0.2 | 0.5 | **Complejo.** Requiere un motor de recomendación (ML) que no vi en el stack actual. | 🔴 ROJO |
| | **Ruta Inteligente** | 0.6 | 0.7 | **Medio.** Lógica de desbloqueo secuencial. Factible pero aumenta complejidad de gestión de estado. | ⚠️ AMARILLO |
| **CONECTORES** | **Dúo Dinámico** | 0.5 | 0.7 | **Riesgo.** Detectar 2 categorías en una misma factura depende 100% de que el OCR identifique ambos productos correctamente. | ⚠️ AMARILLO |
| | **Ruta del Sabor** | 0.8 | 0.8 | **Viable.** Si tenemos los comercios clasificados (ej. "Restaurante"), es fácil validar `merchant_category`. | ⚠️ AMARILLO |
| | **Desafío Hogar Smart** | 0.4 | 0.7 | **Difícil.** Detectar "Smart TV" vs "TV normal" por OCR es propenso a errores sin SKUs exactos. | ⚠️ AMARILLO |
| | **Ciclo de Reposición** | 0.2 | 0.6 | **Complejo.** Requiere predecir cuándo se acaba el producto. Mucha lógica de datos histórica necesaria. | 🔴 ROJO |
| | **Asado Perfecto** | 0.3 | 0.7 | **Difícil.** Detectar "Carbón" es difícil si el ticket dice "BOLSA 5KG". | 🔴 ROJO |
| | **Semana del Bebé** | 0.6 | 0.8 | **Medio.** Categoría "Bebé" suele ser fácil de detectar (Pañales, Leche), pero no infalible. | ⚠️ AMARILLO |
| | **Beauty Lovers** | 0.6 | 0.8 | **Medio.** Similar al anterior. Depende del catálogo de productos (`dim_product`). | ⚠️ AMARILLO |
| | **Fin de Semana Dorado** | 0.7 | 0.8 | **Viable.** Filtro simple: `Day IN (Fri, Sat, Sun) AND Category = 'Licor'`. | ⚠️ AMARILLO |
| | **Día de Mascotas** | 0.6 | 0.8 | **Medio.** Depende de detectar marcas de comida de perro/gato. | ⚠️ AMARILLO |
| | **Combo Inteligente** | 0.3 | 0.7 | **Difícil.** Exige 3 aciertos simultáneos del OCR/Categorizador. Alta probabilidad de frustración del usuario. | 🔴 ROJO |
| | **Carrito Inteligente** | 0.5 | 0.7 | **Medio.** `COUNT(DISTINCT category_id) >= 3`. Posible, pero sensible a errores de OCR. | ⚠️ AMARILLO |
| | **Efecto Dominó** | **1.0** | **0.8** | **Excelente.** `COUNT(DISTINCT merchant_id)`. Muy robusto y fácil de medir. | 🟢 VERDE |
| | **Ritual del Primerizo** | 0.8 | 0.8 | **Viable.** `SELECT count(*) WHERE category = X`. Si es 0, es la primera vez. | ⚠️ AMARILLO |
| | **Genealogía del Gusto** | **1.0** | **0.6** | **Muy Viable.** Solo es guardar respuestas de un formulario en DB. | 🟢 VERDE |
| **EXPLORADORES**| **Ojo Borroso** | **0.9** | **0.7** | **Viable.** Juego simple en Flutter. Backend solo valida la respuesta correcta. | 🟢 VERDE |
| | **Oráculo (Survey)** | **1.0** | **0.8** | **Muy Viable.** Ya vi tablas de encuestas (`load_encuestas_panama.sql`). Es activar el trigger post-scan. | 🟢 VERDE |
| | **Primera Factura Ciega** | 0.8 | 0.8 | **Viable.** Igual que "Ritual del Primerizo". | ⚠️ AMARILLO |
| | **Operación Pharma** | **0.9** | **0.8** | **Muy Viable.** Si tenemos una lista de RUCs de farmacias o `merchant_category`, es trivial. | 🟢 VERDE |
| | **Cazador de Tendencia** | 0.8 | 0.8 | **Viable.** Buscar strings específicos en `invoice_detail`. "Vegano", "Keto". | 🟢 VERDE |
| | **Cacería de Artefactos** | 0.4 | 0.7 | **Difícil.** Complejo de explicar al usuario y validar múltiples ítems. | ⚠️ AMARILLO |
| | **Categorías Ocultas** | **0.9** | **0.8** | **Muy Viable.** Incentivar subir facturas de categorías donde `count == 0`. | 🟢 VERDE |
| | **Detección Colaborativa**| **1.0** | **0.6** | **Excelente.** Ayuda a limpiar tu data. UI: "¿Qué es este producto 'X'?" -> Usuario etiqueta. | 🟢 VERDE |
| | **Preguntas Relámpago** | **1.0** | **0.7** | **Muy Viable.** Micro-encuestas rápidas. | 🟢 VERDE |
| | **Explorador de Canal** | 0.5 | 0.8 | **Medio.** Distinguir Online vs Físico en XML/OCR a veces es imposible si el formato es idéntico. | ⚠️ AMARILLO |
| | **Coincidencia Silenciosa**| 0.2 | 0.0 | **Inviable.** "Caja negra". Difícil de comunicar y de implementar feedback loop. | ⚫ NEGRO |
| **RIESGO** | **Doble o Nada** | **1.0** | **0.8** | **Muy Viable.** Juego de azar simple con saldo de Lümis. | 🟢 VERDE |
| | **Caja Misteriosa** | **1.0** | **0.9** | **Muy Viable.** Tabla de probabilidades (`loot_tables`). | 🟢 VERDE |
| | **Seguir o Cobrar** | 0.8 | 0.9 | **Viable.** Mecánica "Push your luck". Estado temporal en backend. | 🟢 VERDE |
| | **Riesgo Inverso** | **1.0** | **0.8** | **Muy Viable.** Invertir probabilidad según monto. Matemática simple. | 🟢 VERDE |
| **COMUNIDAD** | **Manada / Clan / Team** | 0.1 | 0.2 | **Inviable MVP.** No encontré grafo social (tablas de amigos/seguidores) en tu DB. Construir esto es un proyecto entero aparte. | 🔴 ROJO |
| | **Eco del Mercado** | 0.3 | 0.4 | **Difícil.** Validar que alguien compartió en IG/TikTok es técnicamente complejo sin APIs costosas. | 🔴 ROJO |
| | **Mega Meta Global** | **1.0** | **0.9** | **Excelente.** Contador global de facturas. Fomenta comunidad sin necesitar grafo social. | 🟢 VERDE |
| **LEYENDA** | **Trono del Barrio** | 0.4 | 0.6 | **Difícil.** Requiere geolocalización precisa de comercios, que suele estar sucia o incompleta. | ⚫ NEGRO |
| | **Museo de Mis Compras** | 0.7 | 0.5 | **Medio.** Generar visualizaciones es trabajo de Frontend, pero requiere mucha data histórica limpia. | ⚠️ AMARILLO |
| | **Saga del Consumidor** | **1.0** | **0.9** | **Muy Viable.** Badges por hitos (Factura #100). Fácil query SQL. | 🟢 VERDE |
| | **Reliquias / Títulos** | 0.9 | 0.8 | **Viable.** Títulos basados en queries (Top 1% comprador de café). | 🟢 VERDE |
| **ORÁCULO** | **Profecía / Termómetro** | **1.0** | **0.8** | **Muy Viable.** Gamificación de encuestas ("Betting" sobre datos). | 🟢 VERDE |
| | **Ecosistema LÜM** | 0.4 | 0.6 | **Complejo.** Mercado de valores ficticio. Mucha lógica de negocio nueva. | ⚠️ AMARILLO |
| **IMPACTO** | **Comercio con Causa** | **1.0** | **0.8** | **Muy Viable.** Redención de Lümis por donación. Ya soportado por sistema de redención. | 🟢 VERDE |
| | **Fondo Comunitario** | **1.0** | **0.8** | **Muy Viable.** Parte del valor de la factura va a un pozo común. | 🟢 VERDE |
| | **Semilla Local** | 0.7 | 0.8 | **Viable.** Requiere identificar PYMES (quizás por tipo de RUC o lista blanca). | ⚠️ AMARILLO |

---

## 🚀 Recomendación MVP (Top 5 Quick Wins)

Estas son las mecánicas que ofrecen el mayor impacto con el menor esfuerzo de desarrollo, aprovechando la infraestructura existente:

1.  **La Chispa Oculta / Caja Misteriosa:**
    *   **Por qué:** Implementación inmediata. Alto impacto dopaminérgico.
    *   **Tech:** Lógica de RNG simple en el backend al procesar la factura.

2.  **Constelación (Streak):**
    *   **Por qué:** Retención pura.
    *   **Tech:** Ya tienes la estructura de datos (`fact_user_streaks`), es solo exponerlo visualmente en el frontend.

3.  **Mega Meta Global:**
    *   **Por qué:** Fomenta comunidad sin necesitar un grafo social complejo (amigos/seguidores).
    *   **Tech:** "Si llegamos a 10,000 facturas entre todos, x2 Lümis mañana". Contador global simple.

4.  **Efecto Dominó:**
    *   **Por qué:** Incentiva el uso recurrente y la exploración de comercios.
    *   **Tech:** "Compra en 3 lugares distintos". Fácil de validar con `COUNT(DISTINCT merchant_id)`.

5.  **Detección Colaborativa:**
    *   **Por qué:** Gamifica la limpieza de tu propia base de datos (Crowdsourcing).
    *   **Tech:** UI: "¿Qué es este producto 'X'?" -> Usuario etiqueta -> Gana Lümis.
