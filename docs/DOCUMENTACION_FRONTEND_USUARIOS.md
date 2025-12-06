# 📱 Documentación Frontend - API de Redenciones para Usuarios

**Versión**: 3.0  
**Fecha**: 19 de Octubre, 2025  
**Audiencia**: Equipo Frontend (React Native, Flutter, Web)  
**Base URL**: `https://webh.lumapp.org/api/v1`

---

## 📋 TABLA DE CONTENIDOS

1. [Contexto General](#contexto-general)
2. [Flujo del Usuario](#flujo-del-usuario)
3. [Autenticación](#autenticación)
4. [APIs Disponibles](#apis-disponibles)
5. [Ejemplos de Código](#ejemplos-de-código)
6. [Manejo de Errores](#manejo-de-errores)
7. [Push Notifications](#push-notifications)
8. [Testing](#testing)

---

## 🎯 CONTEXTO GENERAL

### ¿Qué son los Lümis?

Los **Lümis** son puntos de recompensa que los usuarios ganan al:
- Escanear facturas (recibos)
- Completar juegos diarios
- Realizar encuestas
- Actividades de gamificación
- Acciones de onboarding

### ¿Qué es una Redención?

Una **redención** es cuando un usuario canjea sus Lümis por ofertas de merchants (comercios). 

**Estados posibles**:
- `pending` - Creada, esperando ser usada
- `confirmed` - Confirmada por el merchant
- `expired` - Expiró sin uso
- `cancelled` - Cancelada por el usuario

### Arquitectura de Datos

```
┌─────────────────────────────────────────────────────────────┐
│                      FACT_ACCUMULATIONS                     │
│  (Todas las formas en que el usuario GANA lümis)            │
├─────────────────────────────────────────────────────────────┤
│ • Facturas escaneadas (receipts)                            │
│ • Juegos diarios (daily_game)                               │
│ • Onboarding (onboarding)                                   │
│ • Gamificación (gamification)                               │
│ • Invoice scan (invoice_scan)                               │
└─────────────────────────────────────────────────────────────┘
                          ↓ SUMA
┌─────────────────────────────────────────────────────────────┐
│                   FACT_BALANCE_POINTS                       │
│  (Balance actual del usuario)                               │
│  balance = SUM(acumulaciones) - SUM(redenciones)            │
└─────────────────────────────────────────────────────────────┘
                          ↓ RESTA
┌─────────────────────────────────────────────────────────────┐
│                     USER_REDEMPTIONS                        │
│  (Todas las veces que el usuario GASTA lümis)               │
├─────────────────────────────────────────────────────────────┤
│ • Redenciones creadas (pending)                             │
│ • Redenciones confirmadas (confirmed)                       │
│ • Redenciones expiradas (expired)                           │
│ • Redenciones canceladas (cancelled)                        │
└─────────────────────────────────────────────────────────────┘
```

**⚠️ IMPORTANTE**: 
- Cuando un usuario crea una redención, **SE DESCUENTA INMEDIATAMENTE** del balance
- Si se cancela antes de expirar, se **REEMBOLSA** al balance
- Si expira, **NO se reembolsa** (queda registrada como expired)

---

## 🔄 FLUJO DEL USUARIO

```
1. Usuario ve ofertas disponibles
   ↓
2. Selecciona una oferta y hace tap en "Redimir"
   ↓
3. Frontend verifica que tenga suficientes lümis
   ↓
4. Frontend llama POST /api/v1/rewards/redeem
   ↓
5. Backend crea la redención y descuenta lümis
   ↓
6. Backend genera código QR único
   ↓
7. Frontend recibe redemption_id, code, QR image URL
   ↓
8. Usuario muestra QR al merchant
   ↓
9. Merchant escanea QR y valida
   ↓
10. Merchant confirma redención
    ↓
11. Usuario recibe push notification
    ↓
12. Redención queda en estado "confirmed"
```

---

## 🔐 AUTENTICACIÓN

Todas las APIs de usuarios requieren un **JWT Token** en el header:

```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Estructura del Token Usuario

```json
{
  "sub": "12345",           // user_id
  "email": "user@example.com",
  "username": "johndoe",
  "exp": 1698765432,        // Unix timestamp
  "iat": 1698679032         // Unix timestamp
}
```

### Cómo obtener el token

```javascript
// Llamada al endpoint de login (fuera del scope de este doc)
const response = await fetch('https://api.lumapp.org/api/v1/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'password123'
  })
});

const { token } = await response.json();
localStorage.setItem('authToken', token);
```

---

## 📡 APIS DISPONIBLES

### 1. 📊 Obtener Balance del Usuario

**Endpoint**: `GET /api/v1/rewards/balance`

**Headers**:
```http
Authorization: Bearer {token}
```

**Response** (200 OK):
```json
{
  "user_id": 12345,
  "balance": 1500,
  "last_updated": "2025-10-19T14:30:00Z"
}
```

**Ejemplo cURL**:
```bash
curl -X GET https://api.lumapp.org/api/v1/rewards/balance \
  -H "Authorization: Bearer {token}"
```

---

### 2. 🎁 Listar Ofertas Disponibles

**Endpoint**: `GET /api/v1/rewards/offers`

**Headers**:
```http
Authorization: Bearer {token}
```

**Query Parameters** (todos opcionales):
- `limit` (number): Máximo de resultados (default: 50)
- `offset` (number): Para paginación (default: 0)
- `category` (string): Filtrar por categoría
- `min_lumis` (number): Ofertas con costo mínimo
- `max_lumis` (number): Ofertas con costo máximo

**Response** (200 OK):
```json
{
  "offers": [
    {
      "offer_id": "550e8400-e29b-41d4-a716-446655440000",
      "name_friendly": "Café Gratis",
      "name_internal": "cafe_free_small",
      "description": "Café pequeño o mediano gratis",
      "lumis_cost": 50,
      "points": 50,
      "category": "food",
      "merchant": {
        "merchant_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
        "merchant_name": "Café Central",
        "logo_url": "https://cdn.lumapp.org/merchants/cafe-central.png"
      },
      "terms_and_conditions": "Válido de lunes a viernes, 8am-12pm",
      "is_active": true,
      "created_at": "2025-01-15T10:00:00Z"
    },
    {
      "offer_id": "660f9511-e39c-42e5-b827-f17e10440111",
      "name_friendly": "15% de descuento",
      "lumis_cost": 100,
      "category": "discount",
      "merchant": {
        "merchant_name": "Tienda XYZ"
      }
    }
  ],
  "total": 45,
  "limit": 50,
  "offset": 0
}
```

**Ejemplo cURL**:
```bash
curl -X GET "https://api.lumapp.org/api/v1/rewards/offers?limit=10&category=food" \
  -H "Authorization: Bearer {token}"
```

---

### 3. 🎯 Crear una Redención (Canjear Lümis)

**Endpoint**: `POST /api/v1/rewards/redeem`

**Headers**:
```http
Authorization: Bearer {token}
Content-Type: application/json
```

**Request Body**:
```json
{
  "offer_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": 12345
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "message": "Redención creada exitosamente",
  "redemption": {
    "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
    "user_id": 12345,
    "offer_id": "550e8400-e29b-41d4-a716-446655440000",
    "lumis_spent": 50,
    "redemption_code": "LUM-A1B2-C3D4",
    "qr_image_url": "https://cdn.lumapp.org/qr/a1b2c3d4-e5f6-4789-a012-3456789abcde.png",
    "qr_landing_url": "https://lumapp.org/redeem/LUM-A1B2-C3D4",
    "status": "pending",
    "expires_at": "2025-10-26T14:30:00Z",
    "created_at": "2025-10-19T14:30:00Z"
  },
  "new_balance": 1450
}
```

**Errores Posibles**:

- **400 Bad Request** - Balance insuficiente:
```json
{
  "error": "Insufficient balance",
  "message": "No tienes suficientes lümis. Necesitas 50 pero solo tienes 30",
  "required": 50,
  "current_balance": 30
}
```

- **404 Not Found** - Oferta no existe:
```json
{
  "error": "Offer not found",
  "message": "La oferta solicitada no existe o está inactiva"
}
```

- **429 Too Many Requests** - Rate limit:
```json
{
  "error": "Rate limit exceeded",
  "message": "Has alcanzado el límite de 10 redenciones por día. Intenta mañana.",
  "retry_after": 86400
}
```

**Ejemplo cURL**:
```bash
curl -X POST https://api.lumapp.org/api/v1/rewards/redeem \
  -H "Authorization: Bearer {token}" \
  -H "Content-Type: application/json" \
  -d '{
    "offer_id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": 12345
  }'
```

---

### 4. 📜 Obtener Historial de Redenciones

**Endpoint**: `GET /api/v1/rewards/history`

**Headers**:
```http
Authorization: Bearer {token}
```

**Query Parameters** (todos opcionales):
- `limit` (number): Máximo de resultados (default: 20)
- `offset` (number): Para paginación
- `status` (string): Filtrar por estado (`pending`, `confirmed`, `expired`, `cancelled`)
- `start_date` (ISO 8601): Fecha inicial
- `end_date` (ISO 8601): Fecha final

**Response** (200 OK):
```json
{
  "redemptions": [
    {
      "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
      "offer": {
        "offer_id": "550e8400-e29b-41d4-a716-446655440000",
        "name_friendly": "Café Gratis",
        "merchant_name": "Café Central"
      },
      "lumis_spent": 50,
      "redemption_code": "LUM-A1B2-C3D4",
      "status": "confirmed",
      "qr_image_url": "https://cdn.lumapp.org/qr/a1b2c3d4.png",
      "created_at": "2025-10-19T14:30:00Z",
      "validated_at": "2025-10-19T15:00:00Z",
      "expires_at": "2025-10-26T14:30:00Z"
    },
    {
      "redemption_id": "b2c3d4e5-f6a7-4890-b123-456789abcdef",
      "offer": {
        "name_friendly": "15% Descuento",
        "merchant_name": "Tienda XYZ"
      },
      "lumis_spent": 100,
      "status": "pending",
      "created_at": "2025-10-18T10:00:00Z",
      "expires_at": "2025-10-25T10:00:00Z"
    }
  ],
  "total": 25,
  "limit": 20,
  "offset": 0
}
```

**Ejemplo cURL**:
```bash
curl -X GET "https://api.lumapp.org/api/v1/rewards/history?status=pending&limit=10" \
  -H "Authorization: Bearer {token}"
```

---

### 5. 🔍 Obtener Detalles de una Redención

**Endpoint**: `GET /api/v1/rewards/redemptions/:redemption_id`

**Headers**:
```http
Authorization: Bearer {token}
```

**Response** (200 OK):
```json
{
  "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
  "user_id": 12345,
  "offer": {
    "offer_id": "550e8400-e29b-41d4-a716-446655440000",
    "name_friendly": "Café Gratis",
    "description": "Café pequeño o mediano gratis",
    "merchant": {
      "merchant_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "merchant_name": "Café Central",
      "logo_url": "https://cdn.lumapp.org/merchants/cafe-central.png"
    }
  },
  "lumis_spent": 50,
  "redemption_code": "LUM-A1B2-C3D4",
  "qr_image_url": "https://cdn.lumapp.org/qr/a1b2c3d4-e5f6-4789-a012-3456789abcde.png",
  "qr_landing_url": "https://lumapp.org/redeem/LUM-A1B2-C3D4",
  "status": "confirmed",
  "created_at": "2025-10-19T14:30:00Z",
  "validated_at": "2025-10-19T15:00:00Z",
  "validated_by_merchant": {
    "merchant_name": "Café Central",
    "location": "Sucursal Centro"
  },
  "expires_at": "2025-10-26T14:30:00Z"
}
```

**Error 404**:
```json
{
  "error": "Redemption not found",
  "message": "La redención solicitada no existe o no pertenece a este usuario"
}
```

**Ejemplo cURL**:
```bash
curl -X GET https://api.lumapp.org/api/v1/rewards/redemptions/a1b2c3d4-e5f6-4789-a012-3456789abcde \
  -H "Authorization: Bearer {token}"
```

---

### 6. ❌ Cancelar una Redención

**Endpoint**: `POST /api/v1/rewards/redemptions/:redemption_id/cancel`

**Headers**:
```http
Authorization: Bearer {token}
Content-Type: application/json
```

**Request Body** (opcional):
```json
{
  "reason": "Cambié de opinión"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "message": "Redención cancelada exitosamente",
  "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
  "lumis_refunded": 50,
  "new_balance": 1500,
  "cancelled_at": "2025-10-19T16:00:00Z"
}
```

**Errores Posibles**:

- **400 Bad Request** - No se puede cancelar:
```json
{
  "error": "Cannot cancel redemption",
  "message": "Solo puedes cancelar redenciones en estado 'pending'. Esta redención está 'confirmed'"
}
```

- **404 Not Found**:
```json
{
  "error": "Redemption not found"
}
```

**Ejemplo cURL**:
```bash
curl -X POST https://api.lumapp.org/api/v1/rewards/redemptions/a1b2c3d4/cancel \
  -H "Authorization: Bearer {token}" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Cambié de opinión"}'
```

---

### 7. 📊 Obtener Historial de Acumulaciones

**Endpoint**: `GET /api/v1/rewards/accumulations`

**Headers**:
```http
Authorization: Bearer {token}
```

**Query Parameters**:
- `limit` (number): Default 50
- `offset` (number): Default 0
- `accum_type` (string): Filtrar por tipo (`receipts`, `daily_game`, `gamification`, etc.)
- `start_date` (ISO 8601)
- `end_date` (ISO 8601)

**Response** (200 OK):
```json
{
  "accumulations": [
    {
      "id": 12345,
      "user_id": 12345,
      "accum_type": "receipts",
      "dtype": "points",
      "quantity": 10,
      "balance": 1510,
      "date": "2025-10-19T14:00:00Z",
      "description": "Factura escaneada - Supermercado ABC"
    },
    {
      "id": 12344,
      "accum_type": "daily_game",
      "quantity": 5,
      "balance": 1500,
      "date": "2025-10-19T08:00:00Z",
      "description": "Juego diario completado"
    },
    {
      "id": 12343,
      "accum_type": "spend",
      "dtype": "points",
      "quantity": -50,
      "balance": 1495,
      "date": "2025-10-18T15:00:00Z",
      "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
      "description": "Redención: Café Gratis"
    }
  ],
  "total": 250,
  "limit": 50,
  "offset": 0
}
```

**Ejemplo cURL**:
```bash
curl -X GET "https://api.lumapp.org/api/v1/rewards/accumulations?accum_type=receipts&limit=20" \
  -H "Authorization: Bearer {token}"
```

---

## 💻 EJEMPLOS DE CÓDIGO

### React Native Example

```javascript
import React, { useState, useEffect } from 'react';
import { View, Text, FlatList, TouchableOpacity, Image } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';

const API_BASE = 'https://api.lumapp.org/api/v1';

// Hook personalizado para redenciones
const useRedemptions = () => {
  const [balance, setBalance] = useState(0);
  const [offers, setOffers] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  const getHeaders = async () => {
    const token = await AsyncStorage.getItem('authToken');
    return {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    };
  };

  const fetchBalance = async () => {
    try {
      setLoading(true);
      const headers = await getHeaders();
      const response = await fetch(`${API_BASE}/rewards/balance`, { headers });
      
      if (!response.ok) throw new Error('Error al obtener balance');
      
      const data = await response.json();
      setBalance(data.balance);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const fetchOffers = async () => {
    try {
      setLoading(true);
      const headers = await getHeaders();
      const response = await fetch(`${API_BASE}/rewards/offers`, { headers });
      
      if (!response.ok) throw new Error('Error al obtener ofertas');
      
      const data = await response.json();
      setOffers(data.offers);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const redeemOffer = async (offerId, userId) => {
    try {
      setLoading(true);
      const headers = await getHeaders();
      const response = await fetch(`${API_BASE}/rewards/redeem`, {
        method: 'POST',
        headers,
        body: JSON.stringify({ offer_id: offerId, user_id: userId })
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Error al redimir');
      }

      const data = await response.json();
      setBalance(data.new_balance);
      return data.redemption;
    } catch (err) {
      setError(err.message);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  return { balance, offers, loading, error, fetchBalance, fetchOffers, redeemOffer };
};

// Componente de ejemplo
const OffersScreen = () => {
  const { balance, offers, loading, error, fetchBalance, fetchOffers, redeemOffer } = useRedemptions();
  const [userId] = useState(12345); // Obtener del contexto de auth

  useEffect(() => {
    fetchBalance();
    fetchOffers();
  }, []);

  const handleRedeem = async (offer) => {
    try {
      const redemption = await redeemOffer(offer.offer_id, userId);
      // Navegar a pantalla de QR
      navigation.navigate('RedemptionQR', { redemption });
    } catch (err) {
      Alert.alert('Error', err.message);
    }
  };

  return (
    <View style={{ flex: 1, padding: 16 }}>
      <Text style={{ fontSize: 24, fontWeight: 'bold' }}>
        Balance: {balance} Lümis
      </Text>
      
      {loading && <Text>Cargando...</Text>}
      {error && <Text style={{ color: 'red' }}>{error}</Text>}

      <FlatList
        data={offers}
        keyExtractor={(item) => item.offer_id}
        renderItem={({ item }) => (
          <View style={{ marginVertical: 10, padding: 16, backgroundColor: '#f5f5f5' }}>
            <Text style={{ fontSize: 18, fontWeight: 'bold' }}>{item.name_friendly}</Text>
            <Text>{item.description}</Text>
            <Text style={{ color: '#666' }}>
              {item.merchant.merchant_name} - {item.lumis_cost} Lümis
            </Text>
            <TouchableOpacity
              onPress={() => handleRedeem(item)}
              disabled={balance < item.lumis_cost}
              style={{
                marginTop: 10,
                padding: 10,
                backgroundColor: balance >= item.lumis_cost ? '#4CAF50' : '#ccc',
                borderRadius: 5
              }}
            >
              <Text style={{ color: 'white', textAlign: 'center' }}>
                {balance >= item.lumis_cost ? 'Redimir' : 'Lümis insuficientes'}
              </Text>
            </TouchableOpacity>
          </View>
        )}
      />
    </View>
  );
};

export default OffersScreen;
```

### Flutter Example

```dart
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';

class RedemptionService {
  static const String baseUrl = 'https://api.lumapp.org/api/v1';

  Future<Map<String, String>> _getHeaders() async {
    final prefs = await SharedPreferences.getInstance();
    final token = prefs.getString('authToken') ?? '';
    return {
      'Authorization': 'Bearer $token',
      'Content-Type': 'application/json',
    };
  }

  Future<int> getBalance() async {
    final headers = await _getHeaders();
    final response = await http.get(
      Uri.parse('$baseUrl/rewards/balance'),
      headers: headers,
    );

    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      return data['balance'] as int;
    } else {
      throw Exception('Failed to load balance');
    }
  }

  Future<List<dynamic>> getOffers() async {
    final headers = await _getHeaders();
    final response = await http.get(
      Uri.parse('$baseUrl/rewards/offers'),
      headers: headers,
    );

    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      return data['offers'] as List;
    } else {
      throw Exception('Failed to load offers');
    }
  }

  Future<Map<String, dynamic>> redeemOffer(String offerId, int userId) async {
    final headers = await _getHeaders();
    final response = await http.post(
      Uri.parse('$baseUrl/rewards/redeem'),
      headers: headers,
      body: jsonEncode({
        'offer_id': offerId,
        'user_id': userId,
      }),
    );

    if (response.statusCode == 201) {
      final data = jsonDecode(response.body);
      return data['redemption'] as Map<String, dynamic>;
    } else {
      final errorData = jsonDecode(response.body);
      throw Exception(errorData['message'] ?? 'Failed to redeem offer');
    }
  }
}

class OffersPage extends StatefulWidget {
  @override
  _OffersPageState createState() => _OffersPageState();
}

class _OffersPageState extends State<OffersPage> {
  final RedemptionService _service = RedemptionService();
  int _balance = 0;
  List<dynamic> _offers = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  Future<void> _loadData() async {
    try {
      final balance = await _service.getBalance();
      final offers = await _service.getOffers();
      setState(() {
        _balance = balance;
        _offers = offers;
        _loading = false;
      });
    } catch (e) {
      print('Error loading data: $e');
      setState(() => _loading = false);
    }
  }

  Future<void> _handleRedeem(Map<String, dynamic> offer) async {
    try {
      final redemption = await _service.redeemOffer(offer['offer_id'], 12345);
      // Navigate to QR screen
      Navigator.push(
        context,
        MaterialPageRoute(
          builder: (context) => RedemptionQRScreen(redemption: redemption),
        ),
      );
    } catch (e) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Error: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Ofertas'),
      ),
      body: _loading
          ? Center(child: CircularProgressIndicator())
          : Column(
              children: [
                Padding(
                  padding: EdgeInsets.all(16),
                  child: Text(
                    'Balance: $_balance Lümis',
                    style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold),
                  ),
                ),
                Expanded(
                  child: ListView.builder(
                    itemCount: _offers.length,
                    itemBuilder: (context, index) {
                      final offer = _offers[index];
                      final canRedeem = _balance >= offer['lumis_cost'];
                      
                      return Card(
                        margin: EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                        child: Padding(
                          padding: EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                offer['name_friendly'],
                                style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
                              ),
                              SizedBox(height: 8),
                              Text(offer['description'] ?? ''),
                              SizedBox(height: 8),
                              Text(
                                '${offer['merchant']['merchant_name']} - ${offer['lumis_cost']} Lümis',
                                style: TextStyle(color: Colors.grey[600]),
                              ),
                              SizedBox(height: 12),
                              ElevatedButton(
                                onPressed: canRedeem ? () => _handleRedeem(offer) : null,
                                child: Text(canRedeem ? 'Redimir' : 'Lümis insuficientes'),
                                style: ElevatedButton.styleFrom(
                                  backgroundColor: canRedeem ? Colors.green : Colors.grey,
                                ),
                              ),
                            ],
                          ),
                        ),
                      );
                    },
                  ),
                ),
              ],
            ),
    );
  }
}
```

---

## ⚠️ MANEJO DE ERRORES

### Códigos de Error Comunes

| Código | Significado | Acción Recomendada |
|--------|-------------|-------------------|
| 400 | Bad Request | Verificar los datos enviados |
| 401 | Unauthorized | Token inválido o expirado, re-autenticar |
| 403 | Forbidden | Usuario no tiene permisos |
| 404 | Not Found | Recurso no existe |
| 429 | Too Many Requests | Esperar antes de reintentar |
| 500 | Internal Server Error | Contactar soporte |

### Estructura de Errores

Todos los errores siguen este formato:

```json
{
  "error": "error_code",
  "message": "Mensaje descriptivo para el usuario",
  "details": {} // Opcional, información adicional
}
```

### Ejemplos de Manejo

```javascript
async function redeemOffer(offerId, userId) {
  try {
    const response = await fetch(`${API_BASE}/rewards/redeem`, {
      method: 'POST',
      headers: await getHeaders(),
      body: JSON.stringify({ offer_id: offerId, user_id: userId })
    });

    // Primero parsear el JSON
    const data = await response.json();

    // Luego verificar el status
    if (!response.ok) {
      // Manejar errores específicos
      switch (response.status) {
        case 400:
          if (data.error === 'Insufficient balance') {
            Alert.alert(
              'Balance Insuficiente',
              `Necesitas ${data.required} Lümis pero solo tienes ${data.current_balance}`
            );
          }
          break;
        case 401:
          // Token expirado, re-autenticar
          await refreshToken();
          return redeemOffer(offerId, userId); // Reintentar
        case 429:
          Alert.alert(
            'Límite Alcanzado',
            'Has redimido demasiadas ofertas hoy. Intenta mañana.'
          );
          break;
        default:
          Alert.alert('Error', data.message || 'Error desconocido');
      }
      throw new Error(data.message);
    }

    return data.redemption;
  } catch (error) {
    console.error('Error redeeming offer:', error);
    throw error;
  }
}
```

---

## 🔔 PUSH NOTIFICATIONS

El backend envía push notifications automáticamente en estos eventos:

### 1. Redención Creada
```json
{
  "title": "🎁 Nueva redención creada",
  "body": "Tu código: LUM-A1B2-C3D4",
  "data": {
    "type": "redemption_created",
    "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
    "code": "LUM-A1B2-C3D4"
  }
}
```

### 2. Redención Confirmada
```json
{
  "title": "✅ ¡Redención confirmada!",
  "body": "Tu Café Gratis ha sido confirmado",
  "data": {
    "type": "redemption_confirmed",
    "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
    "offer_name": "Café Gratis"
  }
}
```

### 3. Redención Por Expirar
```json
{
  "title": "⏰ Tu redención está por expirar",
  "body": "Tu Café Gratis expira en 5 minutos",
  "data": {
    "type": "redemption_expiring",
    "redemption_id": "a1b2c3d4-e5f6-4789-a012-3456789abcde",
    "minutes_remaining": 5
  }
}
```

### Configuración FCM (Firebase Cloud Messaging)

#### Registrar Token de Dispositivo

**Endpoint**: `POST /api/v1/devices/register`

```json
{
  "user_id": 12345,
  "fcm_token": "dXNlcl9mY21fdG9rZW5fZXhhbXBsZQ==",
  "device_type": "android",
  "device_name": "Samsung Galaxy S21"
}
```

#### React Native con Firebase

```javascript
import messaging from '@react-native-firebase/messaging';

async function registerForPushNotifications() {
  const authStatus = await messaging().requestPermission();
  const enabled =
    authStatus === messaging.AuthorizationStatus.AUTHORIZED ||
    authStatus === messaging.AuthorizationStatus.PROVISIONAL;

  if (enabled) {
    const fcmToken = await messaging().getToken();
    
    // Registrar token en backend
    await fetch('https://api.lumapp.org/api/v1/devices/register', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        user_id: userId,
        fcm_token: fcmToken,
        device_type: Platform.OS,
        device_name: await DeviceInfo.getDeviceName()
      })
    });
  }
}

// Manejar notificaciones en foreground
messaging().onMessage(async remoteMessage => {
  console.log('Notification received:', remoteMessage);
  
  if (remoteMessage.data.type === 'redemption_confirmed') {
    // Actualizar UI, mostrar toast, etc.
    showToast('¡Redención confirmada!');
    refreshRedemptions();
  }
});

// Manejar tap en notificación
messaging().onNotificationOpenedApp(remoteMessage => {
  console.log('Notification caused app to open:', remoteMessage);
  
  if (remoteMessage.data.redemption_id) {
    // Navegar a detalles de redención
    navigation.navigate('RedemptionDetails', {
      redemptionId: remoteMessage.data.redemption_id
    });
  }
});
```

---

## 🧪 TESTING

### Datos de Prueba

Para testing en ambiente de desarrollo:

**Usuario de Prueba**:
```json
{
  "user_id": 99999,
  "email": "test@lumapp.org",
  "balance": 5000
}
```

**Oferta de Prueba**:
```json
{
  "offer_id": "550e8400-e29b-41d4-a716-446655440000",
  "name_friendly": "Producto de Prueba",
  "lumis_cost": 10
}
```

### Endpoints de Testing

**Base URL de Staging**: `https://staging-api.lumapp.org/api/v1`

### Casos de Prueba Recomendados

1. **Happy Path**:
   - ✅ Obtener balance
   - ✅ Listar ofertas
   - ✅ Redimir oferta con balance suficiente
   - ✅ Verificar que balance se actualizó
   - ✅ Cancelar redención pending
   - ✅ Verificar que balance se restauró

2. **Error Cases**:
   - ❌ Intentar redimir sin suficientes lümis
   - ❌ Intentar redimir oferta inválida
   - ❌ Intentar cancelar redención confirmed
   - ❌ Token expirado

3. **Rate Limiting**:
   - 🔄 Crear 11 redenciones en un día (debe fallar la 11va)

---

## 📝 NOTAS IMPORTANTES

### Balance y Sincronización

- El balance se actualiza **inmediatamente** al crear una redención
- Si la redención es cancelada, el balance se **restaura automáticamente**
- Si la redención expira, **NO se restaura** el balance (por diseño)
- Las acumulaciones se procesan en tiempo real vía triggers de base de datos

### Expiración de Redenciones

- Las redenciones tienen **7 días** de validez por defecto
- Se envía notificación push **5 minutos antes** de expirar
- Un job automático marca como `expired` las redenciones vencidas cada hora

### Rate Limiting

| Acción | Límite |
|--------|--------|
| Crear redenciones | 10 por día por usuario |
| Listar ofertas | 100 por minuto por IP |
| Obtener balance | Sin límite |

### Códigos QR

- Los QR se generan automáticamente al crear una redención
- La URL del QR image es **pública** (no requiere auth)
- El QR landing URL redirige a una página web para verificación manual
- Los QR se cachean por 30 días

---

## 🆘 SOPORTE

**Backend Team**:
- Email: backend@lumapp.org
- Slack: #lumis-redemption

**Documentación Adicional**:
- [Arquitectura](./redemptions/01-arquitectura.md)
- [Modelo de Datos](./redemptions/03-modelo-datos.md)
- [Troubleshooting](./redemptions/13-troubleshooting.md)

---

**Última actualización**: 19 de Octubre, 2025  
**Versión del API**: 3.0  
**Mantenido por**: Equipo Backend Lüm
