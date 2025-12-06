# 🚀 Guía de Deployment a Producción - Lum Rust WS

**Versión**: 1.0  
**Fecha**: Octubre 20, 2025  
**Sistema**: Lum Rust Web Service (Rewards & Redemptions API)

---

## 📋 Tabla de Contenidos

1. [Pre-requisitos](#pre-requisitos)
2. [Checklist Pre-Deployment](#checklist-pre-deployment)
3. [Deployment Manual](#deployment-manual)
4. [Deployment Automatizado](#deployment-automatizado)
5. [Verificación Post-Deployment](#verificación-post-deployment)
6. [Troubleshooting](#troubleshooting)
7. [Rollback](#rollback)
8. [Monitoring](#monitoring)

---

## ✅ PRE-REQUISITOS

### En el Servidor de Desarrollo (Local)

- [x] Rust 1.81+ instalado
- [x] Código compilando sin errores ni warnings
- [x] Tests pasando (si aplica)
- [x] `.env.production` configurado con valores reales

### En el Servidor de Producción

**Software requerido**:
```bash
# PostgreSQL
sudo apt-get install postgresql-client

# Redis (para rate limiting)
sudo apt-get install redis-server
sudo systemctl enable redis-server
sudo systemctl start redis-server

# SSL libraries (ya deberían estar)
sudo apt-get install libssl-dev pkg-config

# Opcional: herramientas de monitoring
sudo apt-get install htop curl jq
```

**Configuración de red**:
- Puerto 8000 accesible (o el que configures)
- Firewall permitiendo tráfico HTTP/HTTPS
- DNS apuntando a `api.lumapp.org`

**Permisos**:
- Usuario con acceso SSH al servidor
- Permisos sudo para instalar systemd service

---

## 📝 CHECKLIST PRE-DEPLOYMENT

### 1. Verificar Binario

```bash
# En local
cd /home/client_1099_1/scripts/lum_rust_ws

# Compilar en release mode
cargo build --release

# Verificar tamaño (debe ser ~66MB)
ls -lh target/release/lum_rust_ws

# Verificar que funciona
./target/release/lum_rust_ws &
sleep 5
curl http://localhost:8000/health
# Debe retornar: {"service":"lum_rust_ws","status":"healthy",...}
pkill lum_rust_ws
```

### 2. Configurar Variables de Entorno

```bash
# Copiar template
cp .env.production .env.production.bak

# Editar con valores reales
nano .env.production
```

**Variables CRÍTICAS a configurar**:

| Variable | Descripción | Dónde Obtener |
|----------|-------------|---------------|
| `DATABASE_URL` | Conexión PostgreSQL | DBA o archivo de config existente |
| `JWT_SECRET` | Secret para validar tokens | **DEBE SER EL MISMO** que usa el API de login |
| `FCM_SERVER_KEY` | Firebase Cloud Messaging | Firebase Console > Cloud Messaging |
| `REDIS_URL` | Conexión Redis | Servidor local o remoto |

**Validar que NO hay placeholders**:
```bash
grep -i "REEMPLAZAR" .env.production
# No debe retornar nada
```

### 3. Backup de Base de Datos (Recomendado)

```bash
# Desde el servidor de BD o con acceso remoto
pg_dump -h dbmain.lumapp.org -U postgres -d tfactu \
  --schema=rewards \
  > backup_rewards_$(date +%Y%m%d_%H%M%S).sql
```

### 4. Verificar Acceso SSH

```bash
ssh root@api.lumapp.org
# Si funciona, estás listo
```

---

## 🛠️ DEPLOYMENT MANUAL

### Paso 1: Crear Directorio en Producción

```bash
ssh root@api.lumapp.org

# Crear directorio
sudo mkdir -p /opt/lum_rust_ws/backups
sudo mkdir -p /opt/lum_rust_ws/logs

# Ajustar permisos
sudo chown -R client_1099_1:client_1099_1 /opt/lum_rust_ws
```

### Paso 2: Subir Binario

```bash
# En local
cd /home/client_1099_1/scripts/lum_rust_ws

# Comprimir binario
tar -czf lum_rust_ws_deploy.tar.gz \
  -C target/release lum_rust_ws

# Subir a producción
scp lum_rust_ws_deploy.tar.gz root@api.lumapp.org:/opt/lum_rust_ws/
scp .env.production root@api.lumapp.org:/opt/lum_rust_ws/.env
scp lum_rust_ws.service root@api.lumapp.org:/opt/lum_rust_ws/
```

### Paso 3: Extraer y Configurar

```bash
ssh root@api.lumapp.org

cd /opt/lum_rust_ws

# Extraer binario
tar -xzf lum_rust_ws_deploy.tar.gz

# Permisos
chmod +x lum_rust_ws
chown client_1099_1:client_1099_1 lum_rust_ws .env

# Verificar
./lum_rust_ws --help || echo "Binary OK"
```

### Paso 4: Instalar Systemd Service

```bash
# Copiar service file
sudo cp lum_rust_ws.service /etc/systemd/system/

# Recargar systemd
sudo systemctl daemon-reload

# Habilitar para auto-start
sudo systemctl enable lum_rust_ws

# Ver status
sudo systemctl status lum_rust_ws
```

### Paso 5: Detener Servicio Viejo (Si Aplica)

```bash
# Si existe qreader_api u otro servicio en el mismo puerto
sudo systemctl stop qreader_api
sudo systemctl disable qreader_api
```

### Paso 6: Iniciar Nuevo Servicio

```bash
# Iniciar
sudo systemctl start lum_rust_ws

# Verificar que está corriendo
sudo systemctl status lum_rust_ws

# Ver logs en vivo
sudo journalctl -u lum_rust_ws -f
```

### Paso 7: Configurar Proxy Reverso (Nginx)

Si usas Nginx como proxy reverso:

```bash
sudo nano /etc/nginx/sites-available/api.lumapp.org
```

Agregar o modificar:

```nginx
server {
    listen 443 ssl http2;
    server_name api.lumapp.org;

    ssl_certificate /etc/letsencrypt/live/api.lumapp.org/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.lumapp.org/privkey.pem;

    # Rewards API (nuevo)
    location /api/v1/rewards/ {
        proxy_pass http://localhost:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }

    # Merchant API (nuevo)
    location /api/v1/merchant/ {
        proxy_pass http://localhost:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }

    # Health check
    location /health {
        proxy_pass http://localhost:8000;
        access_log off;
    }

    # Metrics (proteger con auth si es público)
    location /monitoring/metrics {
        proxy_pass http://localhost:8000;
        # auth_basic "Restricted";
        # auth_basic_user_file /etc/nginx/.htpasswd;
    }

    # Otras rutas existentes (qreader, etc.)
    location / {
        proxy_pass http://localhost:5000;  # O tu servidor existente
    }
}
```

Reiniciar Nginx:

```bash
sudo nginx -t  # Verificar sintaxis
sudo systemctl reload nginx
```

---

## 🤖 DEPLOYMENT AUTOMATIZADO

Para deployment más rápido, usa el script automatizado:

### 1. Configurar el Script

```bash
nano deploy_production.sh
```

Editar variables al inicio:

```bash
PRODUCTION_SERVER="api.lumapp.org"
PRODUCTION_USER="root"
PRODUCTION_PATH="/opt/lum_rust_ws"
SERVICE_NAME="lum_rust_ws"
```

### 2. Ejecutar Deployment

```bash
# Asegúrate de estar en el directorio correcto
cd /home/client_1099_1/scripts/lum_rust_ws

# Ejecutar script
./deploy_production.sh
```

El script hará automáticamente:
1. ✅ Verificaciones pre-deployment
2. ✅ Crear package de deployment
3. ✅ Backup del deployment anterior
4. ✅ Subir archivos al servidor
5. ✅ Instalar systemd service
6. ✅ Reiniciar servicio
7. ✅ Health checks

---

## ✅ VERIFICACIÓN POST-DEPLOYMENT

### 1. Health Check

```bash
curl https://api.lumapp.org/health
```

**Esperado**:
```json
{
  "service": "lum_rust_ws",
  "status": "healthy",
  "timestamp": "2025-10-20T..."
}
```

### 2. Test Endpoints con Token Real

Genera un JWT token válido:

```bash
# Usar token de un usuario real o generar uno de prueba
TOKEN="eyJ0eXAiOiJKV1QiLCJhbGc..."

# Test balance
curl https://api.lumapp.org/api/v1/rewards/balance \
  -H "Authorization: Bearer $TOKEN"

# Test ofertas
curl https://api.lumapp.org/api/v1/rewards/offers \
  -H "Authorization: Bearer $TOKEN"
```

### 3. Verificar Conexión a Base de Datos

```bash
# Ver logs para confirmar conexión
ssh root@api.lumapp.org
sudo journalctl -u lum_rust_ws -n 100 | grep -i "database\|pool"
```

Debe mostrar:
```
[INFO] Database connection pool initialized
[INFO] Connected to PostgreSQL at dbmain.lumapp.org
```

### 4. Verificar Redis

```bash
# En el servidor
redis-cli ping
# Debe retornar: PONG

# Verificar en logs del servicio
sudo journalctl -u lum_rust_ws -n 50 | grep -i redis
```

### 5. Test Push Notifications (Opcional)

Si configuraste `FCM_SERVER_KEY`, prueba las notificaciones:

```bash
# Crear una redención de prueba y verificar que llegue push
curl -X POST https://api.lumapp.org/api/v1/rewards/redeem \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "offer_id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": 1
  }'
```

### 6. Verificar Métricas

```bash
curl https://api.lumapp.org/monitoring/metrics | grep redemptions
```

Deberías ver métricas Prometheus:
```
redemptions_created_total 0
redemptions_confirmed_total 0
balance_queries_total 0
```

---

## 🔧 TROUBLESHOOTING

### Problema: Servicio No Inicia

**Síntoma**:
```bash
sudo systemctl status lum_rust_ws
# Status: failed
```

**Diagnóstico**:
```bash
# Ver logs completos
sudo journalctl -u lum_rust_ws -n 100 --no-pager

# Verificar errores de configuración
sudo journalctl -u lum_rust_ws | grep -i error
```

**Causas comunes**:

1. **Error de conexión a base de datos**:
   ```
   Error: Failed to connect to database
   ```
   Solución: Verificar `DATABASE_URL` en `.env`

2. **Puerto ya en uso**:
   ```
   Error: Address already in use (os error 98)
   ```
   Solución: 
   ```bash
   sudo lsof -i :8000
   # Matar proceso o cambiar puerto en .env
   ```

3. **Permisos incorrectos**:
   ```
   Permission denied
   ```
   Solución:
   ```bash
   sudo chown -R client_1099_1:client_1099_1 /opt/lum_rust_ws
   chmod +x /opt/lum_rust_ws/lum_rust_ws
   ```

### Problema: 404 en Endpoints

**Síntoma**:
```bash
curl https://api.lumapp.org/api/v1/rewards/offers
# {"detail":"Not Found"}
```

**Diagnóstico**:
```bash
# Verificar que el servicio está corriendo
curl http://localhost:8000/health
# Si funciona en local pero no público, es problema de proxy
```

**Solución**: Verificar configuración de Nginx (ver sección de proxy reverso arriba)

### Problema: Token Inválido

**Síntoma**:
```json
{"error":"Invalid token","message":"Could not validate credentials"}
```

**Causa**: `JWT_SECRET` en `.env` es diferente al usado por el API de login

**Solución**:
1. Obtener el `JWT_SECRET` correcto del API de login
2. Actualizar `.env`:
   ```bash
   nano /opt/lum_rust_ws/.env
   # Cambiar JWT_SECRET
   ```
3. Reiniciar servicio:
   ```bash
   sudo systemctl restart lum_rust_ws
   ```

### Problema: Push Notifications No Funcionan

**Síntoma**: Redenciones se crean pero no llegan notificaciones

**Diagnóstico**:
```bash
sudo journalctl -u lum_rust_ws | grep -i "fcm\|notification"
```

**Causas comunes**:
1. `FCM_SERVER_KEY` incorrecto o vacío
2. Usuario no tiene FCM token registrado
3. Firebase Cloud Messaging deshabilitado

**Solución**:
```bash
# Verificar variable de entorno
cat /opt/lum_rust_ws/.env | grep FCM_SERVER_KEY

# Reiniciar con nueva key
sudo systemctl restart lum_rust_ws
```

---

## ⏪ ROLLBACK

Si algo sale mal, puedes hacer rollback al deployment anterior:

### Opción 1: Rollback Automático (con backups)

```bash
ssh root@api.lumapp.org

cd /opt/lum_rust_ws/backups

# Listar backups disponibles
ls -lt backup_*.tar.gz

# Restaurar el más reciente
LATEST_BACKUP=$(ls -t backup_*.tar.gz | head -1)
echo "Restoring: $LATEST_BACKUP"

# Detener servicio actual
sudo systemctl stop lum_rust_ws

# Extraer backup
tar -xzf "$LATEST_BACKUP" -C /opt/lum_rust_ws

# Reiniciar
sudo systemctl start lum_rust_ws

# Verificar
sudo systemctl status lum_rust_ws
curl http://localhost:8000/health
```

### Opción 2: Rollback Manual (revertir a qreader_api)

```bash
# Detener lum_rust_ws
sudo systemctl stop lum_rust_ws
sudo systemctl disable lum_rust_ws

# Iniciar servicio viejo
sudo systemctl enable qreader_api
sudo systemctl start qreader_api

# Verificar
curl https://api.lumapp.org/health
```

---

## 📊 MONITORING

### Ver Logs en Tiempo Real

```bash
# Logs del servicio
sudo journalctl -u lum_rust_ws -f

# Filtrar solo errores
sudo journalctl -u lum_rust_ws -p err -f

# Últimas 100 líneas
sudo journalctl -u lum_rust_ws -n 100 --no-pager
```

### Métricas Prometheus

```bash
# Todas las métricas
curl -s https://api.lumapp.org/monitoring/metrics

# Solo redenciones
curl -s https://api.lumapp.org/monitoring/metrics | grep redemptions

# Queries de balance
curl -s https://api.lumapp.org/monitoring/metrics | grep balance_queries
```

### Status del Sistema

```bash
# CPU y memoria
htop

# Uso de disco
df -h /opt/lum_rust_ws

# Conexiones activas
ss -tulpn | grep :8000

# Estado del servicio
systemctl status lum_rust_ws
```

### Alertas Recomendadas

Configurar alertas para:
- ❌ Servicio caído (`systemctl status lum_rust_ws` != active)
- ⚠️ Uso de memoria > 80%
- ⚠️ Errores de base de datos (logs contienen "database error")
- ⚠️ Rate limit alcanzado frecuentemente

---

## 📝 CHECKLIST FINAL

Después del deployment, verificar:

- [ ] Servicio corriendo: `sudo systemctl status lum_rust_ws`
- [ ] Health check: `curl https://api.lumapp.org/health`
- [ ] Endpoint de ofertas responde: `curl https://api.lumapp.org/api/v1/rewards/offers`
- [ ] Token válido funciona (401 con token inválido es OK)
- [ ] Base de datos conectada (revisar logs)
- [ ] Redis conectado (revisar logs)
- [ ] Push notifications configuradas (revisar `FCM_SERVER_KEY`)
- [ ] Métricas accesibles: `curl https://api.lumapp.org/monitoring/metrics`
- [ ] Logs limpios (sin errores críticos)
- [ ] Frontend puede hacer llamadas correctamente

---

## 🆘 CONTACTO DE SOPORTE

**Backend Team**:
- Email: backend@lumapp.org
- Slack: #lum-deployment

**Emergencias**:
- On-call: +507-XXXX-XXXX
- PagerDuty: lumapp-api

---

**Última actualización**: Octubre 20, 2025  
**Mantenido por**: Equipo DevOps & Backend Lüm
