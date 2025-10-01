# 🎮 API de Gamificación - Guía Frontend

## 📋 **Índice**
1. [Conceptos Clave](#conceptos-clave)
2. [API de Niveles Lüm](#api-de-niveles-lüm)
3. [APIs de Streaks (Rachas)](#apis-de-streaks-rachas)
4. [Ejemplos de Implementación](#ejemplos-de-implementación)
5. [Tipos de Datos](#tipos-de-datos)

---

## 🎯 **Conceptos Clave**

### **Separación de Conceptos:**
- **📊 NIVEL LÜM:** Basado en total de facturas subidas (nunca baja)
- **💰 LÜMIS DISPONIBLES:** Balance para redimir (puede bajar)
- **🔥 STREAKS:** Rachas de actividades consecutivas
- **🎯 PROGRESO:** Facturas restantes para siguiente nivel

---

## 📊 **API de Niveles Lüm**

### **Endpoint Principal**
```http
GET /api/v4/gamification/dashboard
```

**Headers:**
```http
Authorization: Bearer {jwt_token}
Content-Type: application/json
```

### **Respuesta Ejemplo:**
```json
{
  "success": true,
  "data": {
    "user_id": 123,
    "email": "usuario@example.com",
    "total_lumis": 47,              // ⚠️ Representa FACTURAS subidas, NO balance disponible
    "current_level": 4,
    "level_name": "Silver Hunter",
    "level_description": "Silver Hunter - Basado en 47 facturas",
    "level_color": "#C0C0C0",
    "level_benefits": {
      "lumis_multiplier": 1.2,
      "special_events": true
    },
    "lumis_to_next_level": 3,       // Facturas restantes para siguiente nivel
    "next_level_name": "Silver Expert",
    "engagement_score": 78,
    "total_invoices": 47,           // Campo explícito para facturas
    "last_invoice_date": "2025-08-30T10:30:00Z",
    "active_streaks": [
      {
        "type": "daily_login",
        "current": 5,
        "max": 12,
        "last_date": "2025-09-01"
      }
    ],
    "active_missions_count": 2,
    "completed_missions_count": 8,
    "total_achievements": 15
  }
}
```

### **Niveles Disponibles:**
| Nivel | Nombre | Facturas Requeridas |
|-------|--------|-------------------|
| 1 | Chispa Lüm | 0-4 facturas |
| 2 | Bronze Explorer | 5-9 facturas |
| 3 | Bronze Master | 10-24 facturas |
| 4 | Silver Hunter | 25-49 facturas |
| 5 | Silver Expert | 50-99 facturas |
| 6 | Gold Hunter | 100-249 facturas |
| 7 | Gold Legend | 250-499 facturas |
| 8 | Platinum Master | 500+ facturas |

---

## 🔥 **APIs de Streaks (Rachas)**

### **1. Registrar Login Diario**
```http
POST /api/v4/gamification/track
```

**Request Body:**
```json
{
  "action": "daily_login",
  "channel": "mobile_app",
  "metadata": {}
}
```

**Respuesta:**
```json
{
  "success": true,
  "data": {
    "lumis_earned": 2,
    "total_lumis": 47,
    "xp_earned": 0,
    "current_level": 4,
    "level_name": "Silver Hunter",
    "streaks": {
      "current_streak": 5,
      "already_claimed": false,
      "next_reward_day": 7,
      "max_streak": 12,
      "achievement_unlocked": null
    },
    "achievements_unlocked": [],
    "active_events": [],
    "message": "¡Día 5! +2 Lümis"
  }
}
```

### **2. Registrar Subida de Factura**
```http
POST /api/v4/gamification/track
```

**Request Body:**
```json
{
  "action": "invoice_upload",
  "channel": "mobile_app",
  "metadata": {
    "invoice_id": 12345,
    "category": "restaurant",
    "amount": 25.50
  }
}
```

**Respuesta:**
```json
{
  "success": true,
  "data": {
    "lumis_earned": 5,
    "total_lumis": 48,              // Se incrementó por la nueva factura
    "xp_earned": 0,
    "current_level": 4,
    "level_name": "Silver Hunter",
    "streaks": {
      "weekly_invoice": {
        "current": 2,
        "message": "¡2 facturas esta semana!"
      }
    },
    "achievements_unlocked": [],
    "active_events": [],
    "message": "¡Factura subida! +5 Lümis"
  }
}
```

### **3. Consultar Todas las Rachas**
```http
GET /api/v4/gamification/dashboard
```

En la respuesta del dashboard viene el campo `active_streaks`:

```json
{
  "active_streaks": [
    {
      "type": "daily_login",
      "current": 5,
      "max": 12,
      "last_date": "2025-09-01"
    },
    {
      "type": "weekly_invoice", 
      "current": 2,
      "max": 4,
      "last_date": "2025-09-01"
    }
  ]
}
```

---

## 🛠️ **Ejemplos de Implementación**

### **Frontend - Mostrar Nivel del Usuario**
```typescript
interface UserLevel {
  user_id: number;
  total_lumis: number;        // ⚠️ Este campo representa facturas, NO balance
  current_level: number;
  level_name: string;
  level_color: string;
  lumis_to_next_level: number; // Facturas restantes
  next_level_name: string;
  total_invoices: number;     // Campo explícito para facturas
  engagement_score: number;
}

// Componente React/Vue ejemplo
function UserLevelDisplay({ user }: { user: UserLevel }) {
  return (
    <div className="user-level-card">
      <h3 style={{ color: user.level_color }}>
        {user.level_name}
      </h3>
      <p>Has subido {user.total_invoices} facturas</p>
      <p>Faltan {user.lumis_to_next_level} facturas para {user.next_level_name}</p>
      <div className="progress-bar">
        <div 
          className="progress" 
          style={{ 
            width: `${((user.total_invoices / (user.total_invoices + user.lumis_to_next_level)) * 100)}%` 
          }}
        />
      </div>
    </div>
  );
}
```

### **Frontend - Manejar Streaks**
```typescript
interface Streak {
  type: string;
  current: number;
  max: number;
  last_date: string;
}

async function trackDailyLogin() {
  try {
    const response = await fetch('/api/v4/gamification/track', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        action: 'daily_login',
        channel: 'mobile_app',
        metadata: {}
      })
    });
    
    const result = await response.json();
    
    if (result.success) {
      // Mostrar notificación de recompensa
      showNotification(`${result.data.message} (+${result.data.lumis_earned} Lümis)`);
      
      // Actualizar UI con nueva información
      updateUserLevel(result.data);
      updateStreaks(result.data.streaks);
    }
  } catch (error) {
    console.error('Error tracking login:', error);
  }
}
```

---

## 📋 **Tipos de Datos**

### **UserDashboard**
```typescript
interface UserDashboard {
  user_id: number;
  email: string;
  total_lumis: number;              // ⚠️ Representa facturas subidas
  current_level: number;            // 1-8
  level_name: string;               // "Silver Hunter"
  level_description: string;
  level_color: string;              // "#C0C0C0"
  level_benefits: object;
  lumis_to_next_level: number;      // Facturas restantes
  next_level_name: string;
  engagement_score: number;         // 0-100
  total_invoices: number;           // Campo real de facturas
  last_invoice_date: string;        // ISO 8601
  active_streaks: Streak[];
  active_missions_count: number;
  completed_missions_count: number;
  total_achievements: number;
}
```

### **GamificationResponse** 
```typescript
interface GamificationResponse {
  lumis_earned: number;
  total_lumis: number;              // ⚠️ Representa facturas subidas
  xp_earned: number;
  current_level: number;
  level_name: string;
  streaks: object;                  // Información de rachas específica
  achievements_unlocked: object[];
  active_events: object[];
  message: string;
}
```

### **Streak**
```typescript
interface Streak {
  type: 'daily_login' | 'weekly_invoice' | 'survey_complete';
  current: number;                  // Días/semanas consecutivas actuales
  max: number;                      // Récord máximo del usuario
  last_date: string;               // Última actividad (YYYY-MM-DD)
}
```

---

## ⚠️ **Notas Importantes para Frontend**

### **1. Nomenclatura Confusa (Legacy)**
- El campo `total_lumis` en realidad representa **facturas subidas**
- Para mostrar balance real de Lümis usar otra API de wallet/balance
- Siempre usar `total_invoices` para mostrar facturas al usuario

### **2. Actualización en Tiempo Real**
- Llamar `/track` después de cada acción importante (login, subir factura)
- Refrescar dashboard después de `track` exitoso
- Las rachas se actualizan automáticamente

### **3. Manejo de Errores**
```typescript
// Siempre verificar success en la respuesta
if (response.data.success) {
  // Procesar datos
} else {
  // Manejar error
  console.error('Gamification error:', response.data.message);
}
```

### **4. UX Recomendada**
- Mostrar animaciones cuando suben de nivel
- Celebrar hitos de rachas (7 días, 30 días, etc.)
- Usar colores de nivel para personalización
- Mostrar progreso visual hacia siguiente nivel

---

## 🔗 **Enlaces Útiles**

- **Dashboard:** `GET /api/v4/gamification/dashboard`
- **Track Login:** `POST /api/v4/gamification/track` (action: "daily_login")
- **Track Invoice:** `POST /api/v4/gamification/track` (action: "invoice_upload")
- **Missions:** `GET /api/v4/gamification/missions`
- **Achievements:** `GET /api/v4/gamification/achievements`
