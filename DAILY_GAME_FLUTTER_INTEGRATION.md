# 🎮 Daily Game - Guía de Integración Flutter

## 📱 Integración con App Flutter

Esta guía explica cómo integrar los endpoints del Daily Game en la aplicación Flutter.

---

## 🔌 Endpoints Disponibles

### Base URL
```
https://api.2factu.com/api/v4/daily-game
```

O para desarrollo:
```
http://localhost:8000/api/v4/daily-game
```

---

## 📋 API Reference

### 1️⃣ GET `/status` - Obtener Estado del Juego

**Descripción**: Verifica si el usuario puede jugar hoy y obtiene sus estadísticas.

**Método**: `GET`  
**Auth**: Bearer Token (JWT)  
**Endpoint**: `/api/v4/daily-game/status`

#### Request
```dart
final response = await http.get(
  Uri.parse('$baseUrl/api/v4/daily-game/status'),
  headers: {
    'Authorization': 'Bearer $token',
    'Content-Type': 'application/json',
  },
);
```

#### Response Success (200 OK)

**Primera vez (nunca jugó)**:
```json
{
  "success": true,
  "data": {
    "can_play_today": true,
    "has_played_today": false
  }
}
```

**Ya jugó hoy**:
```json
{
  "success": true,
  "data": {
    "can_play_today": false,
    "has_played_today": true,
    "todays_reward": 5,
    "stats": {
      "total_plays": 10,
      "total_lumis_won": 23,
      "golden_stars_captured": 2
    }
  }
}
```

#### Modelo Dart
```dart
class DailyGameStatus {
  final bool canPlayToday;
  final bool hasPlayedToday;
  final int? todaysReward;
  final DailyGameStats? stats;

  DailyGameStatus({
    required this.canPlayToday,
    required this.hasPlayedToday,
    this.todaysReward,
    this.stats,
  });

  factory DailyGameStatus.fromJson(Map<String, dynamic> json) {
    return DailyGameStatus(
      canPlayToday: json['can_play_today'],
      hasPlayedToday: json['has_played_today'],
      todaysReward: json['todays_reward'],
      stats: json['stats'] != null 
        ? DailyGameStats.fromJson(json['stats']) 
        : null,
    );
  }
}

class DailyGameStats {
  final int totalPlays;
  final int totalLumisWon;
  final int goldenStarsCaptured;

  DailyGameStats({
    required this.totalPlays,
    required this.totalLumisWon,
    required this.goldenStarsCaptured,
  });

  factory DailyGameStats.fromJson(Map<String, dynamic> json) {
    return DailyGameStats(
      totalPlays: json['total_plays'],
      totalLumisWon: json['total_lumis_won'],
      goldenStarsCaptured: json['golden_stars_captured'],
    );
  }
}
```

---

### 2️⃣ POST `/claim` - Reclamar Recompensa

**Descripción**: Reclama la recompensa después de que el usuario elija una estrella.

**Método**: `POST`  
**Auth**: Bearer Token (JWT)  
**Endpoint**: `/api/v4/daily-game/claim`

#### Request
```dart
final response = await http.post(
  Uri.parse('$baseUrl/api/v4/daily-game/claim'),
  headers: {
    'Authorization': 'Bearer $token',
    'Content-Type': 'application/json',
  },
  body: jsonEncode({
    'star_id': 'star_3',  // star_0 a star_8
    'lumis_won': 5,       // 0, 1, o 5
  }),
);
```

#### Request Body
```json
{
  "star_id": "star_3",
  "lumis_won": 5
}
```

**Valores válidos**:
- `star_id`: `"star_0"` a `"star_8"` (9 estrellas totales)
- `lumis_won`: `0` (vacía), `1` (normal), `5` (dorada)

#### Response Success (200 OK)

**Estrella dorada (5 Lümis)**:
```json
{
  "success": true,
  "data": {
    "lumis_added": 5,
    "new_balance": 308,
    "play_id": 1
  },
  "message": "¡Increíble! 🌟✨ ¡Encontraste la estrella dorada! +5 Lümis"
}
```

**Estrella normal (1 Lümi)**:
```json
{
  "success": true,
  "data": {
    "lumis_added": 1,
    "new_balance": 304,
    "play_id": 2
  },
  "message": "¡Genial! ⭐ Has ganado +1 Lümi"
}
```

**Estrella vacía (0 Lümis)**:
```json
{
  "success": true,
  "data": {
    "lumis_added": 0,
    "new_balance": 303,
    "play_id": 3
  },
  "message": "¡Ups! 💫 Estrella vacía, pero mañana tendrás otra oportunidad."
}
```

#### Response Error (400 Bad Request)

**lumis_won inválido**:
```json
{
  "success": false,
  "error": {
    "code": "ERROR",
    "message": "Invalid lumis_won value: 10. Must be 0, 1, or 5"
  }
}
```

**star_id inválido**:
```json
{
  "success": false,
  "error": {
    "code": "ERROR",
    "message": "Invalid star_id: star_99. Must be star_0 to star_8"
  }
}
```

#### Response Error (409 Conflict)

**Ya jugó hoy**:
```json
{
  "success": false,
  "error": {
    "code": "ALREADY_PLAYED_TODAY",
    "message": "Ya jugaste hoy. Vuelve mañana a las 00:00."
  }
}
```

#### Modelo Dart
```dart
class DailyGameClaimResponse {
  final int lumisAdded;
  final int newBalance;
  final int playId;
  final String message;

  DailyGameClaimResponse({
    required this.lumisAdded,
    required this.newBalance,
    required this.playId,
    required this.message,
  });

  factory DailyGameClaimResponse.fromJson(Map<String, dynamic> json) {
    return DailyGameClaimResponse(
      lumisAdded: json['data']['lumis_added'],
      newBalance: json['data']['new_balance'],
      playId: json['data']['play_id'],
      message: json['message'] ?? '',
    );
  }
}
```

---

## 🎨 Flujo de UX Recomendado

### 1. Pantalla Inicial
```dart
// Al entrar a la pantalla del Daily Game
Future<void> loadDailyGameStatus() async {
  setState(() => isLoading = true);
  
  try {
    final response = await http.get(
      Uri.parse('$baseUrl/api/v4/daily-game/status'),
      headers: {'Authorization': 'Bearer $token'},
    );
    
    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      final status = DailyGameStatus.fromJson(data['data']);
      
      setState(() {
        canPlay = status.canPlayToday;
        hasPlayed = status.hasPlayedToday;
        stats = status.stats;
        todaysReward = status.todaysReward;
      });
      
      // Mostrar UI apropiada
      if (!canPlay) {
        _showAlreadyPlayedMessage(todaysReward);
      }
    }
  } catch (e) {
    _showError('Error al cargar el juego');
  } finally {
    setState(() => isLoading = false);
  }
}
```

### 2. Usuario Elige Estrella
```dart
// Animación de selección de estrella (cliente-side)
void onStarTapped(int starIndex) async {
  if (!canPlay) {
    _showSnackBar('Ya jugaste hoy. Vuelve mañana.');
    return;
  }
  
  setState(() => isProcessing = true);
  
  // Animación de "revelación" (puedes hacer esto en Flutter)
  final result = await _revealStar(starIndex);
  
  // Llamar al backend con el resultado
  await claimReward(starIndex, result.lumisWon);
}

// Simulación de "revelación" (Frontend)
Future<StarResult> _revealStar(int starIndex) async {
  // Algoritmo para determinar qué estrella tiene qué premio
  // Puede ser aleatorio o basado en probabilidades
  
  final random = Random();
  final roll = random.nextDouble();
  
  int lumisWon;
  if (roll < 0.10) {
    lumisWon = 5; // 10% estrella dorada
  } else if (roll < 0.60) {
    lumisWon = 1; // 50% estrella normal
  } else {
    lumisWon = 0; // 40% estrella vacía
  }
  
  return StarResult(
    starId: 'star_$starIndex',
    lumisWon: lumisWon,
  );
}
```

### 3. Reclamar Recompensa
```dart
Future<void> claimReward(int starIndex, int lumisWon) async {
  try {
    final response = await http.post(
      Uri.parse('$baseUrl/api/v4/daily-game/claim'),
      headers: {
        'Authorization': 'Bearer $token',
        'Content-Type': 'application/json',
      },
      body: jsonEncode({
        'star_id': 'star_$starIndex',
        'lumis_won': lumisWon,
      }),
    );
    
    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      final result = DailyGameClaimResponse.fromJson(data);
      
      // Mostrar animación de celebración
      _showCelebration(result);
      
      // Actualizar balance del usuario
      _updateBalance(result.newBalance);
      
      // Actualizar estado
      setState(() {
        canPlay = false;
        hasPlayed = true;
        todaysReward = lumisWon;
      });
      
    } else if (response.statusCode == 409) {
      _showSnackBar('Ya jugaste hoy');
    } else {
      final error = jsonDecode(response.body);
      _showSnackBar(error['error']['message']);
    }
  } catch (e) {
    _showError('Error al reclamar recompensa');
  } finally {
    setState(() => isProcessing = false);
  }
}
```

### 4. Mostrar Resultado
```dart
void _showCelebration(DailyGameClaimResponse result) {
  // Animaciones según el tipo de premio
  if (result.lumisAdded == 5) {
    _showGoldenStarAnimation();
  } else if (result.lumisAdded == 1) {
    _showNormalStarAnimation();
  } else {
    _showEmptyStarAnimation();
  }
  
  // Mostrar mensaje personalizado
  ScaffoldMessenger.of(context).showSnackBar(
    SnackBar(
      content: Text(result.message),
      backgroundColor: _getColorByReward(result.lumisAdded),
      duration: Duration(seconds: 3),
    ),
  );
}

Color _getColorByReward(int lumis) {
  if (lumis == 5) return Colors.amber; // Dorado
  if (lumis == 1) return Colors.blue;  // Azul
  return Colors.grey;                   // Gris
}
```

---

## 🎲 Lógica de Probabilidades (Frontend)

### Opción 1: Cliente decide (Recomendado)
El cliente Flutter genera el resultado aleatorio y solo envía el resultado al backend.

**Ventajas**:
- ✅ UX más fluida (animación instantánea)
- ✅ No hay latencia de red antes de mostrar resultado
- ✅ Backend solo valida y registra

**Desventajas**:
- ⚠️ Vulnerable a modificación del cliente (pero backend valida)

```dart
class DailyGameLogic {
  static const double GOLDEN_STAR_PROBABILITY = 0.10; // 10%
  static const double NORMAL_STAR_PROBABILITY = 0.50; // 50%
  static const double EMPTY_STAR_PROBABILITY = 0.40;  // 40%
  
  static int calculateReward() {
    final random = Random();
    final roll = random.nextDouble();
    
    if (roll < GOLDEN_STAR_PROBABILITY) {
      return 5; // Estrella dorada
    } else if (roll < GOLDEN_STAR_PROBABILITY + NORMAL_STAR_PROBABILITY) {
      return 1; // Estrella normal
    } else {
      return 0; // Estrella vacía
    }
  }
}
```

### Opción 2: Backend decide (Más seguro)
Agregar un nuevo endpoint `/reveal` que devuelva el resultado antes de reclamar.

**No implementado en MVP**, pero puede agregarse en Fase 2:
```rust
// FUTURO: POST /api/v4/daily-game/reveal
// Response: { "lumis_won": 5, "star_id": "star_3" }
```

---

## 🔒 Seguridad

### ✅ Validaciones del Backend

1. **Autenticación**: Todos los endpoints requieren JWT válido
2. **Valores permitidos**: `lumis_won` ∈ {0, 1, 5}, `star_id` ∈ {star_0..star_8}
3. **Duplicados**: UNIQUE constraint en BD previene múltiples jugadas por día
4. **Zona horaria**: Usa hora de Panamá (UTC-5) para calcular "hoy"
5. **Transacciones**: Inserciones atómicas (jugada + acumulación)

### ⚠️ Consideraciones

**Cliente puede modificar `lumis_won`**:
- ✅ El backend **acepta** el valor enviado por el cliente
- ✅ Pero está **validado** (solo 0, 1, o 5)
- ✅ UNIQUE constraint previene múltiples intentos
- ⚠️ Si el cliente modifica el código, podría elegir siempre 5
  - **Mitigación**: Análisis de patrones sospechosos (Fase 2)
  - **Mitigación**: Rate limiting por IP (Fase 2)
  - **Mitigación**: Auditoría de usuarios con muchas estrellas doradas (Fase 2)

**Para producción**:
- Opción 1: Confiar en el cliente (MVP actual)
- Opción 2: Backend decide el resultado (más seguro, pero requiere endpoint adicional)
- Opción 3: Híbrido: Cliente genera seed, backend genera resultado determinístico

---

## 📱 Ejemplo Completo Flutter

```dart
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'dart:convert';
import 'dart:math';

class DailyGameScreen extends StatefulWidget {
  @override
  _DailyGameScreenState createState() => _DailyGameScreenState();
}

class _DailyGameScreenState extends State<DailyGameScreen> {
  static const String baseUrl = 'https://api.2factu.com';
  
  bool isLoading = true;
  bool canPlay = false;
  bool hasPlayed = false;
  bool isProcessing = false;
  
  int? todaysReward;
  DailyGameStats? stats;
  String? token; // Obtener del auth provider
  
  @override
  void initState() {
    super.initState();
    _loadDailyGameStatus();
  }
  
  Future<void> _loadDailyGameStatus() async {
    setState(() => isLoading = true);
    
    try {
      final response = await http.get(
        Uri.parse('$baseUrl/api/v4/daily-game/status'),
        headers: {'Authorization': 'Bearer $token'},
      );
      
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        final status = DailyGameStatus.fromJson(data['data']);
        
        setState(() {
          canPlay = status.canPlayToday;
          hasPlayed = status.hasPlayedToday;
          stats = status.stats;
          todaysReward = status.todaysReward;
        });
      }
    } catch (e) {
      _showError('Error al cargar el juego');
    } finally {
      setState(() => isLoading = false);
    }
  }
  
  Future<void> _onStarTapped(int starIndex) async {
    if (!canPlay) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Ya jugaste hoy. Vuelve mañana.')),
      );
      return;
    }
    
    setState(() => isProcessing = true);
    
    // Calcular recompensa (cliente-side)
    final lumisWon = _calculateReward();
    
    // Animación de revelación
    await _showRevealAnimation(starIndex, lumisWon);
    
    // Reclamar en backend
    await _claimReward(starIndex, lumisWon);
  }
  
  int _calculateReward() {
    final random = Random();
    final roll = random.nextDouble();
    
    if (roll < 0.10) return 5;      // 10% dorada
    if (roll < 0.60) return 1;      // 50% normal
    return 0;                        // 40% vacía
  }
  
  Future<void> _showRevealAnimation(int starIndex, int lumisWon) async {
    // TODO: Implementar animación bonita
    await Future.delayed(Duration(seconds: 1));
  }
  
  Future<void> _claimReward(int starIndex, int lumisWon) async {
    try {
      final response = await http.post(
        Uri.parse('$baseUrl/api/v4/daily-game/claim'),
        headers: {
          'Authorization': 'Bearer $token',
          'Content-Type': 'application/json',
        },
        body: jsonEncode({
          'star_id': 'star_$starIndex',
          'lumis_won': lumisWon,
        }),
      );
      
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body);
        final result = DailyGameClaimResponse.fromJson(data);
        
        _showSuccessMessage(result.message);
        
        setState(() {
          canPlay = false;
          hasPlayed = true;
          todaysReward = lumisWon;
        });
        
        // TODO: Actualizar balance global
        
      } else if (response.statusCode == 409) {
        _showError('Ya jugaste hoy');
      } else {
        final error = jsonDecode(response.body);
        _showError(error['error']['message']);
      }
    } catch (e) {
      _showError('Error al reclamar recompensa');
    } finally {
      setState(() => isProcessing = false);
    }
  }
  
  void _showSuccessMessage(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        backgroundColor: Colors.green,
      ),
    );
  }
  
  void _showError(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        backgroundColor: Colors.red,
      ),
    );
  }
  
  @override
  Widget build(BuildContext context) {
    if (isLoading) {
      return Center(child: CircularProgressIndicator());
    }
    
    return Scaffold(
      appBar: AppBar(title: Text('Constelación Diaria')),
      body: Column(
        children: [
          // Estadísticas
          if (stats != null) _buildStats(),
          
          // Grid de estrellas
          Expanded(
            child: GridView.builder(
              gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                crossAxisCount: 3,
                childAspectRatio: 1,
              ),
              itemCount: 9,
              itemBuilder: (context, index) {
                return GestureDetector(
                  onTap: () => _onStarTapped(index),
                  child: _buildStar(index),
                );
              },
            ),
          ),
          
          // Mensaje de estado
          if (!canPlay) _buildAlreadyPlayedMessage(),
        ],
      ),
    );
  }
  
  Widget _buildStats() {
    return Card(
      child: Padding(
        padding: EdgeInsets.all(16),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceAround,
          children: [
            _buildStatItem('Jugadas', '${stats!.totalPlays}'),
            _buildStatItem('Lümis', '${stats!.totalLumisWon}'),
            _buildStatItem('⭐ Doradas', '${stats!.goldenStarsCaptured}'),
          ],
        ),
      ),
    );
  }
  
  Widget _buildStatItem(String label, String value) {
    return Column(
      children: [
        Text(value, style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold)),
        Text(label, style: TextStyle(fontSize: 12, color: Colors.grey)),
      ],
    );
  }
  
  Widget _buildStar(int index) {
    return Container(
      margin: EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: canPlay ? Colors.blue[100] : Colors.grey[300],
        shape: BoxShape.circle,
      ),
      child: Icon(
        Icons.star,
        size: 50,
        color: canPlay ? Colors.amber : Colors.grey,
      ),
    );
  }
  
  Widget _buildAlreadyPlayedMessage() {
    return Container(
      padding: EdgeInsets.all(16),
      color: Colors.orange[100],
      child: Row(
        children: [
          Icon(Icons.info, color: Colors.orange),
          SizedBox(width: 8),
          Expanded(
            child: Text(
              'Ya jugaste hoy. Ganaste $todaysReward Lümis. Vuelve mañana.',
              style: TextStyle(color: Colors.orange[900]),
            ),
          ),
        ],
      ),
    );
  }
}

// Modelos (ya definidos arriba)
class DailyGameStatus { /* ... */ }
class DailyGameStats { /* ... */ }
class DailyGameClaimResponse { /* ... */ }
```

---

## 🎯 Checklist de Integración

### Backend (Rust)
- [x] Endpoints implementados
- [x] Autenticación JWT
- [x] Validaciones
- [x] Integración con rewards
- [x] Testing completo

### Frontend (Flutter)
- [ ] Crear pantalla `DailyGameScreen`
- [ ] Implementar modelos Dart
- [ ] Implementar servicio HTTP
- [ ] Implementar lógica de probabilidades
- [ ] Diseñar UI de estrellas
- [ ] Implementar animaciones
- [ ] Testing en desarrollo
- [ ] Testing en producción

---

## 📞 Soporte

**Issues conocidos**: Ninguno  
**Documentación adicional**: Ver `DAILY_GAME_IMPLEMENTATION_SUMMARY.md`  
**Testing**: Ver `DAILY_GAME_TESTING_RESULTS.md`

---

**Última actualización**: 2025-10-13  
**Versión API**: v4  
**Estado**: ✅ Listo para integración
