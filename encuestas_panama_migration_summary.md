# Adaptación de Encuestas de Panamá al Nuevo Esquema Survey

## Archivo Adaptado
**Original**: `load_encuestas_panama.sql`  
**Nuevo**: `load_encuestas_panama_survey.sql`

## Cambios Principales Aplicados

### **1. Migración de Esquema**
✅ **Tablas**: `trivia.` → `survey.`
- `trivia.dim_campaigns` → `survey.dim_campaigns`
- `trivia.dim_surveys` → `survey.dim_surveys`
- `trivia.fact_user_survey_status` → `survey.fact_user_survey_status`

✅ **Funciones**: `trivia.` → `survey.`
- `trivia.api_assign_survey` → `survey.api_assign_survey`
- `trivia.api_get_survey_details` → `survey.api_get_survey_details`
- etc.

### **2. Nuevos Campos de Targeting**

Agregados a todas las 4 encuestas:
```sql
target_audience,  -- Nuevo campo
auto_assign,      -- Nuevo campo
is_active
```

**Valores configurados**:
- `target_audience = 'todos'` ← **Aplican para todos los usuarios**
- `auto_assign = TRUE` ← **Se asignan automáticamente**
- `is_active = TRUE` ← **Encuestas activas**

### **3. Estructura de las 4 Encuestas**

✅ **Encuesta 1: Hábitos Alimenticios y de Consumo**
- 10 preguntas sobre alimentación y consumo
- Target: todos los usuarios
- Auto-asignación: habilitada

✅ **Encuesta 2: Preferencias y Experiencias en Citas**  
- 10 preguntas sobre comportamiento social/romántico
- Target: todos los usuarios
- Auto-asignación: habilitada

✅ **Encuesta 3: Estilo de Vida y Perfil Socioeconómico**
- 10 preguntas sobre demografía y estilo de vida
- Target: todos los usuarios  
- Auto-asignación: habilitada

✅ **Encuesta 4: Top of Mind - Marcas y Preferencias en Panamá**
- 10 preguntas sobre marcas y preferencias comerciales
- Target: todos los usuarios
- Auto-asignación: habilitada

### **4. Funcionalidad de Auto-Asignación**

**Comportamiento automático**:
1. **Al crear las encuestas**: El trigger `trigger_auto_assign_new_survey` se ejecuta automáticamente
2. **Asigna a usuarios existentes**: Todas las encuestas se asignan a todos los usuarios en `public.dim_users`
3. **Para nuevos usuarios**: El sistema asigna automáticamente al registrarse

**No requiere asignación manual** - todo es automático por el targeting `'todos'`

### **5. Instrucciones Actualizadas**

✅ **Automatización**:
- Ya no es necesario asignar encuestas manualmente
- Los triggers se encargan de todo
- Compatible con registro de nuevos usuarios

✅ **Funciones disponibles**:
- `survey.api_auto_assign_surveys()` - Para forzar re-asignación
- `survey.api_auto_assign_surveys_async()` - Para nuevos usuarios
- `survey.api_get_user_surveys()` - Para obtener encuestas del usuario

✅ **Consultas útiles**:
- Ver encuestas pendientes/completadas por usuario
- Estadísticas de asignación y completitud
- Detalles de encuestas específicas

## Beneficios de la Adaptación

1. **Automatización completa**: No requiere intervención manual
2. **Escalabilidad**: Funciona para cualquier cantidad de usuarios
3. **Consistencia**: Todos los usuarios reciben las mismas encuestas
4. **Flexibilidad**: Fácil modificar targeting en el futuro
5. **Trazabilidad**: Control completo del estado de cada encuesta por usuario

## Uso del Archivo

1. **Ejecutar**: `psql -f load_encuestas_panama_survey.sql`
2. **Verificar**: Las 4 encuestas se crean y asignan automáticamente
3. **Validar**: Todos los usuarios activos tendrán las 4 encuestas pendientes

El archivo está listo para uso en producción con el nuevo esquema `survey`.
