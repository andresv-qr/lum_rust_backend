#!/usr/bin/env python3
"""
Test push notification for user 1 using FCM HTTP v1 API
"""
import json
import time
import google.auth.transport.requests
from google.oauth2 import service_account

# Configuration
SERVICE_ACCOUNT_FILE = "firebase_account/lum-rewards-app-firebase-adminsdk-fbsvc-b27bb8d7e4.json"
PROJECT_ID = "lum-rewards-app"
FCM_TOKEN = "eLY7wTnLTj2AjKn8VaQBi2:APA91bFqxLWseePG8ankCNRBCZJohWgelpTO-ax2zpzKD1Q0gEU82f9oO7f6Rp_a0NN2UALobXKUsP2NnfmjIrRLa64_IBMGsZ0khq9NGVmJIZyppKdLP4o"

def get_access_token():
    """Get OAuth 2.0 access token for FCM"""
    credentials = service_account.Credentials.from_service_account_file(
        SERVICE_ACCOUNT_FILE,
        scopes=['https://www.googleapis.com/auth/firebase.messaging']
    )
    request = google.auth.transport.requests.Request()
    credentials.refresh(request)
    return credentials.token

def send_push_notification():
    """Send test push notification"""
    import requests
    
    print("🔐 Getting OAuth 2.0 access token...")
    access_token = get_access_token()
    print(f"✅ Token obtained (first 50 chars): {access_token[:50]}...")
    
    url = f"https://fcm.googleapis.com/v1/projects/{PROJECT_ID}/messages:send"
    
    headers = {
        "Authorization": f"Bearer {access_token}",
        "Content-Type": "application/json"
    }
    
    # Message payload
    message = {
        "message": {
            "token": FCM_TOKEN,
            "notification": {
                "title": "🎉 ¡Test Push Notification!",
                "body": f"Esta es una prueba enviada el {time.strftime('%Y-%m-%d %H:%M:%S')}"
            },
            "data": {
                "type": "test",
                "user_id": "1",
                "timestamp": str(int(time.time()))
            },
            "android": {
                "priority": "high",
                "notification": {
                    "channel_id": "lum_rewards_channel",
                    "sound": "default"
                }
            },
            "apns": {
                "payload": {
                    "aps": {
                        "sound": "default",
                        "badge": 1
                    }
                }
            }
        }
    }
    
    print(f"\n📤 Sending push notification to FCM...")
    print(f"   URL: {url}")
    print(f"   Token: {FCM_TOKEN[:50]}...")
    
    response = requests.post(url, headers=headers, json=message)
    
    print(f"\n📥 Response Status: {response.status_code}")
    print(f"📥 Response Body: {response.text}")
    
    if response.status_code == 200:
        print("\n✅ ¡Push notification enviado exitosamente!")
        print("   El usuario 1 debería recibir la notificación en su dispositivo.")
    else:
        print("\n❌ Error al enviar la notificación")
        try:
            error_data = response.json()
            if 'error' in error_data:
                error = error_data['error']
                print(f"   Error Code: {error.get('code')}")
                print(f"   Error Status: {error.get('status')}")
                print(f"   Error Message: {error.get('message')}")
                
                # Check for specific FCM errors
                for detail in error.get('details', []):
                    if '@type' in detail and 'errorCode' in detail:
                        fcm_error = detail.get('errorCode')
                        print(f"   FCM Error Code: {fcm_error}")
                        
                        if fcm_error == 'UNREGISTERED':
                            print("\n   ⚠️  El token ya no es válido.")
                            print("   El usuario necesita reinstalar la app o re-autenticarse.")
                        elif fcm_error == 'INVALID_ARGUMENT':
                            print("\n   ⚠️  El token tiene formato inválido.")
        except:
            pass
    
    return response.status_code == 200

if __name__ == "__main__":
    print("=" * 60)
    print("🔔 Test Push Notification - User ID: 1")
    print("=" * 60)
    
    success = send_push_notification()
    
    print("\n" + "=" * 60)
    print("✅ Test completado" if success else "❌ Test falló")
    print("=" * 60)
