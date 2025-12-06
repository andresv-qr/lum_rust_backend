# 📱 Gamification API Reference for Flutter Clients

> **Version:** 4.0
> **Last Updated:** November 30, 2025
> **Base URL:** `/api/v4/gamification`
> **Auth:** Bearer Token required (JWT)

This document details the endpoints required to implement the full Gamification experience in the Flutter application.

---

## 1. 🏠 User Dashboard (Main Endpoint)

This is the **primary endpoint** for the Gamification Home Screen. It aggregates all critical user status data into a single high-performance call.

- **Endpoint:** `GET /dashboard`
- **Use Case:** Call on App Start, Pull-to-Refresh, and after successful invoice uploads.

### Response Example (200 OK)

```json
{
  "success": true,
  "data": {
    "user_id": 1099,
    "email": "user@example.com",
    
    // --- Level & Progression ---
    "current_level": 12,
    "level_name": "Nebulosa",
    "level_color": "#303F9F",
    "level_description": "Nebulosa - Basado en 449 facturas",
    "total_lumis": 449, // This is actually Total Invoices (XP)
    
    // --- Next Level Progress ---
    "next_level_name": "Galaxia",
    "lumis_to_next_level": 1,
    "next_level_hint": "Faltan 1 facturas para Galaxia",
    
    // --- Wallet ---
    "wallet_balance": 1250, // Spendable Lumis
    
    // --- Active Streaks (Rachas) ---
    "active_streaks": [
      {
        "type": "daily_login",
        "current": 3,
        "max": 7
      },
      {
        "type": "consistent_month",
        "current": 2,
        "max": 4
      }
    ],
    
    // --- Counters ---
    "active_missions_count": 2,
    "completed_missions_count": 0,
    "total_achievements": 5,
    
    // --- Level Benefits (For UI display) ---
    "level_benefits": {
      "lumi_multiplier": 1.55,
      "description": "Cuna de estrellas."
    }
  },
  "request_id": "uuid-string",
  "execution_time": 12
}
```

---

## 2. 👣 Track User Action

Use this endpoint to report user behaviors that trigger gamification logic (e.g., Daily Login).

- **Endpoint:** `POST /track`
- **Use Case:** `AppLifecycleState.resumed` (Daily Login), Survey Completion, Share App.

### Request Body

```json
{
  "action": "daily_login", // Options: 'daily_login', 'survey_complete', 'referral_complete'
  "channel": "mobile_app",
  "metadata": {
    "device_os": "android",
    "app_version": "2.1.0"
  }
}
```

### Response Example (200 OK)

```json
{
  "success": true,
  "data": {
    "lumis_earned": 0, // > 0 if reward triggered
    "xp_earned": 0,
    "current_level": 12,
    "level_name": "Nebulosa",
    "message": "Login registrado",
    
    // Updated streak info immediately
    "streaks": {
      "daily_login": {
        "current": 4,
        "next_milestone": 7
      }
    },
    
    "achievements_unlocked": [],
    "active_events": []
  }
}
```

---

## 3. 🎯 Missions & Quests

Returns the list of active and completed missions for the user.

- **Endpoint:** `GET /missions`

### Response Example

```json
{
  "success": true,
  "data": [
    {
      "mission_code": "upload_5_invoices_week",
      "mission_name": "Facturador Semanal",
      "description": "Sube 5 facturas esta semana",
      "current_progress": 3,
      "target_count": 5,
      "progress_percentage": 60.0,
      "reward_lumis": 50,
      "due_date": "2025-12-07",
      "status": "active" // 'active', 'completed', 'expired'
    }
  ]
}
```

---

## 4. 🏆 Leaderboard

Returns the top users based on Total Invoices (XP).

- **Endpoint:** `GET /leaderboard`
- **Query Params:** `?limit=50&offset=0`

### Response Example

```json
{
  "success": true,
  "data": [
    {
      "rank": 1,
      "user_id": 45,
      "username": "SuperUser",
      "total_lumis": 1200, // XP
      "current_level": 17,
      "level_name": "Omnisciente"
    },
    {
      "rank": 2,
      "user_id": 1099,
      "username": "You",
      "total_lumis": 449,
      "current_level": 12,
      "level_name": "Nebulosa"
    }
  ]
}
```

---

## 5. 🧩 Flutter Integration Guide (Riverpod)

### Architecture Recommendation

We use **Riverpod** with `codegen` for type-safe, reactive state management.

#### 1. Models (Freezed)

```dart
// models/gamification/user_dashboard.dart
@freezed
class UserDashboard with _$UserDashboard {
  const factory UserDashboard({
    required int currentLevel,
    required String levelName,
    required String levelColor,
    required int totalLumis,
    required int lumisToNextLevel,
    required List<StreakInfo> activeStreaks,
    // ... map all JSON fields
  }) = _UserDashboard;
  
  factory UserDashboard.fromJson(Map<String, dynamic> json) => _$UserDashboardFromJson(json);
}
```

#### 2. Repository

```dart
// repositories/gamification_repository.dart
@Riverpod(keepAlive: true)
GamificationRepository gamificationRepository(GamificationRepositoryRef ref) {
  return GamificationRepository(ref.watch(apiClientProvider));
}

class GamificationRepository {
  final ApiClient _client;
  
  Future<UserDashboard> getDashboard() async {
    final response = await _client.get('/api/v4/gamification/dashboard');
    return UserDashboard.fromJson(response.data['data']);
  }
  
  Future<TrackResponse> trackAction(String action) async {
    // ... implementation
  }
}
```

#### 3. Controllers (Providers)

```dart
// features/gamification/controllers/dashboard_controller.dart
@riverpod
class GamificationDashboardController extends _$GamificationDashboardController {
  @override
  FutureOr<UserDashboard> build() {
    return ref.watch(gamificationRepositoryProvider).getDashboard();
  }
  
  Future<void> refresh() async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() => ref.read(gamificationRepositoryProvider).getDashboard());
  }
}
```

#### 4. UI Implementation (Streaks)

```dart
class StreakCard extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dashboardAsync = ref.watch(gamificationDashboardControllerProvider);
    
    return dashboardAsync.when(
      data: (dashboard) {
        final loginStreak = dashboard.activeStreaks.firstWhere(
          (s) => s.type == 'daily_login',
          orElse: () => StreakInfo.empty(),
        );
        
        return Card(
          color: HexColor(dashboard.levelColor),
          child: Column(
            children: [
              Text("Nivel: ${dashboard.levelName}"),
              Text("Racha: ${loginStreak.current} días"),
            ],
          ),
        );
      },
      loading: () => const CircularProgressIndicator(),
      error: (err, stack) => Text('Error: $err'),
    );
  }
}
```
