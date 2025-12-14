# Diagnóstico de Error de Autenticación GCP

## Problema Reportado
La aplicación reporta errores de "Invalid JWT Signature" al intentar autenticarse con Google Cloud Platform (Firebase Cloud Messaging).

```
gcp_auth::types: token request failed body={"error":"invalid_grant","error_description":"Invalid JWT Signature."}
```

## Análisis Realizado

1.  **Verificación de Hora del Sistema**:
    *   La hora del servidor es correcta (`Sat Dec 13 21:05:55 UTC 2025`). La desincronización de reloj es una causa común, pero descartada aquí.

2.  **Verificación del Archivo de Credenciales**:
    *   Archivo: `/home/client_1099_1/scripts/lum_rust_ws/firebase_account/lum-rewards-app-firebase-adminsdk-fbsvc-94ed659b15.json`
    *   El archivo existe y es un JSON válido.
    *   El `project_id` coincide con la configuración (`lum-rewards-app`).
    *   El `client_email` parece correcto (`firebase-adminsdk-fbsvc@lum-rewards-app.iam.gserviceaccount.com`).

3.  **Validación de la Clave Privada**:
    *   Se extrajo la clave privada y se verificó con `openssl rsa -check`.
    *   Resultado: `RSA key ok`. La clave es criptográficamente válida.

4.  **Prueba de Autenticación Manual**:
    *   Se creó un script de depuración (`debug_jwt_sign.rs`) para firmar manualmente un JWT y enviarlo al endpoint de OAuth2 de Google.
    *   Resultado: `400 Bad Request - Invalid JWT Signature`.

## Conclusión
La clave privada contenida en el archivo JSON **no corresponde** a la cuenta de servicio en los servidores de Google. Esto suele ocurrir si:
*   La clave fue revocada o eliminada en la consola de Google Cloud/Firebase.
*   La cuenta de servicio fue recreada, invalidando las claves anteriores.

## Solución Requerida
Es necesario generar una nueva clave de cuenta de servicio.

### Pasos para solucionar:

1.  Ir a la **Consola de Firebase** > Configuración del Proyecto > Cuentas de servicio.
2.  Seleccionar la cuenta de servicio `firebase-adminsdk-fbsvc`.
3.  Hacer clic en **"Generar nueva clave privada"**.
4.  Descargar el archivo JSON.
5.  Subir el archivo al servidor en la ruta:
    `/home/client_1099_1/scripts/lum_rust_ws/firebase_account/`
6.  Actualizar el archivo `.env` con el nombre del nuevo archivo (si es diferente) o renombrar el nuevo archivo para que coincida con el existente.
    *   Variable: `GOOGLE_APPLICATION_CREDENTIALS`

Una vez reemplazado el archivo, reiniciar el servicio.
