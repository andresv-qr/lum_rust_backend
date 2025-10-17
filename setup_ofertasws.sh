#!/bin/bash
# Setup script para API de OfertasWs

echo "🚀 Setup API de OfertasWs - Lüm App"
echo "=================================="
echo ""

# 1. Verificar .env existe
if [ ! -f .env ]; then
    echo "❌ Error: .env no encontrado"
    exit 1
fi

# 2. Agregar WS_DATABASE_URL si no existe
if ! grep -q "WS_DATABASE_URL" .env; then
    echo "📝 Agregando WS_DATABASE_URL a .env..."
    echo "" >> .env
    echo "# Base de datos WS para ofertasws" >> .env
    echo "WS_DATABASE_URL=postgresql://avalencia:Jacobo23@dbws.lumapp.org/ws" >> .env
    echo "✅ WS_DATABASE_URL agregado"
else
    echo "✅ WS_DATABASE_URL ya existe en .env"
fi

# 3. Verificar Redis
echo ""
echo "🔍 Verificando Redis..."
if redis-cli PING > /dev/null 2>&1; then
    echo "✅ Redis está corriendo"
else
    echo "⚠️ Redis no responde. Intentando iniciar..."
    sudo systemctl start redis 2>/dev/null || echo "❌ No se pudo iniciar Redis automáticamente"
fi

# 4. Ejecutar migración SQL
echo ""
echo "🗄️ Ejecutando migración SQL..."
echo "Nota: Deberás ingresar la contraseña de PostgreSQL manualmente"
echo ""

PGPASSWORD="Jacobo23" psql -h dbws.lumapp.org -U avalencia -d ws -f ofertasws_refresh_log.sql

if [ $? -eq 0 ]; then
    echo "✅ Migración SQL ejecutada correctamente"
else
    echo "⚠️ Hubo un problema con la migración SQL"
    echo "   Puedes ejecutarla manualmente:"
    echo "   psql -h dbws.lumapp.org -U avalencia -d ws -f ofertasws_refresh_log.sql"
fi

# 5. Build
echo ""
echo "🔨 Compilando proyecto..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Compilación exitosa"
else
    echo "❌ Error en compilación"
    exit 1
fi

# 6. Resumen
echo ""
echo "✅ Setup completado!"
echo ""
echo "📋 Próximos pasos:"
echo "   1. Detener servidor actual: kill -TERM \$(ps aux | grep lum_rust_ws | grep -v grep | awk '{print \$2}')"
echo "   2. Iniciar nueva versión: nohup ./target/release/lum_rust_ws > nohup_ofertasws.out 2>&1 &"
echo "   3. Verificar logs: tail -f nohup_ofertasws.out"
echo "   4. Test endpoint: curl http://localhost:8000/api/v4/ofertasws -H \"Authorization: Bearer \$TOKEN\""
echo ""
echo "📖 Documentación completa: OFERTAS_API_DOCUMENTATION.md"
