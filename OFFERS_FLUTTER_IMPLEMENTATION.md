# 📱 Implementación Flutter - Sistema de Ofertas

## 🎯 Arquitectura de la App

### Clean Architecture + Riverpod
```dart
lib/
├── core/
│   ├── constants/
│   ├── errors/
│   ├── network/
│   └── utils/
├── features/
│   ├── offers/
│   │   ├── data/
│   │   │   ├── datasources/
│   │   │   ├── models/
│   │   │   └── repositories/
│   │   ├── domain/
│   │   │   ├── entities/
│   │   │   ├── repositories/
│   │   │   └── usecases/
│   │   └── presentation/
│   │       ├── providers/
│   │       ├── screens/
│   │       └── widgets/
│   └── [other features]/
└── main.dart
```

## 📦 Implementación de Providers

### Offer Provider con Riverpod
```dart
// lib/features/offers/presentation/providers/offers_provider.dart

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:dio/dio.dart';

// API Client Provider
final dioProvider = Provider<Dio>((ref) {
  final dio = Dio(BaseOptions(
    baseUrl: 'https://api.lumis.com/v1',
    connectTimeout: const Duration(seconds: 5),
    receiveTimeout: const Duration(seconds: 3),
  ));
  
  dio.interceptors.add(AuthInterceptor(ref));
  dio.interceptors.add(LogInterceptor());
  
  return dio;
});

// Offers Repository Provider
final offersRepositoryProvider = Provider<OffersRepository>((ref) {
  return OffersRepositoryImpl(
    dio: ref.watch(dioProvider),
  );
});

// Offers List Provider with Pagination
final offersListProvider = StateNotifierProvider<OffersNotifier, OffersState>((ref) {
  return OffersNotifier(
    repository: ref.watch(offersRepositoryProvider),
  );
});

// Offers State
class OffersState {
  final List<Offer> offers;
  final bool isLoading;
  final bool hasMore;
  final String? error;
  final OfferFilters filters;
  final int currentPage;
  
  const OffersState({
    this.offers = const [],
    this.isLoading = false,
    this.hasMore = true,
    this.error,
    this.filters = const OfferFilters(),
    this.currentPage = 1,
  });
  
  OffersState copyWith({
    List<Offer>? offers,
    bool? isLoading,
    bool? hasMore,
    String? error,
    OfferFilters? filters,
    int? currentPage,
  }) {
    return OffersState(
      offers: offers ?? this.offers,
      isLoading: isLoading ?? this.isLoading,
      hasMore: hasMore ?? this.hasMore,
      error: error,
      filters: filters ?? this.filters,
      currentPage: currentPage ?? this.currentPage,
    );
  }
}

// Offers Notifier
class OffersNotifier extends StateNotifier<OffersState> {
  final OffersRepository repository;
  
  OffersNotifier({required this.repository}) : super(const OffersState()) {
    loadOffers();
  }
  
  Future<void> loadOffers({bool refresh = false}) async {
    if (state.isLoading) return;
    
    if (refresh) {
      state = state.copyWith(
        offers: [],
        currentPage: 1,
        hasMore: true,
      );
    }
    
    state = state.copyWith(isLoading: true, error: null);
    
    try {
      final result = await repository.getOffers(
        page: state.currentPage,
        filters: state.filters,
      );
      
      state = state.copyWith(
        offers: [...state.offers, ...result.offers],
        hasMore: result.hasMore,
        currentPage: state.currentPage + 1,
        isLoading: false,
      );
    } catch (e) {
      state = state.copyWith(
        isLoading: false,
        error: e.toString(),
      );
    }
  }
  
  void updateFilters(OfferFilters filters) {
    state = state.copyWith(filters: filters);
    loadOffers(refresh: true);
  }
  
  Future<void> toggleFavorite(String offerId) async {
    final index = state.offers.indexWhere((o) => o.id == offerId);
    if (index == -1) return;
    
    final offer = state.offers[index];
    final newOffer = offer.copyWith(isFavorite: !offer.isFavorite);
    
    final newOffers = [...state.offers];
    newOffers[index] = newOffer;
    state = state.copyWith(offers: newOffers);
    
    try {
      if (newOffer.isFavorite) {
        await repository.addToFavorites(offerId);
      } else {
        await repository.removeFromFavorites(offerId);
      }
    } catch (e) {
      // Revert on error
      newOffers[index] = offer;
      state = state.copyWith(offers: newOffers);
    }
  }
}

// Selected Offer Provider for Detail View
final selectedOfferProvider = FutureProvider.family<Offer, String>((ref, offerId) async {
  final repository = ref.watch(offersRepositoryProvider);
  return repository.getOfferDetail(offerId);
});
```

## 🎨 Implementación de Pantallas

### Home Screen con Infinite Scroll
```dart
// lib/features/offers/presentation/screens/offers_home_screen.dart

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class OffersHomeScreen extends ConsumerStatefulWidget {
  const OffersHomeScreen({super.key});
  
  @override
  ConsumerState<OffersHomeScreen> createState() => _OffersHomeScreenState();
}

class _OffersHomeScreenState extends ConsumerState<OffersHomeScreen> {
  final ScrollController _scrollController = ScrollController();
  
  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
  }
  
  void _onScroll() {
    if (_scrollController.position.pixels >= 
        _scrollController.position.maxScrollExtent * 0.8) {
      ref.read(offersListProvider.notifier).loadOffers();
    }
  }
  
  @override
  Widget build(BuildContext context) {
    final offersState = ref.watch(offersListProvider);
    
    return Scaffold(
      body: CustomScrollView(
        controller: _scrollController,
        slivers: [
          // Custom App Bar with Search
          SliverAppBar(
            floating: true,
            snap: true,
            expandedHeight: 120,
            flexibleSpace: FlexibleSpaceBar(
              background: Container(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                    colors: [
                      Theme.of(context).primaryColor,
                      Theme.of(context).primaryColor.withOpacity(0.7),
                    ],
                  ),
                ),
                child: SafeArea(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        // Search Bar
                        GestureDetector(
                          onTap: () => Navigator.pushNamed(context, '/search'),
                          child: Container(
                            height: 48,
                            decoration: BoxDecoration(
                              color: Colors.white,
                              borderRadius: BorderRadius.circular(24),
                              boxShadow: [
                                BoxShadow(
                                  color: Colors.black.withOpacity(0.1),
                                  blurRadius: 10,
                                  offset: const Offset(0, 2),
                                ),
                              ],
                            ),
                            child: Row(
                              children: [
                                const SizedBox(width: 16),
                                Icon(Icons.search, color: Colors.grey[600]),
                                const SizedBox(width: 12),
                                Text(
                                  'Buscar ofertas...',
                                  style: TextStyle(
                                    color: Colors.grey[600],
                                    fontSize: 16,
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
          
          // Balance Card
          SliverToBoxAdapter(
            child: Consumer(
              builder: (context, ref, _) {
                final balance = ref.watch(userBalanceProvider);
                return LumisBalanceCard(balance: balance);
              },
            ),
          ),
          
          // Categories Carousel
          SliverToBoxAdapter(
            child: SizedBox(
              height: 100,
              child: CategoriesCarousel(
                onCategorySelected: (category) {
                  ref.read(offersListProvider.notifier).updateFilters(
                    OfferFilters(category: category),
                  );
                },
              ),
            ),
          ),
          
          // Featured Offers
          SliverToBoxAdapter(
            child: FeaturedOffersSection(),
          ),
          
          // Offers Grid
          if (offersState.offers.isEmpty && !offersState.isLoading)
            SliverFillRemaining(
              child: EmptyStateWidget(
                icon: Icons.local_offer_outlined,
                title: 'No hay ofertas disponibles',
                subtitle: 'Vuelve pronto para ver nuevas ofertas',
                onAction: () {
                  ref.read(offersListProvider.notifier).loadOffers(refresh: true);
                },
                actionLabel: 'Actualizar',
              ),
            )
          else
            SliverPadding(
              padding: const EdgeInsets.all(16),
              sliver: SliverGrid(
                gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                  crossAxisCount: 2,
                  childAspectRatio: 0.75,
                  crossAxisSpacing: 16,
                  mainAxisSpacing: 16,
                ),
                delegate: SliverChildBuilderDelegate(
                  (context, index) {
                    if (index < offersState.offers.length) {
                      return OfferCard(
                        offer: offersState.offers[index],
                        onTap: () => _navigateToDetail(
                          context,
                          offersState.offers[index],
                        ),
                        onFavorite: () => ref
                            .read(offersListProvider.notifier)
                            .toggleFavorite(offersState.offers[index].id),
                      );
                    } else if (offersState.hasMore) {
                      return const ShimmerOfferCard();
                    }
                    return null;
                  },
                  childCount: offersState.offers.length + 
                    (offersState.isLoading ? 4 : 0),
                ),
              ),
            ),
          
          // Loading indicator at bottom
          if (offersState.isLoading && offersState.offers.isNotEmpty)
            const SliverToBoxAdapter(
              child: Padding(
                padding: EdgeInsets.all(16),
                child: Center(
                  child: CircularProgressIndicator(),
                ),
              ),
            ),
        ],
      ),
    );
  }
  
  void _navigateToDetail(BuildContext context, Offer offer) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => OfferDetailScreen(offerId: offer.id),
      ),
    );
  }
}
```

### Offer Detail Screen con Hero Animation
```dart
// lib/features/offers/presentation/screens/offer_detail_screen.dart

class OfferDetailScreen extends ConsumerWidget {
  final String offerId;
  
  const OfferDetailScreen({
    super.key,
    required this.offerId,
  });
  
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final offerAsync = ref.watch(selectedOfferProvider(offerId));
    
    return Scaffold(
      body: offerAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, stack) => ErrorWidget(error: error.toString()),
        data: (offer) => _buildContent(context, ref, offer),
      ),
    );
  }
  
  Widget _buildContent(BuildContext context, WidgetRef ref, Offer offer) {
    return CustomScrollView(
      slivers: [
        // Image Gallery with Hero
        SliverAppBar(
          expandedHeight: 300,
          pinned: true,
          flexibleSpace: FlexibleSpaceBar(
            background: Hero(
              tag: 'offer_${offer.id}',
              child: ImageGallery(
                images: offer.images,
                onImageTap: (index) {
                  Navigator.push(
                    context,
                    MaterialPageRoute(
                      builder: (_) => FullScreenGallery(
                        images: offer.images,
                        initialIndex: index,
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
        ),
        
        // Content
        SliverToBoxAdapter(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Title and Merchant
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            offer.title,
                            style: Theme.of(context).textTheme.headlineSmall,
                          ),
                          const SizedBox(height: 8),
                          Row(
                            children: [
                              CircleAvatar(
                                radius: 16,
                                backgroundImage: NetworkImage(
                                  offer.merchant.logoUrl,
                                ),
                              ),
                              const SizedBox(width: 8),
                              Text(
                                offer.merchant.name,
                                style: TextStyle(
                                  color: Colors.grey[600],
                                  fontSize: 16,
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                    // Favorite Button
                    IconButton(
                      icon: Icon(
                        offer.isFavorite 
                          ? Icons.favorite 
                          : Icons.favorite_border,
                        color: offer.isFavorite ? Colors.red : null,
                      ),
                      onPressed: () {
                        ref.read(offersListProvider.notifier)
                          .toggleFavorite(offer.id);
                      },
                    ),
                  ],
                ),
                
                const SizedBox(height: 24),
                
                // Price and Stock Info
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: Theme.of(context).primaryColor.withOpacity(0.1),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Column(
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                'Costo',
                                style: TextStyle(
                                  color: Colors.grey[600],
                                  fontSize: 14,
                                ),
                              ),
                              const SizedBox(height: 4),
                              AnimatedLumisCounter(
                                lumis: offer.lumisCost,
                                fontSize: 24,
                                color: Theme.of(context).primaryColor,
                              ),
                            ],
                          ),
                          if (offer.inventory != null)
                            Column(
                              crossAxisAlignment: CrossAxisAlignment.end,
                              children: [
                                Text(
                                  'Disponibles',
                                  style: TextStyle(
                                    color: Colors.grey[600],
                                    fontSize: 14,
                                  ),
                                ),
                                const SizedBox(height: 4),
                                Text(
                                  '${offer.inventory!.currentStock}',
                                  style: const TextStyle(
                                    fontSize: 24,
                                    fontWeight: FontWeight.bold,
                                  ),
                                ),
                              ],
                            ),
                        ],
                      ),
                      if (offer.expiresIn != null)
                        Padding(
                          padding: const EdgeInsets.only(top: 16),
                          child: CountdownTimer(
                            endTime: offer.endDate,
                            textStyle: TextStyle(
                              color: Colors.orange[700],
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
                
                const SizedBox(height: 24),
                
                // Description
                ExpandableText(
                  text: offer.description,
                  maxLines: 5,
                ),
                
                const SizedBox(height: 24),
                
                // Terms and Conditions
                ExpansionTile(
                  title: const Text('Términos y Condiciones'),
                  children: offer.terms.map((term) => ListTile(
                    leading: const Icon(Icons.check_circle_outline, size: 20),
                    title: Text(term),
                    dense: true,
                  )).toList(),
                ),
                
                // Reviews Section
                const SizedBox(height: 24),
                ReviewsSection(offerId: offer.id),
                
                // Similar Offers
                const SizedBox(height: 24),
                SimilarOffersSection(
                  category: offer.category,
                  excludeId: offer.id,
                ),
                
                const SizedBox(height: 100), // Space for bottom bar
              ],
            ),
          ),
        ),
      ],
    );
  }
}
```

### Redemption Flow Implementation
```dart
// lib/features/redemption/presentation/screens/redemption_flow.dart

class RedemptionFlow extends ConsumerStatefulWidget {
  final Offer offer;
  
  const RedemptionFlow({
    super.key,
    required this.offer,
  });
  
  @override
  ConsumerState<RedemptionFlow> createState() => _RedemptionFlowState();
}

class _RedemptionFlowState extends ConsumerState<RedemptionFlow> {
  final PageController _pageController = PageController();
  int _currentStep = 0;
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Canjear Oferta'),
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(4),
          child: LinearProgressIndicator(
            value: (_currentStep + 1) / 3,
            backgroundColor: Colors.grey[300],
          ),
        ),
      ),
      body: PageView(
        controller: _pageController,
        physics: const NeverScrollableScrollPhysics(),
        children: [
          // Step 1: Confirmation
          RedemptionConfirmationStep(
            offer: widget.offer,
            onConfirm: () => _nextStep(),
            onCancel: () => Navigator.pop(context),
          ),
          
          // Step 2: Processing
          RedemptionProcessingStep(
            onComplete: (redemption) => _nextStep(data: redemption),
          ),
          
          // Step 3: Success with QR
          RedemptionSuccessStep(
            redemption: _redemption!,
            onDone: () => Navigator.popUntil(
              context,
              ModalRoute.withName('/'),
            ),
          ),
        ],
      ),
    );
  }
  
  Redemption? _redemption;
  
  void _nextStep({Redemption? data}) {
    if (data != null) {
      _redemption = data;
    }
    
    setState(() {
      _currentStep++;
    });
    
    _pageController.animateToPage(
      _currentStep,
      duration: const Duration(milliseconds: 300),
      curve: Curves.easeInOut,
    );
  }
}

// QR Code Display with Animation
class QRCodeDisplay extends StatefulWidget {
  final String code;
  final DateTime expiresAt;
  
  const QRCodeDisplay({
    super.key,
    required this.code,
    required this.expiresAt,
  });
  
  @override
  State<QRCodeDisplay> createState() => _QRCodeDisplayState();
}

class _QRCodeDisplayState extends State<QRCodeDisplay>
    with SingleTickerProviderStateMixin {
  late AnimationController _animationController;
  late Animation<double> _scaleAnimation;
  
  @override
  void initState() {
    super.initState();
    _animationController = AnimationController(
      duration: const Duration(milliseconds: 500),
      vsync: this,
    );
    
    _scaleAnimation = Tween<double>(
      begin: 0.0,
      end: 1.0,
    ).animate(CurvedAnimation(
      parent: _animationController,
      curve: Curves.elasticOut,
    ));
    
    _animationController.forward();
    
    // Set maximum brightness
    SystemChrome.setSystemUIOverlayStyle(
      const SystemUiOverlayStyle(
        statusBarBrightness: Brightness.light,
      ),
    );
    
    // Keep screen on
    WakelockPlus.enable();
  }
  
  @override
  void dispose() {
    _animationController.dispose();
    WakelockPlus.disable();
    super.dispose();
  }
  
  @override
  Widget build(BuildContext context) {
    return Container(
      color: Colors.white,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          // Countdown Timer
          CountdownTimer(
            endTime: widget.expiresAt,
            onEnd: () {
              showDialog(
                context: context,
                barrierDismissible: false,
                builder: (_) => AlertDialog(
                  title: const Text('Código Expirado'),
                  content: const Text(
                    'El código ha expirado. Por favor, genera uno nuevo.',
                  ),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.popUntil(
                        context,
                        ModalRoute.withName('/'),
                      ),
                      child: const Text('Entendido'),
                    ),
                  ],
                ),
              );
            },
          ),
          
          const SizedBox(height: 32),
          
          // QR Code with animation
          ScaleTransition(
            scale: _scaleAnimation,
            child: Container(
              padding: const EdgeInsets.all(24),
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(20),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withOpacity(0.1),
                    blurRadius: 20,
                    offset: const Offset(0, 10),
                  ),
                ],
              ),
              child: QrImageView(
                data: widget.code,
                version: QrVersions.auto,
                size: 250,
                backgroundColor: Colors.white,
                errorCorrectionLevel: QrErrorCorrectLevel.H,
                embeddedImage: const AssetImage('assets/logo.png'),
                embeddedImageStyle: const QrEmbeddedImageStyle(
                  size: Size(40, 40),
                ),
              ),
            ),
          ),
          
          const SizedBox(height: 24),
          
          // Code Text
          Text(
            widget.code,
            style: const TextStyle(
              fontSize: 24,
              fontWeight: FontWeight.bold,
              letterSpacing: 2,
              fontFamily: 'monospace',
            ),
          ),
          
          const SizedBox(height: 16),
          
          // Copy button
          OutlinedButton.icon(
            icon: const Icon(Icons.copy),
            label: const Text('Copiar código'),
            onPressed: () {
              Clipboard.setData(ClipboardData(text: widget.code));
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(
                  content: Text('Código copiado'),
                  duration: Duration(seconds: 2),
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}
```

## 🔧 Custom Widgets Implementation

### Animated Lumis Balance Card
```dart
// lib/features/offers/presentation/widgets/lumis_balance_card.dart

class LumisBalanceCard extends StatelessWidget {
  final UserBalance balance;
  
  const LumisBalanceCard({
    super.key,
    required this.balance,
  });
  
  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(16),
      height: 120,
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            Colors.purple[600]!,
            Colors.purple[800]!,
          ],
        ),
        borderRadius: BorderRadius.circular(16),
        boxShadow: [
          BoxShadow(
            color: Colors.purple.withOpacity(0.3),
            blurRadius: 20,
            offset: const Offset(0, 10),
          ),
        ],
      ),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => Navigator.pushNamed(context, '/wallet'),
          borderRadius: BorderRadius.circular(16),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Text(
                        'Tu Balance',
                        style: TextStyle(
                          color: Colors.white.withOpacity(0.8),
                          fontSize: 14,
                        ),
                      ),
                      const SizedBox(height: 8),
                      AnimatedLumisCounter(
                        lumis: balance.available,
                        fontSize: 28,
                        color: Colors.white,
                      ),
                      const SizedBox(height: 4),
                      Text(
                        '≈ \$${(balance.available * 0.01).toStringAsFixed(2)} MXN',
                        style: TextStyle(
                          color: Colors.white.withOpacity(0.7),
                          fontSize: 12,
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 8,
                  ),
                  decoration: BoxDecoration(
                    color: Colors.white.withOpacity(0.2),
                    borderRadius: BorderRadius.circular(20),
                  ),
                  child: Row(
                    children: const [
                      Text(
                        'Canjear',
                        style: TextStyle(
                          color: Colors.white,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      SizedBox(width: 4),
                      Icon(
                        Icons.arrow_forward,
                        color: Colors.white,
                        size: 16,
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
```

## 🎯 State Management Patterns

### Complex Filter State
```dart
// lib/features/offers/presentation/providers/filter_provider.dart

@freezed
class OfferFilters with _$OfferFilters {
  const factory OfferFilters({
    @Default([]) List<String> categories,
    @Default([]) List<OfferType> types,
    @Default(0) int minLumis,
    @Default(10000) int maxLumis,
    @Default(10.0) double radiusKm,
    double? lat,
    double? lng,
    @Default(SortBy.relevance) SortBy sortBy,
    String? searchQuery,
  }) = _OfferFilters;
}

final filterProvider = StateNotifierProvider<FilterNotifier, OfferFilters>((ref) {
  return FilterNotifier();
});

class FilterNotifier extends StateNotifier<OfferFilters> {
  FilterNotifier() : super(const OfferFilters());
  
  void updateCategory(String category, bool selected) {
    if (selected) {
      state = state.copyWith(
        categories: [...state.categories, category],
      );
    } else {
      state = state.copyWith(
        categories: state.categories.where((c) => c != category).toList(),
      );
    }
  }
  
  void updatePriceRange(int min, int max) {
    state = state.copyWith(minLumis: min, maxLumis: max);
  }
  
  void updateLocation(double lat, double lng, double radius) {
    state = state.copyWith(lat: lat, lng: lng, radiusKm: radius);
  }
  
  void clearFilters() {
    state = const OfferFilters();
  }
  
  int get activeFilterCount {
    int count = 0;
    if (state.categories.isNotEmpty) count++;
    if (state.types.isNotEmpty) count++;
    if (state.minLumis > 0 || state.maxLumis < 10000) count++;
    if (state.lat != null) count++;
    if (state.searchQuery?.isNotEmpty ?? false) count++;
    return count;
  }
}
```

---

*Última actualización: Diciembre 2024*
