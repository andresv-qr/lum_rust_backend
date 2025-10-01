# Corrección: Eliminación del Campo is_active de dim_users

## Problema Identificado
El campo `is_active` no existe en la tabla `public.dim_users`, pero el esquema tenía múltiples referencias a este campo inexistente.

## Cambios Aplicados

### **Funciones de Auto-Asignación**

✅ **`survey.auto_assign_survey_to_existing_users()`**
- **Línea ~659**: `WHERE u.is_active = TRUE;` → Eliminado (conteo de usuarios)
- **Línea ~674**: `WHERE id = v_target_user_id AND is_active = TRUE` → `WHERE id = v_target_user_id`
- **Línea ~711**: `WHERE u.is_active = TRUE AND u.groups ?` → `WHERE u.groups ?`
- **Línea ~729**: `WHERE u.is_active = TRUE AND u.groups ?` → `WHERE u.groups ?`

### **Consultas de Asignación**
✅ **Targeting "todos"**
- Eliminada validación `u.is_active = TRUE` en INSERT SELECT
- Ahora asigna a TODOS los usuarios en dim_users

✅ **Targeting "user_especifico"**  
- Eliminada validación `is_active = TRUE` en verificación de existencia
- Solo valida que el usuario exista: `SELECT 1 FROM public.dim_users WHERE id = v_target_user_id`

✅ **Targeting "grupo_especifico"**
- Eliminada validación `u.is_active = TRUE` en INSERT SELECT
- Eliminada validación `u.is_active = TRUE` en conteo de usuarios elegibles
- Ahora filtra solo por pertenencia al grupo: `WHERE u.groups ? v_target_group`

### **Datos de Prueba**

✅ **Scripts de INSERT**
- **Línea ~982**: Eliminado campo `is_active` de estructura INSERT
- **Línea ~1133**: Eliminado campo `is_active` del comentario INSERT
- Eliminados valores `true/false` para el campo inexistente

## Impacto Funcional

### **Comportamiento Anterior** (Con is_active)
- Solo asignaba encuestas a usuarios "activos" 
- Requería mantenimiento del estado activo/inactivo

### **Comportamiento Actual** (Sin is_active)
- Asigna encuestas a **todos** los usuarios en la tabla
- Simplifica la lógica de asignación
- Compatible con estructura real de dim_users

## Funcionalidades Afectadas

✅ **Auto-asignación "todos"**: Ahora funciona con todos los usuarios de dim_users
✅ **Auto-asignación por usuario**: Solo valida existencia del ID
✅ **Auto-asignación por grupo**: Solo filtra por membresía al grupo
✅ **Conteos y estadísticas**: Reflejan todos los usuarios reales
✅ **Scripts de prueba**: Compatible con estructura real de tabla

## Verificación

- ✅ No quedan referencias a `u.is_active` 
- ✅ No quedan referencias a `dim_users.is_active`
- ✅ Todas las queries funcionan sin el campo inexistente
- ✅ Datos de prueba actualizados correctamente

## Beneficios

1. **Compatibilidad**: Ahora funciona con la estructura real de dim_users
2. **Simplicidad**: Menos validaciones innecesarias  
3. **Inclusión**: Todos los usuarios reciben encuestas asignadas
4. **Mantenimiento**: No requiere gestión de estado activo/inactivo

El esquema ahora es completamente compatible con una tabla `public.dim_users` que no tiene campo `is_active`.
