# Especificaciones de Aplicación Móvil para Comercios: LumiMerchant

> **Proposito:** Documentación técnica y funcional para el desarrollo de la aplicación móvil (Flutter) que permitirá a los comercios validar y redimir Lümis de los usuarios mediante escaneo de códigos QR.
> **Plataforma:** iOS, Android y Web (Flutter)
> **Versión API:** v1 y v4

---

## 1. Visión General del Producto

**LumiMerchant** es una aplicación herramienta ligera diseñada para el personal de caja y servicio al cliente de los aliados comerciales. Su única función es validar que una redención de puntos (Lümis) es legítima y marcarla como "utilizada" en el sistema.

### Flujo Principal
1.  **Login:** El comercio inicia sesión con sus credenciales o API Key.
2.  **Escaneo:** El cajero presiona "Escanear" y apunta la cámara al QR del cliente.
3.  **Validación:** La app consulta al backend si el QR es válido.
4.  **Confirmación:** La app muestra los detalles (Cliente, Oferta, Estado) y el cajero presiona "Confirmar Canje".
5.  **Éxito:** Se muestra una pantalla verde de éxito.

---

## 2. Arquitectura Técnica (Flutter)

### Stack Recomendado
*   **Framework:** Flutter (Última versión estable)
*   **State Management:** `flutter_riverpod` (o `provider` para simplicidad extrema).
*   **Networking:** `dio` (Manejo robusto de HTTP, interceptors para headers).
*   **Almacenamiento Local:** `flutter_secure_storage` (para guardar token/API Key de forma segura).
*   **Cámara/QR:** `mobile_scanner` (Rendimiento nativo y fácil implementación).
*   **Navegación:** `go_router`.

### Estructura de Proyecto Sugerida
```
lib/
├── main.dart
├── config/
│   ├── theme.dart
│   └── routes.dart
├── core/
│   ├── api_client.dart       (Configuración de Dio)
│   ├── storage.dart          (Manejo de SecureStorage)
│   └── constants.dart
├── features/
│   ├── auth/
│   │   ├── screens/
│   │   │   └── login_screen.dart
│   │   ├── providers/
│   │   └── auth_service.dart
│   ├── scanner/
│   │   ├── screens/
│   │   │   ├── scanner_view.dart
│   │   │   └── validation_result_screen.dart
│   │   ├── providers/
│   │   └── redemption_service.dart
│   └── history/              (Opcional v2)
│       └── screens/
│           └── history_screen.dart
```

---

## 3. Especificaciones de Pantallas y Flujos

### A. Pantalla de Login
*   **UI:** Logo de Lumis, Campos de texto para `Merchant Code` y `API Key` (o Usuario/Contraseña según auth unificado).
*   **Lógica:**
    *   Almacenar credenciales de forma segura (`flutter_secure_storage`).
    *   Validar contra el backend (Endpoint de prueba o login específico).
    *   Si es exitoso, navegar a `Home`.

### B. Pantalla Principal (Home)
*   **UI:**
    *   Bienvenida ("Hola, Pizza Hut").
    *   Botón grande y central: **"Escanear Código QR"**.
    *   (Opcional) Resumen del día: "5 cupones canjeados hoy".
    *   Botón de "Cerrar Sesión" en la esquina.

### C. Pantalla de Escáner
*   **UI:** Vista de cámara a pantalla completa con un recuadro de guía.
*   **Lógica:**
    *   Usar `mobile_scanner`.
    *   Al detectar un QR, pausar la cámara.
    *   **Parsing:** El QR contiene una URL o un JSON. Extraer el `redemption_code` y el `token`.
    *   **Acción:** Llamar inmediatamente a `POST /merchant/validate`.
    *   **Loading:** Mostrar indicador de carga mientras valida.

### D. Pantalla de Resultado (Validación)
Esta pantalla es CRÍTICA. Debe mostrar claramente si es válido o no.

**Caso 1: Código Válido**
*   **Color Predominante:** Azul o Neutro.
*   **Información Mostrada:**
    *   ✅ "Código Válido"
    *   **Oferta:** "20% descuento en Pizza" (Grande y claro)
    *   **Cliente:** "Juan Pérez"
    *   **Vence:** "Hoy 10:00 PM"
*   **Acción:** Botón grande **"CONFIRMAR USO"**.

**Caso 2: Código Inválido/Usado/Expirado**
*   **Color Predominante:** Rojo.
*   **Información Mostrada:**
    *   🚫 "CÓDIGO INVÁLIDO" o "YA FUE USADO".
    *   Razón del error (viene del backend).
*   **Acción:** Botón "Volver a Escanear".

### E. Pantalla de Éxito
*   **Activación:** Tras presionar "CONFIRMAR USO" y recibir 200 OK del backend.
*   **UI:** Pantalla verde completa con un gran Check animado.
*   **Texto:** "¡Canje Exitoso!"
*   **Acción:** Redirección automática al Home en 3 segundos o botón "Nuevo Escaneo".

---

## 4. Integración con API Backend

Según `API_DOC_REDEMPTIONS.md`, estos son los endpoints requeridos.

### Configuración de HTTP Client (Dio)
Todos los requests deben incluir los headers de seguridad del comercio:
```dart
headers: {
  'Content-Type': 'application/json',
  'X-Merchant-Code': merchantCode, // Obtenido del storage
  'X-Api-Key': apiKey              // Obtenido del storage
}
```

### 1. Validar Código (Pre-Check)
Se llama cuando la cámara detecta el QR.

*   **Endpoint:** `POST /api/v1/merchant/validate`
*   **Body:**
    ```json
    {
      "redemption_code": "LUMS-A1B2C3",  // Extraído del QR
      "validation_token": "eyJ..."       // Extraído del QR (si aplica)
    }
    ```
*   **Manejo de Respuesta:**
    *   `200 OK` -> `data.valid == true` -> Ir a Pantalla Resultado (Caso Válido).
    *   `400 Bad Request` -> Mostrar Error específico (Ej: "Código expirado").

### 2. Confirmar Canje (Acción Final)
Se llama al presionar el botón "Confirmar Uso".

*   **Endpoint:** `POST /api/v1/merchant/confirm/{redemption_id}` (El ID viene de la respuesta de validación).
*   **Body:** `{}` (Vacío).
*   **Manejo de Respuesta:**
    *   `200 OK` -> Ir a Pantalla de Éxito.
    *   Error -> Mostrar snackbar "Error al confirmar, intente de nuevo".

---

## 5. Consideraciones de Seguridad
1.  **HTTPS:** Obligatorio para todas las comunicaciones.
2.  **No almacenar datos sensibles:** La app no debe guardar historial de clientes en el dispositivo local, solo tokens de sesión.
3.  **Timeout de Sesión:** Si la API Key es muy sensible, considerar obligar al login cada 24 horas.

## 6. Siguientes Pasos para Desarrollo
1.  Crear proyecto Flutter `flutter create lumi_merchant`.
2.  Configurar flavors (dev/prod) para apuntar a `localhost:8000` o `api.lumis.pa`.
3.  Implementar capa de Auth y Storage.
4.  Implementar UI de Escáner.
5.  Integrar endpoints.
