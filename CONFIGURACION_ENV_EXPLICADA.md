# 🔧 CONFIGURACIÓN: .env vs .env.production

**Fecha**: Octubre 20, 2025  
**Para**: Setup actual de desarrollo vs futuro deployment

---

## 📋 RESUMEN RÁPIDO

| Aspecto | Desarrollo (AHORA) | Producción (FUTURO) |
|---------|-------------------|---------------------|
| **Archivo de config** | `.env` ✅ | `.env.production` → se copia como `.env` |
| **Comando de inicio** | `nohup cargo run --bin lum_rust_ws &` ✅ | `sudo systemctl start lum_rust_ws` |
| **Puerto** | 8000 (definido en `.env`) ✅ | 8000 (mismo) |
| **Servidor** | Tu servidor actual ✅ | `api.lumapp.org` |
| **Base de datos** | dbmain.lumapp.org ✅ | dbmain.lumapp.org (mismo) |

---

## 🎯 RESPUESTAS A TUS PREGUNTAS

### 1️⃣ **¿Se inicia de forma diferente?**

**Tu forma actual (desarrollo)**:
```bash
nohup cargo run --bin lum_rust_ws &
```
- ✅ Compila y ejecuta en modo debug
- ✅ Usa archivo `.env` 
- ✅ Para desarrollo y testing
- ✅ **SIGUE USÁNDOLO ASÍ** ← No cambies nada

**Forma alternativa (más rápida)**:
```bash
# Compilar una sola vez
cargo build --release

# Luego ejecutar el binario (más rápido)
nohup ./target/release/lum_rust_ws &
```
- ✅ No necesita recompilar cada vez
- ✅ Usa menos recursos
- ✅ **También usa archivo `.env`**

**Forma de producción (futuro)**:
```bash
sudo systemctl start lum_rust_ws
```
- ✅ Auto-start al reiniciar servidor
- ✅ Reinicio automático si se cae
- ✅ Solo para cuando despliegues a `api.lumapp.org`

---

### 2️⃣ **¿Se inicia en el mismo puerto o diferente?**

**MISMO puerto: 8000** (definido en tu `.env`):

```bash
# En tu .env actual:
SERVER_PORT=8000
PORT=8000
```

**Todos los métodos usan el puerto que definas en `.env`**:
- `cargo run` → Lee `.env` → Puerto 8000 ✅
- `./target/release/lum_rust_ws` → Lee `.env` → Puerto 8000 ✅
- `systemctl start lum_rust_ws` → Lee `.env` → Puerto 8000 ✅

**Si hay conflicto de puerto**:

```bash
# Ver qué está usando el puerto 8000
sudo lsof -i :8000

# Si hay otro servicio, opciones:
# A) Cambiar puerto en .env a 8001 o 9000
# B) Detener el servicio viejo
```

---

### 3️⃣ **¿Ahora también es .env.production?**

**NO. Sigue usando `.env` para desarrollo**:

```
📁 Tu directorio actual:

.env              ← TU ARCHIVO ACTUAL ✅
                   Úsalo con: nohup cargo run &
                   NO TOCAR, sigue funcionando igual

.env.production   ← TEMPLATE para futuro deployment ⏳
                   Se usa SOLO cuando ejecutes ./deploy_production.sh
                   NO afecta tu desarrollo actual
                   IGNORAR por ahora
```

**¿Qué hace el script de deployment?**

Cuando ejecutes `./deploy_production.sh` (en el FUTURO):
1. Toma `.env.production`
2. Lo copia al servidor de producción como `.env`
3. Inicia el servicio en producción

**Tu desarrollo local NO cambia** ✅

---

### 4️⃣ **¿El JWT_SECRET actualmente se usa para algo?**

**SÍ, es CRÍTICO**. Ya está configurado en tu `.env`:

```bash
# En tu .env actual (línea 61):
JWT_SECRET="lumis_jwt_secret_super_seguro_production_2024_rust_server_key"
```

**¿Para qué se usa?**
- ✅ Validar TODOS los tokens JWT de usuarios
- ✅ Sin él, ningún endpoint protegido funcionaría

**¿Qué endpoints requieren JWT?**

| Endpoint | Requiere JWT | Descripción |
|----------|--------------|-------------|
| `/health` | ❌ NO | Health check público |
| `/api/v1/rewards/balance` | ✅ SÍ | Balance del usuario |
| `/api/v1/rewards/offers` | ✅ SÍ | Listar ofertas |
| `/api/v1/rewards/redeem` | ✅ SÍ | Crear redención |
| `/api/v1/rewards/history` | ✅ SÍ | Historial |
| `/api/v1/merchant/*` | ✅ SÍ | Todos los de merchant |

**⚠️ IMPORTANTE**: Este secret debe ser **EXACTAMENTE EL MISMO** que usa tu API de login existente (el que genera los tokens). Si es diferente, los tokens generados por el login no funcionarán aquí.

**Acción requerida**: ✅ **Ninguna**, ya lo tienes configurado correctamente.

---

### 5️⃣ **¿FCM_SERVER_KEY podemos no usarlo por ahora?**

**SÍ, es totalmente OPCIONAL**:

```bash
# En tu .env actual:
# NO ESTÁ CONFIGURADO ✅ (y está bien)
```

**¿Qué hace FCM_SERVER_KEY?**
- Envía push notifications a los usuarios cuando:
  - ✅ Se crea una redención → "Nueva redención creada"
  - ✅ Se confirma una redención → "¡Redención confirmada!"
  - ✅ Una redención está por expirar → "Expira en 5 minutos"

**¿Qué pasa si no lo configuras?**
- ✅ El servidor arranca normalmente
- ✅ Las redenciones se crean y funcionan perfectamente
- ✅ Todo el sistema funciona EXCEPTO push notifications
- ⚠️ Los usuarios NO recibirán notificaciones

**¿Cómo obtenerlo (cuando lo necesites)?**
1. Ir a Firebase Console
2. Project Settings
3. Cloud Messaging
4. Copiar "Server Key"

**Acción recomendada**: ✅ **Déjalo sin configurar por ahora**. Puedes agregarlo después cuando quieras habilitar notificaciones push.

---

### 6️⃣ **¿REDIS_URL está en .env y no en .env.production?**

**Está en AMBOS** (y debe estar):

```bash
# En tu .env actual (línea 4):
REDIS_URL="redis://127.0.0.1/"  ✅

# En .env.production (actualizado ahora):
REDIS_URL=redis://127.0.0.1/  ✅
```

**¿Para qué se usa Redis?**
- ✅ Rate limiting (limitar redenciones por día)
- ✅ Caching de datos frecuentes
- ✅ Optimización de performance

**¿Redis está corriendo?**

```bash
# Verificar
redis-cli ping
# Debe retornar: PONG ✅
```

Si no está instalado:
```bash
sudo apt-get install redis-server
sudo systemctl start redis-server
```

**Acción requerida**: ✅ **Ninguna**, ya está configurado correctamente.

---

## 📊 TU CONFIGURACIÓN ACTUAL (Todo OK ✅)

### Archivo: `.env` (en uso ahora)

```bash
# ✅ Base de datos
DATABASE_URL=postgres://avalencia:Jacobo23@dbmain.lumapp.org:5432/tfactu?sslmode=require

# ✅ Redis
REDIS_URL=redis://127.0.0.1/

# ✅ JWT (CRÍTICO)
JWT_SECRET=lumis_jwt_secret_super_seguro_production_2024_rust_server_key

# ✅ Puerto
SERVER_PORT=8000

# ✅ Logging
RUST_LOG=info,lum_rust_ws=debug

# ❌ FCM (opcional, no configurado - OK)
# FCM_SERVER_KEY=... (no lo necesitas por ahora)
```

**Estado**: ✅ **TODO CONFIGURADO CORRECTAMENTE**

---

## 🚀 FLUJO COMPLETO

### **AHORA (Desarrollo)**:

```bash
# 1. Iniciar servidor (forma actual)
nohup cargo run --bin lum_rust_ws &

# O alternativa más rápida:
nohup ./target/release/lum_rust_ws &

# 2. Verificar que funciona
curl http://localhost:8000/health

# 3. Archivo de configuración usado
.env  ← Este archivo

# 4. Variables necesarias (ya las tienes)
DATABASE_URL  ✅
REDIS_URL     ✅
JWT_SECRET    ✅
SERVER_PORT   ✅
```

### **FUTURO (Cuando despliegues a producción)**:

```bash
# 1. Editar .env.production (ya está listo)
# (Ya tiene los mismos valores que tu .env actual)

# 2. Ejecutar deployment automatizado
./deploy_production.sh

# 3. El script automáticamente:
#    - Copia .env.production como .env en producción
#    - Sube el binario a api.lumapp.org
#    - Configura systemd service
#    - Inicia el servicio

# 4. Verificar en producción
curl https://api.lumapp.org/health
```

---

## ⚙️ COMPARACIÓN DE ARCHIVOS

### Tu `.env` actual (desarrollo):

```properties
DATABASE_URL=postgres://avalencia:Jacobo23@dbmain.lumapp.org:5432/tfactu?sslmode=require
REDIS_URL=redis://127.0.0.1/
JWT_SECRET=lumis_jwt_secret_super_seguro_production_2024_rust_server_key
SERVER_PORT=8000
RUST_LOG=info,lum_rust_ws=debug
ENVIRONMENT=production
# ... más variables de tu sistema actual
```

### `.env.production` (template para futuro):

```properties
# ⚠️ Este archivo es SOLO para deployment a api.lumapp.org
# Para desarrollo, usa .env (sin .production)

DATABASE_URL=postgres://avalencia:Jacobo23@dbmain.lumapp.org:5432/tfactu?sslmode=require
REDIS_URL=redis://127.0.0.1/
JWT_SECRET=lumis_jwt_secret_super_seguro_production_2024_rust_server_key
PORT=8000
SERVER_PORT=8000
RUST_LOG=info,lum_rust_ws=debug
ENVIRONMENT=production

# FCM_SERVER_KEY=... (opcional, comentado)
```

**Son casi idénticos** ✅ La única diferencia es el comentario de que `.env.production` es para deployment futuro.

---

## ✅ CHECKLIST FINAL

### Para Desarrollo (AHORA):

- [x] Archivo `.env` configurado correctamente
- [x] `DATABASE_URL` apunta a dbmain.lumapp.org
- [x] `REDIS_URL` configurado (127.0.0.1)
- [x] `JWT_SECRET` configurado (debe coincidir con API login)
- [x] `SERVER_PORT=8000` definido
- [x] `FCM_SERVER_KEY` NO configurado (opcional, OK)
- [x] Servidor inicia con `nohup cargo run &`
- [x] **NO NECESITAS TOCAR .env.production**

### Para Producción (FUTURO):

- [ ] Editar `.env.production` si algo cambia (ya está listo)
- [ ] Ejecutar `./deploy_production.sh`
- [ ] Verificar health check en `api.lumapp.org`
- [ ] Configurar Nginx como proxy reverso (si aplica)

---

## 🎯 RESUMEN DE 3 PUNTOS

1. **Tu desarrollo NO cambia**: Sigue usando `nohup cargo run &` con archivo `.env` ✅

2. **`.env.production` es solo para futuro**: Cuando ejecutes `./deploy_production.sh` para mover el sistema a `api.lumapp.org` ⏳

3. **FCM_SERVER_KEY es opcional**: El sistema funciona sin push notifications. Agrégalo después si lo necesitas 📱

---

## 📞 NECESITAS AYUDA?

Si algo no funciona:

```bash
# Ver logs del servidor
tail -f nohup.out

# Ver procesos
ps aux | grep lum_rust_ws

# Verificar puerto
sudo lsof -i :8000

# Test health
curl http://localhost:8000/health
```

---

**Última actualización**: Octubre 20, 2025  
**Estado**: ✅ Tu configuración actual está correcta, no necesitas cambiar nada
