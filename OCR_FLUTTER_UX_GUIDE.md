# Guía de Diseño UX/UI - OCR Multi-Imagen Flutter

## Resumen Ejecutivo

Sistema de carga iterativa de facturas sin QR que permite hasta 5 intentos, con feedback específico y preview tipo WhatsApp antes del guardado.

## Flujo de Usuario Principal

### 1. Pantalla de Entrada
**Ubicación**: `screens/invoice_upload_screen.dart`

```dart
// Elementos UI requeridos
- AppBar(title: "Subir Factura")
- Card con instrucciones claras
- Botón "Tomar Foto" (cámara)
- Botón "Seleccionar de Galería"
- Texto: "Máximo 5 imágenes por factura"
- Indicador de progreso: "0/5 intentos"
```

**Diseño Visual**:
- 📷 Ícono grande de cámara centrado
- 🔢 Contador visible: "Intento 1 de 5"
- 📋 Lista de campos requeridos:
  - ✅ Nombre del comercio
  - ✅ Número de factura  
  - ✅ Fecha
  - ✅ Total
  - ✅ Productos

### 2. Procesamiento y Resultados
**Ubicación**: `screens/ocr_processing_screen.dart`

#### 2.1 Estados de Pantalla

```dart
enum OcrScreenState {
  uploading,      // Subiendo imagen
  processing,     // Procesando OCR
  showingResults, // Mostrando resultados
  needsRetry,     // Requiere más imágenes
  previewReady,   // Listo para confirmar
  manualReview,   // Revisión manual
  success,        // Guardado exitoso
}
```

#### 2.2 Componente de Progreso
```dart
// Widget: ProgressIndicator
- LinearProgressIndicator con pasos
- Texto: "Procesando imagen 2 de 5..."
- Spinner durante procesamiento
- Timer estimado: "~30 segundos restantes"
```

#### 2.3 Resultados Parciales
```dart
// Widget: FieldStatusCard
Card(
  child: Column([
    Text("Datos detectados:"),
    FieldStatus("Comercio", detected: true, value: "Supermercado La Pradera"),
    FieldStatus("Número", detected: false, value: null),
    FieldStatus("Fecha", detected: true, value: "2025-09-05"),
    FieldStatus("Total", detected: false, value: null),
    FieldStatus("Productos", detected: false, value: null),
  ])
)
```

### 3. Pantalla de Reintento
**Ubicación**: `screens/retry_capture_screen.dart`

#### 3.1 Mensaje Específico
```dart
// Widget: MissingFieldsPrompt
Container(
  decoration: BoxDecoration(color: Colors.orange.shade50),
  child: Column([
    Icon(Icons.camera_enhance, size: 48, color: Colors.orange),
    Text("¡Faltan algunos datos!", style: headlineSmall),
    Text("Enfoca estas áreas en tu próxima foto:"),
    ...missingFields.map((field) => 
      ListTile(
        leading: Icon(Icons.arrow_right),
        title: Text(getFieldDisplayName(field)),
        subtitle: Text(getFieldHint(field)),
      )
    ),
    Text("Intento ${attemptCount} de 5", style: caption),
  ])
)
```

#### 3.2 Guías Visuales
```dart
// Overlay en cámara con hints específicos
CameraOverlay(
  hints: [
    if (missingFields.contains("total")) 
      HintBox(position: Alignment.bottomCenter, text: "Enfoca el total"),
    if (missingFields.contains("invoice_number"))
      HintBox(position: Alignment.topCenter, text: "Enfoca el número"),
  ]
)
```

### 4. Preview Final (tipo WhatsApp)
**Ubicación**: `screens/invoice_preview_screen.dart`

#### 4.1 Diseño Principal
```dart
// Similar a WhatsApp preview
Scaffold(
  backgroundColor: Colors.black,
  appBar: AppBar(
    title: Text("Confirmar Factura"),
    backgroundColor: Colors.black,
    actions: [
      IconButton(icon: Icons.edit, onPressed: showEditDialog),
    ]
  ),
  body: Column([
    // Imagen consolidada (scrollable/zoomable)
    Expanded(
      flex: 3,
      child: InteractiveViewer(
        child: Image.memory(consolidatedImageBytes),
      )
    ),
    
    // Datos detectados
    Expanded(
      flex: 2,
      child: InvoiceDataCard(invoiceData: detectedData)
    ),
    
    // Botones de acción
    SafeArea(
      child: Row([
        Expanded(
          child: ElevatedButton.icon(
            icon: Icon(Icons.close),
            label: Text("Necesito más fotos"),
            onPressed: requestMoreImages,
            style: ElevatedButton.styleFrom(backgroundColor: Colors.grey),
          )
        ),
        SizedBox(width: 16),
        Expanded(
          child: ElevatedButton.icon(
            icon: Icon(Icons.check),
            label: Text("Confirmar y Guardar"),
            onPressed: confirmAndSave,
            style: ElevatedButton.styleFrom(backgroundColor: Colors.green),
          )
        ),
      ])
    )
  ])
)
```

#### 4.2 Card de Datos
```dart
// Widget: InvoiceDataCard
Card(
  child: Padding(
    padding: EdgeInsets.all(16),
    child: Column([
      Text("Datos de la Factura", style: headlineSmall),
      Divider(),
      DataRow("🏪 Comercio:", invoiceData.issuerName),
      DataRow("📄 Número:", invoiceData.invoiceNumber),
      DataRow("📅 Fecha:", invoiceData.date),
      DataRow("💰 Total:", "\$${invoiceData.total}"),
      DataRow("📦 Productos:", "${invoiceData.products.length} artículos"),
      
      if (invoiceData.products.isNotEmpty) ...[
        Divider(),
        Text("Productos:", style: titleMedium),
        ...invoiceData.products.take(3).map((p) => 
          ProductTile(product: p)
        ),
        if (invoiceData.products.length > 3)
          Text("y ${invoiceData.products.length - 3} más...")
      ]
    ])
  )
)
```

### 5. Pantalla de Límite Alcanzado
**Ubicación**: `screens/manual_review_screen.dart`

```dart
Scaffold(
  appBar: AppBar(title: Text("Revisión Manual")),
  body: Padding(
    padding: EdgeInsets.all(16),
    child: Column([
      Icon(Icons.support_agent, size: 64, color: Colors.blue),
      SizedBox(height: 16),
      Text(
        "Límite de intentos alcanzado",
        style: headlineMedium,
        textAlign: TextAlign.center,
      ),
      SizedBox(height: 8),
      Text(
        "Has usado los 5 intentos disponibles. Tu factura será revisada por nuestro equipo.",
        style: bodyLarge,
        textAlign: TextAlign.center,
      ),
      
      // Datos parciales detectados
      Expanded(child: PartialDataCard(partialData: detectedData)),
      
      // Opciones
      Column([
        ElevatedButton.icon(
          icon: Icon(Icons.send),
          label: Text("Enviar para Revisión Manual"),
          onPressed: sendForManualReview,
          style: ElevatedButton.styleFrom(
            backgroundColor: Colors.orange,
            minimumSize: Size(double.infinity, 48),
          ),
        ),
        SizedBox(height: 8),
        OutlinedButton.icon(
          icon: Icon(Icons.cancel),
          label: Text("Cancelar y Volver"),
          onPressed: () => Navigator.pop(context),
          style: OutlinedButton.styleFrom(
            minimumSize: Size(double.infinity, 48),
          ),
        ),
      ])
    ])
  )
)
```

### 6. Pantalla de Éxito
**Ubicación**: `screens/success_screen.dart`

```dart
// Animación de éxito tipo WhatsApp
Column([
  Lottie.asset('assets/success_checkmark.json'),
  Text("¡Factura Guardada!", style: headlineLarge),
  Text("CUFE: ${response.cufe}", style: bodySmall.copyWith(fontFamily: 'monospace')),
  
  InfoCard(
    icon: Icons.schedule,
    title: "¿Qué sigue?",
    content: "Tu factura será validada en 24-48 horas. Te notificaremos por WhatsApp cuando esté lista.",
  ),
  
  Row([
    Expanded(
      child: OutlinedButton(
        child: Text("Ver Mis Facturas"),
        onPressed: () => Navigator.pushReplacementNamed(context, '/invoices'),
      )
    ),
    SizedBox(width: 16),
    Expanded(
      child: ElevatedButton(
        child: Text("Subir Otra"),
        onPressed: () => Navigator.pushReplacementNamed(context, '/upload'),
      )
    ),
  ])
])
```

## Componentes Reutilizables

### 1. FieldStatus Widget
```dart
class FieldStatus extends StatelessWidget {
  final String fieldName;
  final bool detected;
  final String? value;
  
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(
        detected ? Icons.check_circle : Icons.radio_button_unchecked,
        color: detected ? Colors.green : Colors.grey,
      ),
      title: Text(fieldName),
      subtitle: detected ? Text(value ?? '') : Text('No detectado'),
      trailing: detected ? null : Icon(Icons.camera_alt, color: Colors.orange),
    );
  }
}
```

### 2. ProgressDots Widget
```dart
class ProgressDots extends StatelessWidget {
  final int current;
  final int total;
  
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: List.generate(total, (index) => 
        Container(
          margin: EdgeInsets.symmetric(horizontal: 4),
          width: 12,
          height: 12,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: index < current ? Colors.blue : Colors.grey.shade300,
          ),
        )
      ),
    );
  }
}
```

### 3. CameraButton Widget
```dart
class CameraButton extends StatelessWidget {
  final VoidCallback onTap;
  final String label;
  final IconData icon;
  
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Theme.of(context).primaryColor,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column([
          Icon(icon, size: 32, color: Colors.white),
          SizedBox(height: 8),
          Text(label, style: TextStyle(color: Colors.white)),
        ]),
      ),
    );
  }
}
```

## Estados de la Aplicación

### 1. State Management (Provider/Riverpod)
```dart
class OcrInvoiceNotifier extends StateNotifier<OcrInvoiceState> {
  OcrInvoiceNotifier() : super(OcrInvoiceState.initial());
  
  Future<void> uploadImage(File image) async {
    state = state.copyWith(status: OcrStatus.uploading);
    
    final response = await _apiService.processOcrImage(
      image: image,
      sessionId: state.sessionId,
      action: state.attemptCount == 0 ? 'initial' : 'retry',
      missingFields: state.missingFields,
    );
    
    state = state.copyWith(
      status: _mapResponseStatus(response.status),
      attemptCount: response.attemptCount,
      detectedFields: response.detectedFields,
      missingFields: response.missingFields,
      sessionId: response.sessionId,
    );
  }
  
  Future<void> confirmAndSave() async {
    state = state.copyWith(status: OcrStatus.saving);
    
    final saveResponse = await _apiService.saveOcrInvoice(
      sessionId: state.sessionId!,
      invoiceData: state.detectedFields,
      consolidatedImage: state.consolidatedImage!,
    );
    
    if (saveResponse.success) {
      state = state.copyWith(
        status: OcrStatus.success,
        cufe: saveResponse.cufe,
      );
    }
  }
}

class OcrInvoiceState {
  final OcrStatus status;
  final int attemptCount;
  final String? sessionId;
  final Map<String, dynamic> detectedFields;
  final List<String> missingFields;
  final String? consolidatedImage;
  final String? cufe;
  
  // ... constructors and copyWith
}

enum OcrStatus {
  initial, uploading, processing, needsRetry, 
  previewReady, saving, success, manualReview, error
}
```

## Navegación y Rutas

### 1. App Router
```dart
// routes.dart
final appRouter = GoRouter(
  routes: [
    GoRoute(
      path: '/upload-invoice',
      builder: (context, state) => InvoiceUploadScreen(),
    ),
    GoRoute(
      path: '/ocr-processing',
      builder: (context, state) => OcrProcessingScreen(),
    ),
    GoRoute(
      path: '/invoice-preview',
      builder: (context, state) => InvoicePreviewScreen(),
    ),
    GoRoute(
      path: '/manual-review',
      builder: (context, state) => ManualReviewScreen(),
    ),
    GoRoute(
      path: '/upload-success',
      builder: (context, state) => SuccessScreen(),
    ),
  ],
);
```

### 2. Navigation Flow
```dart
// Flujo de navegación
uploadImage() {
  Navigator.pushNamed(context, '/ocr-processing');
}

processComplete() {
  if (needsMoreImages) {
    // Permanecer en processing screen, mostrar retry UI
    setState(() => showRetryMode = true);
  } else {
    Navigator.pushReplacementNamed(context, '/invoice-preview');
  }
}

maxAttemptsReached() {
  Navigator.pushReplacementNamed(context, '/manual-review');
}

saveSuccess() {
  Navigator.pushReplacementNamed(context, '/upload-success');
}
```

## Mensajes y Textos

### 1. Mensajes de Error
```dart
class AppMessages {
  static const maxAttemptsReached = "Has alcanzado el límite de 5 intentos. Tu factura será revisada manualmente.";
  static const fieldsMissing = "Faltan algunos campos. Toma una foto enfocando las áreas marcadas.";
  static const savingError = "Error guardando la factura. Intenta nuevamente.";
  static const networkError = "Sin conexión. Verifica tu internet e intenta de nuevo.";
}
```

### 2. Field Display Names
```dart
class FieldDisplayNames {
  static const Map<String, String> names = {
    'issuer_name': 'Nombre del Comercio',
    'invoice_number': 'Número de Factura',
    'date': 'Fecha',
    'total': 'Total',
    'products': 'Productos',
  };
  
  static const Map<String, String> hints = {
    'issuer_name': 'Busca el nombre del negocio en la parte superior',
    'invoice_number': 'Número único de la factura (ej: F001-123456)',
    'date': 'Fecha de emisión de la factura',
    'total': 'Monto total a pagar',
    'products': 'Lista de artículos comprados',
  };
}
```

## Configuración y Constantes

### 1. App Constants
```dart
class OcrConstants {
  static const int maxAttempts = 5;
  static const int sessionTimeoutMinutes = 30;
  static const double maxImageSizeMB = 10.0;
  static const List<String> supportedFormats = ['jpg', 'jpeg', 'png'];
  
  static const List<String> requiredFields = [
    'issuer_name', 'invoice_number', 'date', 'total', 'products'
  ];
}
```

### 2. Theme Configuration
```dart
class AppTheme {
  static final primaryColor = Color(0xFF2196F3);
  static final successColor = Color(0xFF4CAF50);
  static final warningColor = Color(0xFFFF9800);
  static final errorColor = Color(0xFFF44336);
  
  static final cardElevation = 4.0;
  static final borderRadius = BorderRadius.circular(12);
}
```

## Testing y Validación

### 1. Casos de Prueba UI
- ✅ Flujo completo 1 intento exitoso
- ✅ Flujo con 3 reintentos antes de éxito
- ✅ Flujo que llega a límite de 5 intentos
- ✅ Manejo de errores de red
- ✅ Navegación hacia atrás/cancelación
- ✅ Estado de la app al minimizar/reabrir

### 2. Validaciones de UX
- ✅ Botones claramente etiquetados
- ✅ Feedback visual inmediato
- ✅ Progreso siempre visible
- ✅ Accesibilidad (screen readers)
- ✅ Responsive design (tablets)

## Performance y Optimización

### 1. Optimización de Imágenes
```dart
// Comprimir antes de enviar
Future<File> compressImage(File image) async {
  final result = await FlutterImageCompress.compressAndGetFile(
    image.absolute.path,
    "${image.path}_compressed.jpg",
    quality: 85,
    minWidth: 1024,
    minHeight: 1024,
  );
  return result!;
}
```

### 2. Cache Management
```dart
// Cache de sesiones locales
class OcrSessionCache {
  static const String _key = 'ocr_sessions';
  
  static Future<void> saveSession(OcrSession session) async {
    final prefs = await SharedPreferences.getInstance();
    final sessions = await getSessions();
    sessions[session.id] = session;
    await prefs.setString(_key, jsonEncode(sessions));
  }
  
  static Future<Map<String, OcrSession>> getSessions() async {
    // Implementación del cache local
  }
}
```

Esta documentación proporciona una guía completa para el equipo de Flutter para implementar la experiencia de usuario del sistema OCR multi-imagen con todos los estados, componentes y flujos necesarios.
