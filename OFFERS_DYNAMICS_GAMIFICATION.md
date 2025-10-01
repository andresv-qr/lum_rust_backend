````markdown
# 🎯 Sistema de Ofertas y Redención Lümis - Documentación de Pantallas Flutter

## 📚 Índice de Documentación

1. [Componentes Reutilizables](#-componentes-reutilizables)
2. [Patrones de Diseño Responsivo](#-patrones-de-diseño-responsivo)
3. [Animaciones y Transiciones](#-animaciones-y-transiciones)
4. [Notificaciones In-App](#-notificaciones-in-app)
5. [Internacionalización](#-internacionalización)
6. [Temas y Personalización](#-temas-y-personalización)
7. [Optimización de Rendimiento](#-optimización-de-rendimiento)

## 🔧 Componentes Reutilizables

### AnimatedLumisCounter
```dart
class AnimatedLumisCounter extends StatelessWidget {
  final int lumis;
  final double fontSize;
  final Color? color;
  
  Widget build(context) {
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: lumis.toDouble()),
      duration: Duration(milliseconds: 1500),
      curve: Curves.easeOutCubic,
      builder: (context, value, child) {
        return Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.star,
              color: color ?? Colors.amber,
              size: fontSize * 1.2,
            ),
            SizedBox(width: 4),
            Text(
              "Ł ${value.toInt().toString().replaceAllMapped(
                RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))'),
                (Match m) => '${m[1]},'
              )}",
              style: TextStyle(
                fontSize: fontSize,
                fontWeight: FontWeight.bold,
                color: color,
              ),
            ),
          ],
        );
      },
    );
  }
}
```

### ShimmerLoading
```dart
class ShimmerLoading extends StatelessWidget {
  final Widget child;
  
  Widget build(context) {
    return Shimmer.fromColors(
      baseColor: Colors.grey[300]!,
      highlightColor: Colors.grey[100]!,
      child: child,
    );
  }
}
```

### EmptyStateWidget
```dart
class EmptyStateWidget extends StatelessWidget {
  final String title;
  final String subtitle;
  final IconData icon;
  final VoidCallback? onAction;
  final String? actionLabel;
  
  Widget build(context) {
    return Center(
      child: Padding(
        padding: EdgeInsets.all(32),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              icon,
              size: 80,
              color: Colors.grey[400],
            ),
            SizedBox(height: 16),
            Text(
              title,
              style: Theme.of(context).textTheme.headlineSmall,
              textAlign: TextAlign.center,
            ),
            SizedBox(height: 8),
            Text(
              subtitle,
              style: TextStyle(color: Colors.grey[600]),
              textAlign: TextAlign.center,
            ),
            if (onAction != null) ...[
              SizedBox(height: 24),
              ElevatedButton(
                onPressed: onAction,
                child: Text(actionLabel ?? "Explorar"),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
```

## 📐 Responsive Design Patterns

### Adaptive Layout
```dart
class AdaptiveOfferGrid extends StatelessWidget {
  Widget build(context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        // Calcular columnas basado en el ancho
        int crossAxisCount = 2;
        if (constraints.maxWidth > 600) crossAxisCount = 3;
        if (constraints.maxWidth > 900) crossAxisCount = 4;
        if (constraints.maxWidth > 1200) crossAxisCount = 5;
        
        return GridView.builder(
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: crossAxisCount,
            childAspectRatio: 0.75,
            crossAxisSpacing: 16,
            mainAxisSpacing: 16,
          ),
          itemBuilder: (context, index) => OfferCard(
            offer: offers[index],
          ),
        );
      },
    );
  }
}
```

## 🎭 Animaciones y Transiciones

### Hero Animation para Ofertas
```dart
// En la lista
Hero(
  tag: "offer_${offer.id}",
  child: OfferCard(offer: offer),
)

// En el detalle
Hero(
  tag: "offer_${offer.id}",
  child: OfferDetailHeader(offer: offer),
)
```

### Staggered Animation
```dart
class StaggeredOfferList extends StatelessWidget {
  Widget build(context) {
    return AnimationLimiter(
      child: ListView.builder(
        itemCount: offers.length,
        itemBuilder: (context, index) {
          return AnimationConfiguration.staggeredList(
            position: index,
            duration: const Duration(milliseconds: 375),
            child: SlideAnimation(
              verticalOffset: 50.0,
              child: FadeInAnimation(
                child: OfferListTile(offer: offers[index]),
              ),
            ),
          );
        },
      ),
    );
  }
}
```

## 🔔 Notificaciones In-App

### Toast Notifications
```dart
class LumisToast {
  static void showSuccess(BuildContext context, String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Icon(Icons.check_circle, color: Colors.white),
            SizedBox(width: 12),
            Expanded(child: Text(message)),
          ],
        ),
        backgroundColor: Colors.green,
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
        ),
      ),
    );
  }
  
  static void showError(BuildContext context, String message) {
    // Similar implementation con colores de error
  }
}
```

## 🌐 Internacionalización

### Configuración de idiomas
```dart
class AppLocalizations {
  static const supportedLocales = [
    Locale('es', 'ES'),
    Locale('en', 'US'),
    Locale('pt', 'BR'),
  ];
  
  static String of(BuildContext context, String key) {
    // Implementación de traducciones
  }
}
```

## 🎨 Temas y Personalización

### Dynamic Theming
```dart
class LumisTheme {
  static ThemeData lightTheme = ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: Color(0xFF6B46C1), // Purple
      brightness: Brightness.light,
    ),
    cardTheme: CardTheme(
      elevation: 2,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
      ),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
        padding: EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      ),
    ),
  );
  
  static ThemeData darkTheme = ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: Color(0xFF6B46C1),
      brightness: Brightness.dark,
    ),
    // Configuración similar para dark mode
  );
}
```

## 📊 Performance Optimizations

### Image Caching Strategy
```dart
class OptimizedNetworkImage extends StatelessWidget {
  final String imageUrl;
  final double? width;
  final double? height;
  
  Widget build(context) {
    return CachedNetworkImage(
      imageUrl: imageUrl,
      width: width,
      height: height,
      memCacheWidth: width?.toInt(),
      memCacheHeight: height?.toInt(),
      placeholder: (context, url) => ShimmerLoading(
        child: Container(
          color: Colors.grey[300],
        ),
      ),
      errorWidget: (context, url, error) => Icon(Icons.error),
      fadeInDuration: Duration(milliseconds: 300),
    );
  }
}
```

### Lazy Loading
```dart
class LazyLoadOffers extends StatefulWidget {
  @override
  State<LazyLoadOffers> createState() => _LazyLoadOffersState();
}

class _LazyLoadOffersState extends State<LazyLoadOffers> {
  final ScrollController _scrollController = ScrollController();
  final List<Offer> _offers = [];
  bool _isLoading = false;
  int _page = 1;
  
  @override
  void initState() {
    super.initState();
    _loadMore();
    _scrollController.addListener(() {
      if (_scrollController.position.pixels ==
          _scrollController.position.maxScrollExtent) {
        _loadMore();
      }
    });
  }
  
  Future<void> _loadMore() async {
    if (_isLoading) return;
    
    setState(() => _isLoading = true);
    
    final newOffers = await OfferService.getOffers(page: _page);
    
    setState(() {
      _offers.addAll(newOffers);
      _page++;
      _isLoading = false;
    });
  }
  
  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      controller: _scrollController,
      itemCount: _offers.length + (_isLoading ? 1 : 0),
      itemBuilder: (context, index) {
        if (index == _offers.length) {
          return Center(
            child: CircularProgressIndicator(),
          );
        }
        return OfferCard(offer: _offers[index]);
      },
    );
  }
}
```

---

*Última actualización: Diciembre 2024*
*Versión: 1.0.0*

# 🎯 Sistema de Ofertas y Redención Lümis - Documentación de Dinámicas y Gamificación

## 📚 Índice de Documentación

1. [Mecánicas de Colaboración](#-mecánicas-de-colaboración)
2. [Sistema de Recompensas Dinámicas](#-sistema-de-recompensas-dinámicas)
3. [Personalización y Avatares](#-personalización-y-avatares)
4. [Integración Social](#-integración-social)
5. [Eventos Estacionales](#-eventos-estacionales)
6. [Cuestionamiento del Sistema](#-cuestionamiento-del-sistema)
7. [Implementación Técnica](#-implementación-técnica)

## 6. Mecánicas de Colaboración

#### Ofertas Grupales
```yaml
Compra Grupal:
  mecánica:
    - Usuarios se unen para desbloquear mejor precio
    - Tiempo límite para alcanzar meta
    - Precio disminuye según participantes
  
  ejemplo_Netflix_Anual:
    participantes_necesarios: 4
    precio_individual: 2000 Lümis
    precios_grupales:
      2_personas: 1800 Lümis c/u (10% desc)
      3_personas: 1600 Lümis c/u (20% desc)
      4_personas: 1400 Lümis c/u (30% desc)
    tiempo_limite: 48 horas
    recompensa_organizador: 200 Lümis bonus

Retos Comunitarios:
  "Meta del Millón":
    objetivo: "1,000,000 Lümis canjeados por la comunidad"
    tiempo: 1 semana
    recompensa_global:
      - Todos reciben 100 Lümis
      - Desbloqueo de ofertas exclusivas
      - Sorteo especial para participantes
    
  "Flash Mob de Ofertas":
    trigger: Notificación sorpresa
    duración: 1 hora
    objetivo: 500 usuarios activos simultáneos
    recompensa:
      - 50% descuento en siguiente redención
      - Badge "Flash Mob"
```

## 7. Sistema de Recompensas Dinámicas

#### Ruleta Diaria
```typescript
interface DailyWheel {
  segments: [
    { prize: "10 Lümis", probability: 0.30 },
    { prize: "25 Lümis", probability: 0.25 },
    { prize: "50 Lümis", probability: 0.20 },
    { prize: "100 Lümis", probability: 0.10 },
    { prize: "Descuento 20%", probability: 0.08 },
    { prize: "Ticket Sorteo", probability: 0.05 },
    { prize: "Gift Card $50", probability: 0.015 },
    { prize: "1000 Lümis", probability: 0.005 }
  ];
  
  rules: {
    free_spins_per_day: 1,
    additional_spin_cost: 50, // Lümis
    max_paid_spins: 3,
    bonus_spin_triggers: ["complete_daily_challenge", "refer_friend"]
  };
}
```

#### Cofres del Tesoro
```yaml
Sistema de Cofres:
  Cofre_Bronce:
    costo: 100 Lümis
    contenido_posible:
      - 50-150 Lümis (60%)
      - Descuento 10% (20%)
      - Ticket sorteo (15%)
      - Badge común (5%)
  
  Cofre_Plata:
    costo: 500 Lümis
    contenido_posible:
      - 300-700 Lümis (50%)
      - Descuento 25% (25%)
      - 2 Tickets sorteo (15%)
      - Badge raro (8%)
      - Gift card $100 (2%)
  
  Cofre_Oro:
    costo: 2000 Lümis
    contenido_posible:
      - 1500-3000 Lümis (40%)
      - Descuento 50% (20%)
      - 5 Tickets sorteo (20%)
      - Badge épico (10%)
      - Gift card $500 (8%)
      - Experiencia VIP (2%)
  
  Cofre_Legendario:
    obtención: Solo eventos especiales
    contenido_garantizado:
      - Mínimo 5000 Lümis
      - Badge legendario
      - Acceso VIP 1 mes
      - Gift card $1000+
```

## 8. Personalización y Avatares

#### Sistema de Avatares
```yaml
Personalización:
  Avatares_Base:
    - Se desbloquean con niveles
    - Personalización de colores gratis
    
  Accesorios:
    Sombreros:
      - Gorra básica: 100 Lümis
      - Sombrero elegante: 500 Lümis
      - Corona (Nivel Leyenda): Gratis al alcanzar
    
    Marcos:
      - Marco simple: 200 Lümis
      - Marco animado: 1000 Lümis
      - Marco exclusivo evento: Solo en eventos
    
    Efectos:
      - Brillo: 300 Lümis
      - Fuego: 800 Lümis
      - Arcoíris: 1500 Lümis

  Temas_Perfil:
    - Tema oscuro/claro: Gratis
    - Temas estacionales: 500 Lümis
    - Temas premium: 2000 Lümis
```

## 9. Integración Social

#### Clubs y Comunidades
```yaml
Clubs_de_Usuarios:
  creación:
    costo: 1000 Lümis
    requisitos: Nivel mínimo Aventurero
    
  beneficios_miembros:
    - Chat exclusivo
    - Desafíos de club
    - Descuentos grupales
    - Tabla de posiciones interna
    
  niveles_club:
    Bronce: 10 miembros
    Plata: 50 miembros
    Oro: 100 miembros
    Diamante: 500+ miembros
    
  recompensas_club:
    semanal:
      top_1: 500 Lümis para cada miembro
      top_3: 300 Lümis para cada miembro
      top_10: 100 Lümis para cada miembro

Comparación_Social:
  funciones:
    - Ver actividad de amigos
    - Comparar ahorros
    - Compartir logros
    - Regalar Lümis (con límite)
    - Desafíos 1v1
```

## 10. Eventos Estacionales

#### Calendario de Eventos
```yaml
Enero - "Año Nuevo, Nuevos Ahorros":
  duración: Todo el mes
  mecánicas:
    - Resoluciones de ahorro
    - Bonus 2x en primera semana
    - Reset de beneficios premium
  recompensas:
    - Badge "Fresh Start"
    - 500 Lümis por completar resoluciones

Febrero - "Mes del Amor":
  14_febrero:
    - Ofertas 2x1 especiales
    - Gift cards con bonus
    - Sorteo viaje romántico
  mecánicas:
    - Regala Lümis sin límite
    - Ofertas para compartir
  
Mayo - "Día de las Madres":
  especial:
    - Gift cards de spa y belleza
    - Experiencias para mamá
    - Descuentos en flores y regalos
    
Septiembre - "Mes Patrio":
  15-16_septiembre:
    - Ofertas en restaurantes mexicanos
    - Sorteos temáticos
    - Lümis bonus en comercios nacionales
    
Octubre - "Halloween":
  31_octubre:
    - Caza del tesoro virtual
    - Ofertas "terroríficamente buenas"
    - Disfraces para avatar
    
Noviembre - "Buen Fin":
  duración: 4 días
  mecánicas:
    - Ofertas flash cada hora
    - Multiplicador 3x Lümis
    - Sorteos cada 6 horas
  premios:
    - Hasta 70% descuento
    - Gift cards con 30% bonus
    
Diciembre - "Navidad Lümis":
  calendario_adviento:
    - 24 días de sorpresas
    - Regalos diarios progresivos
    - Gran premio día 24
  posada_virtual:
    - Minijuegos navideños
    - Intercambio de regalos virtual
```

## 🎯 Cuestionamiento del Sistema

### ❓ Análisis Crítico de Escalabilidad

#### Problemas Potenciales:
1. **Complejidad Excesiva**
   - ⚠️ Demasiadas mecánicas pueden abrumar a usuarios nuevos
   - ✅ Solución: Onboarding progresivo, desbloquear features gradualmente

2. **Inflación de Lümis**
   - ⚠️ Dar demasiados Lümis puede devaluar la moneda
   - ✅ Solución: Economía balanceada con "sinks" (cofres, personalización)

3. **Mantenimiento de Engagement**
   - ⚠️ Usuarios pueden perder interés después del honeymoon
   - ✅ Solución: Contenido dinámico, eventos rotativos, PvP

4. **Costo de Implementación**
   - ⚠️ Sistema complejo = más desarrollo y mantenimiento
   - ✅ Solución: Implementación por fases, MVP con features core

5. **Balance de Recompensas**
   - ⚠️ Difícil balancear para que sea justo pero rentable
   - ✅ Solución: A/B testing continuo, ajustes basados en data

### 💡 Simplificaciones Recomendadas

#### MVP de Gamificación:
```yaml
Fase 1 (Lanzamiento):
  - Sistema de niveles básico (3 niveles)
  - Desafíos diarios simples
  - Rachas de login
  - Badges básicos (10 tipos)
  
Fase 2 (Mes 3):
  - Desafíos semanales
  - Ruleta diaria
  - Sistema de referidos
  - Más niveles (5 total)
  
Fase 3 (Mes 6):
  - Eventos estacionales
  - Clubs
  - Torneos
  - Personalización completa
```

### 🔧 Personalización vs Simplicidad

#### Sistema Modular:
```typescript
interface ModularGamification {
  core: {
    // Siempre activo
    levels: true,
    daily_login: true,
    basic_achievements: true
  },
  
  optional_modules: {
    // Activar según tipo de usuario
    competitive: ["tournaments", "leaderboards", "pvp"],
    social: ["clubs", "friend_challenges", "sharing"],
    collector: ["badges", "avatars", "themes"],
    casual: ["daily_wheel", "simple_challenges"]
  },
  
  user_preference: {
    // Usuario elige su estilo
    show_rankings: boolean,
    enable_notifications: boolean,
    participate_in_events: boolean
  }
}
```

## 📊 Métricas de Éxito de Gamificación

### KPIs Principales:
```yaml
Engagement:
  - DAU/MAU ratio objetivo: >25%
  - Sesiones por día: >2
  - Tiempo en app: >5 min/sesión
  
Monetización:
  - Conversión a primera redención: >40%
  - LTV incremento: +30%
  - Churn reduction: -20%
  
Virality:
  - K-factor: >1.2
  - Referral rate: >15%
  - Social shares: >10% usuarios
  
Retention:
  - D1: >60%
  - D7: >40%
  - D30: >25%
```

## 🚀 Implementación Técnica

### Backend Requirements:
```yaml
Servicios Necesarios:
  - Achievement Service: Tracking de logros
  - Leaderboard Service: Rankings en tiempo real
  - Event Service: Gestión de eventos temporales
  - Reward Service: Distribución de premios
  - Analytics Service: Tracking de comportamiento

Base de Datos:
  Nuevas Tablas:
    - user_achievements
    - user_streaks
    - daily_challenges
    - tournament_participants
    - club_members
    - user_avatars
    
  Optimizaciones:
    - Cache Redis para leaderboards
    - Índices para queries frecuentes
    - Particionamiento por fecha
```

### Frontend Implementation:
```dart
// Ejemplo de widget de progreso de nivel
class LevelProgressBar extends StatelessWidget {
  final int currentXP;
  final int nextLevelXP;
  final int level;
  
  @override
  Widget build(BuildContext context) {
    final progress = currentXP / nextLevelXP;
    
    return Container(
      padding: EdgeInsets.all(16),
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: [Colors.purple[700]!, Colors.purple[900]!],
        ),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('Nivel $level', style: TextStyle(color: Colors.white)),
              Text('${currentXP}/${nextLevelXP} XP', 
                style: TextStyle(color: Colors.white70)),
            ],
          ),
          SizedBox(height: 8),
          LinearProgressIndicator(
            value: progress,
            backgroundColor: Colors.white24,
            valueColor: AlwaysStoppedAnimation(Colors.amber),
            minHeight: 8,
          ),
        ],
      ),
    );
  }
}
```

## 🎮 Conclusión

El sistema de gamificación propuesto es **ambicioso pero implementable por fases**. La clave está en:

1. **Comenzar simple**: MVP con mecánicas core probadas
2. **Iterar basado en datos**: A/B testing constante
3. **Escuchar a usuarios**: Feedback loops cortos
4. **Balancear diversión y negocio**: ROI positivo manteniendo engagement
5. **Personalización progresiva**: No forzar todo a todos

**Recomendación Final**: Implementar en 3 fases de 2 meses cada una, midiendo impacto en cada fase antes de continuar.

---

*Última actualización: Diciembre 2024*
*Versión: 1.0.0*
````