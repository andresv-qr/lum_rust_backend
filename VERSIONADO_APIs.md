# ✅ VERSIONADO DE APIs - ACLARACIÓN

**Fecha**: 19 de octubre, 2024

---

## 📌 Estructura de Versionado

La aplicación Lüm tiene **diferentes versiones de API** para diferentes módulos:

### `/api/v4/` - Core de la Aplicación
```
Endpoints principales:
├── /api/v4/users          → Usuarios y autenticación
├── /api/v4/invoices       → Procesamiento de facturas
├── /api/v4/lumis_balance  → Balance de lümis general
├── /api/v4/qr_processing  → Detección de QR
├── /api/v4/rewards        → Recompensas (no confundir con redenciones)
├── /api/v4/daily-game     → Juego diario (constelaciones)
├── /api/v4/surveys        → Encuestas
└── /api/v4/gamification   → Sistema de gamificación
```

**Version**: 4  
**Razón**: Evolución del sistema original a través de 4 iteraciones  
**Status**: Activa y estable

---

### `/api/v1/` - Sistema de Redenciones (NUEVO)
```
Endpoints de redenciones:
├── /api/v1/rewards/      → APIs para usuarios
│   ├── balance           → Consultar balance
│   ├── offers            → Listar ofertas
│   ├── redeem            → Crear redención
│   ├── history           → Historial
│   ├── redemptions/:id   → Detalle
│   ├── cancel            → Cancelar
│   └── accumulations     → Ver acumulaciones
│
└── /api/v1/merchant/     → APIs para merchants
    ├── pending           → Redenciones pendientes
    ├── validate/:id      → Validar código
    ├── confirm/:id       → Confirmar redención
    ├── reject/:id        → Rechazar redención
    └── analytics         → Dashboard analítico
```

**Version**: 1  
**Razón**: Es un módulo **completamente nuevo**, implementado en octubre 2024  
**Status**: Activa, recién lanzada

---

## ❓ ¿Por Qué v1 y No v4?

### Razones Técnicas:

1. **Módulo Independiente**
   - Sistema de redenciones es completamente nuevo
   - No tiene versiones previas (no hubo v1, v2, v3)
   - Lógica independiente del core v4

2. **Versionado Semántico**
   - v1 = Primera versión del módulo de redenciones
   - v4 = Cuarta versión del core de la app
   - Son **líneas evolutivas diferentes**

3. **Facilita Mantenimiento**
   - Cambios en redenciones no afectan core v4
   - Cambios en core v4 no afectan redenciones v1
   - Puedes deprecar v1 y lanzar v2 sin afectar v4

4. **Claridad para Frontend**
   - `/api/v4/...` → Funciones existentes de la app
   - `/api/v1/rewards/...` → Sistema de redenciones nuevo
   - No hay confusión sobre qué es qué

---

## 📊 Comparación

### Ejemplo 1: Balance de Lümis

**Endpoint v4** (general):
```
GET /api/v4/lumis_balance
Respuesta: { "user_id": 123, "balance": 500, "breakdown": {...} }
Propósito: Balance general de lümis del usuario
```

**Endpoint v1** (redenciones):
```
GET /api/v1/rewards/balance
Respuesta: { "user_id": 123, "balance_lumis": 450, "balance_points": 150 }
Propósito: Balance específico para sistema de redenciones
```

Ambos coexisten porque:
- v4 es para el balance general de toda la app
- v1 es específico para redenciones con detalles de puntos canjeables

### Ejemplo 2: Ofertas

**Endpoint v4**:
```
GET /api/v4/ofertasws
Respuesta: Ofertas de WS (base de datos externa)
```

**Endpoint v1**:
```
GET /api/v1/rewards/offers
Respuesta: Ofertas de redención de Lümis
```

Son **sistemas diferentes** con propósitos diferentes.

---

## ✅ Documentación Correcta

### Frontend Docs
```
✅ docs/DOCUMENTACION_FRONTEND_USUARIOS.md
   Base URL: https://api.lumapp.org/api/v1
   
   Correcto porque documenta el sistema de redenciones (v1)
```

### Otros Docs con v1
```
✅ TESTING_RAPIDO.md
✅ INICIO_RAPIDO.md
✅ SISTEMA_LISTO_PARA_PRODUCCION.md
✅ API_DOC_REDEMPTIONS.md

Todos usan /api/v1/ correctamente para redenciones
```

---

## 🔮 Evolución Futura

### Si lanzamos Redenciones v2:
```
Nueva versión:
├── /api/v2/rewards/      → Nuevas features de redenciones
│
Versión anterior (deprecated):
└── /api/v1/rewards/      → Mantenida por 6 meses
```

### Core v4 sigue su camino:
```
Sin afectación:
├── /api/v4/users          → Sin cambios
├── /api/v4/invoices       → Sin cambios
└── ... todo v4 sin cambios
```

---

## 📝 Reglas de Versionado

### Para Nuevos Módulos:
1. Inicia en **v1** siempre
2. Incrementa cuando hay breaking changes
3. Mantén retrocompatibilidad con versión anterior por 6 meses

### Para Core Existente:
1. Sigue usando **v4**
2. Solo incrementa si hay breaking changes grandes
3. Avisa con meses de anticipación

---

## 🎯 Resumen

```
┌──────────────────────────────────────────────────────┐
│         VERSIONADO DE LA API LÜM                     │
├──────────────────────────────────────────────────────┤
│                                                      │
│  /api/v4/*  →  Core de la aplicación                │
│                (usuarios, facturas, perfil, etc.)    │
│                                                      │
│  /api/v1/*  →  Sistema de redenciones (NUEVO)       │
│                (rewards, merchant, offers, etc.)     │
│                                                      │
│  Ambos coexisten y son CORRECTOS ✅                 │
│                                                      │
└──────────────────────────────────────────────────────┘
```

**No requiere cambios en la documentación** - Todo está correcto.

---

**Generado**: 19 de octubre, 2024  
**Status**: ✅ DOCUMENTACIÓN VERIFICADA Y CORRECTA
