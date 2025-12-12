# 🔐 Guía de Configuración de Variables de Entorno - Seguridad de API Keys

## 📋 Índice
1. [Prevención de Filtración de Secretos](#prevención-de-filtración-de-secretos)
2. [Configuración de OpenRouter API Key](#configuración-de-openrouter-api-key)
3. [Actualización de API Keys en Producción](#actualización-de-api-keys-en-producción)
4. [Prevención Automática con git-secrets](#prevención-automática-con-git-secrets)
5. [Limpieza del Historial de Git](#limpieza-del-historial-de-git)
6. [Variables de Entorno Requeridas](#variables-de-entorno-requeridas)

---

## 🛡️ Prevención de Filtración de Secretos

### ¿Por qué es importante?

GitHub y otros proveedores escanean automáticamente los commits buscando API keys, tokens y credenciales. Cuando detectan una clave filtrada:

1. ✅ Notifican al proveedor del servicio (OpenRouter, AWS, etc.)
2. 🔒 El proveedor **desactiva automáticamente** la clave comprometida
3. 📧 Recibes un correo de notificación de seguridad

### Reglas de Oro

- ❌ **NUNCA** hardcodear API keys en el código fuente
- ❌ **NUNCA** commitear archivos `.env` con valores reales
- ✅ **SIEMPRE** usar variables de entorno
- ✅ **SIEMPRE** verificar `.gitignore` antes de commit
- ✅ **SIEMPRE** revisar los archivos con `git diff` antes de push

---

## 🔑 Configuración de OpenRouter API Key

### Paso 1: Obtener Nueva API Key

1. Ve a [OpenRouter Dashboard](https://openrouter.ai/keys)
2. Genera una nueva API key
3. **Copia la clave inmediatamente** (solo se muestra una vez)
4. Formato: `sk-or-v1-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`

### Paso 2: Configurar en Desarrollo Local

```bash
# Crear archivo .env en el directorio del proyecto (si no existe)
cd /home/client_1099_1/scripts/lum_rust_ws

# Editar .env con tu API key real
nano .env
```

Agregar al archivo `.env`:

```bash
# OpenRouter API Key para OCR
OPENROUTER_API_KEY="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"
```

**⚠️ IMPORTANTE:** El archivo `.env` está en `.gitignore` - nunca será commiteado.

### Paso 3: Verificar .gitignore

Verificar que `.gitignore` contiene:

```bash
cat .gitignore | grep -E "\.env"
```

Debe aparecer:
```
.env
.env.local
```

Si no está, agregarlo:

```bash
echo ".env" >> .gitignore
echo ".env.local" >> .gitignore
```

---

## 🚀 Actualización de API Keys en Producción

### Opción 1: Variable de Entorno del Sistema (Recomendado)

```bash
# SSH al servidor de producción
ssh usuario@servidor-produccion

# Exportar variable de entorno
export OPENROUTER_API_KEY="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"

# Para que persista entre reinicios, agregar a ~/.bashrc o ~/.profile
echo 'export OPENROUTER_API_KEY="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"' >> ~/.bashrc

# Reiniciar el servicio
sudo systemctl restart lum-rust-ws
```

### Opción 2: Archivo .env en Producción

```bash
# Crear .env en servidor con permisos restrictivos
cd /ruta/del/proyecto
nano .env

# Agregar la clave
OPENROUTER_API_KEY="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"

# Restringir permisos (solo lectura para el usuario)
chmod 600 .env
chown usuario:usuario .env

# Reiniciar servicio
sudo systemctl restart lum-rust-ws
```

### Opción 3: Systemd Service con Environment

```bash
# Editar archivo de servicio
sudo nano /etc/systemd/system/lum-rust-ws.service
```

Agregar en la sección `[Service]`:

```ini
[Service]
Environment="OPENROUTER_API_KEY=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"
```

Recargar y reiniciar:

```bash
sudo systemctl daemon-reload
sudo systemctl restart lum-rust-ws
```

### Verificar que la Variable Está Cargada

```bash
# En el servidor, verificar que el proceso la tiene
ps aux | grep lum_rust_ws
cat /proc/$(pgrep lum_rust_ws)/environ | tr '\0' '\n' | grep OPENROUTER
```

---

## 🔒 Prevención Automática con git-secrets

### Instalación

**macOS:**
```bash
brew install git-secrets
```

**Linux (Ubuntu/Debian):**
```bash
# Clonar repositorio
git clone https://github.com/awslabs/git-secrets.git
cd git-secrets
sudo make install
```

### Configuración en el Repositorio

```bash
cd /home/client_1099_1/scripts/lum_rust_ws

# Instalar hooks de git-secrets
git secrets --install

# Registrar patrones comunes (AWS, etc.)
git secrets --register-aws

# Agregar patrón personalizado para OpenRouter
git secrets --add 'sk-or-v1-[A-Za-z0-9]{64}'

# Agregar patrones para otros servicios
git secrets --add 'OPENROUTER_API_KEY\s*=\s*["\']?sk-or-v1-'
git secrets --add 'JWT_SECRET\s*=\s*["\'][^"\']{20,}'
```

### Uso Diario

Ahora, `git-secrets` bloqueará automáticamente commits con API keys:

```bash
git add .
git commit -m "Update config"

# Si hay secretos, verás:
# [ERROR] Matched one or more prohibited patterns
# Aborting commit.
```

### Escanear el Repositorio Completo

```bash
# Escanear todos los archivos trackeados
git secrets --scan

# Escanear historial completo
git secrets --scan-history
```

---

## 🧹 Limpieza del Historial de Git

Si ya commiteaste una API key, **DEBES** limpiar el historial:

### Opción 1: git-filter-repo (Recomendado)

```bash
# Instalar git-filter-repo
pip3 install git-filter-repo

# Crear archivo con texto a reemplazar
cat > secrets.txt << 'EOF'
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==>REDACTED_API_KEY
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==>REDACTED_API_KEY
EOF

# Reemplazar en todo el historial
git filter-repo --replace-text secrets.txt --force

# Force push al repositorio remoto
git push --force origin main
```

### Opción 2: BFG Repo-Cleaner

```bash
# Descargar BFG
wget https://repo1.maven.org/maven2/com/madgag/bfg/1.14.0/bfg-1.14.0.jar

# Crear archivo con secretos a reemplazar
echo "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" > passwords.txt
echo "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" >> passwords.txt

# Limpiar historial
java -jar bfg-1.14.0.jar --replace-text passwords.txt .git

# Limpiar y push
git reflog expire --expire=now --all
git gc --prune=now --aggressive
git push --force origin main
```

### Paso 3: Regenerar API Key

**⚠️ CRÍTICO:** Después de limpiar el historial, la clave antigua ya está comprometida:

1. Ve a OpenRouter Dashboard
2. **Revoca** la clave antigua (si no fue desactivada automáticamente)
3. **Genera** una nueva clave
4. **Actualiza** en `.env` local y en producción

---

## 📝 Variables de Entorno Requeridas

### Lista Completa de Variables

Ver archivo `.env.example` para todas las variables disponibles.

### Variables Críticas para OCR

```bash
# OBLIGATORIO - API Key de OpenRouter
OPENROUTER_API_KEY="sk-or-v1-..."

# Base de datos
DATABASE_URL="postgresql://usuario:password@host:5432/database"

# JWT para autenticación
JWT_SECRET="secret_super_seguro_minimo_32_caracteres"

# Puerto del servidor
SERVER_PORT=8000
```

### Verificar Variables Cargadas

```bash
# Durante desarrollo
cargo run

# Si falta OPENROUTER_API_KEY, verás:
# thread 'main' panicked at 'OPENROUTER_API_KEY must be set in environment variables'
```

---

## ✅ Checklist de Seguridad

Antes de cada commit:

- [ ] No hay API keys hardcodeadas en el código
- [ ] `.env` está en `.gitignore`
- [ ] Ejecuté `git diff --cached` para revisar cambios
- [ ] No hay archivos `.env` en el staging area
- [ ] git-secrets está instalado y configurado
- [ ] Las API keys están solo en variables de entorno

Después de comprometer una clave:

- [ ] Regeneré nueva API key en el proveedor
- [ ] Limpié el historial de Git con git-filter-repo o BFG
- [ ] Hice force push al repositorio remoto
- [ ] Actualicé la nueva clave en desarrollo y producción
- [ ] Verifiqué que la clave antigua fue revocada

---

## 🔗 Referencias

- [OpenRouter Dashboard](https://openrouter.ai/keys)
- [git-secrets GitHub](https://github.com/awslabs/git-secrets)
- [git-filter-repo Docs](https://github.com/newren/git-filter-repo)
- [BFG Repo-Cleaner](https://rtyley.github.io/bfg-repo-cleaner/)

---

## 💡 Consejos Adicionales

### Usar Gestor de Secretos

Para producción empresarial, considera:

- **HashiCorp Vault**: Gestión centralizada de secretos
- **AWS Secrets Manager**: Para despliegues en AWS
- **Google Secret Manager**: Para despliegues en GCP
- **Azure Key Vault**: Para despliegues en Azure

### Rotación Automática de Claves

Configura rotación periódica de API keys:

```bash
# Script de rotación (ejemplo)
#!/bin/bash
# rotate-openrouter-key.sh

# 1. Generar nueva clave vía API (si disponible)
# 2. Actualizar en todos los servidores
# 3. Reiniciar servicios
# 4. Verificar funcionamiento
# 5. Revocar clave antigua
```

### Auditoría de Seguridad

```bash
# Buscar posibles secretos en el repo
git log -p | grep -i "api.*key\|secret\|password" | less

# Verificar archivos sensibles no trackeados
git status --ignored
```

---

**Última actualización:** 2025-12-12  
**Autor:** Sistema de Seguridad Lümis
