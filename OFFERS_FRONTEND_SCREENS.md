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