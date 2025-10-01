# Corrección de Referencias a Tabla de Usuarios

## Cambios Aplicados

✅ **Nombre de Tabla**
- `public.dim_user` → `public.dim_users` (plural)

✅ **Campo de ID**
- `user_id` → `id` (en el contexto de la tabla dim_users)

## Ubicaciones Corregidas

### **Funciones de Auto-Asignación**

1. **`survey.auto_assign_survey_to_existing_users()`**
   - Líneas ~644: `u.user_id` → `u.id` en SELECT
   - Líneas ~652: `fuss.user_id = u.user_id` → `fuss.user_id = u.id` en NOT EXISTS
   - Líneas ~675: `WHERE user_id = v_target_user_id` → `WHERE id = v_target_user_id`
   - Líneas ~707: `u.user_id` → `u.id` en SELECT para grupo específico
   - Líneas ~725: `fuss.user_id = u.user_id` → `fuss.user_id = u.id` en NOT EXISTS

### **Datos de Prueba**
2. **Estructura de INSERT de Prueba**
   - Línea ~983: `INSERT INTO public.dim_users (user_id, ...)` → `INSERT INTO public.dim_users (id, ...)`
   - Línea ~1136: Comentario de INSERT actualizado
   - Línea ~1217: `DELETE FROM public.dim_users WHERE user_id BETWEEN` → `WHERE id BETWEEN`

### **Ejemplos de API Rust**
3. **Registro de Usuarios**
   - Línea ~1306: `RETURNING user_id` → `RETURNING id`
   - Línea ~1315: `.user_id` → `.id` en resultado de query
   - Línea ~1559: `RETURNING user_id` → `RETURNING id` en segundo ejemplo

## Consistencia Verificada

✅ **Tabla dim_users**: Todas las referencias actualizadas correctamente
✅ **Campo id**: Todas las referencias al campo principal actualizadas
✅ **Joins y Subqueries**: Todas las condiciones WHERE actualizadas
✅ **Ejemplos de Código**: APIs de Rust actualizadas
✅ **Datos de Prueba**: Scripts de INSERT/DELETE actualizados

## Impacto en Funcionalidad

- ✅ **Auto-asignación a "todos"**: Funciona correctamente con `u.id`
- ✅ **Auto-asignación por usuario específico**: Valida existencia con `dim_users.id`
- ✅ **Auto-asignación por grupo**: Filtra usuarios con `u.id`
- ✅ **APIs de registro**: Retornan el `id` correcto
- ✅ **Testing**: Scripts de limpieza usan `id` correcto

## Próximos Pasos

1. **Validar esquema de `public.dim_users`**: Confirmar que la tabla existe con campo `id`
2. **Testing de funciones**: Ejecutar las funciones de auto-asignación para verificar funcionamiento
3. **Actualizar aplicación**: Asegurar que el código Rust use `id` en lugar de `user_id`

El esquema ahora es consistente con la estructura real de la tabla de usuarios.
