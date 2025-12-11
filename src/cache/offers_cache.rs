// ============================================================================
// OFFERS CACHE SERVICE - Cache Redis para ofertas
// ============================================================================

use deadpool_redis::Pool as RedisPool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, warn};
use uuid::Uuid;

/// Configuración del cache de ofertas
#[derive(Clone)]
pub struct OffersCacheConfig {
    /// TTL en segundos para la lista de ofertas (5 minutos)
    pub list_ttl_seconds: u64,
    /// TTL en segundos para detalle de oferta (2 minutos)
    pub detail_ttl_seconds: u64,
    /// TTL en segundos para balance de usuario (30 segundos)
    pub balance_ttl_seconds: u64,
    /// Prefijo para las keys
    pub key_prefix: String,
}

impl Default for OffersCacheConfig {
    fn default() -> Self {
        Self {
            list_ttl_seconds: 300,     // 5 minutos
            detail_ttl_seconds: 120,   // 2 minutos
            balance_ttl_seconds: 30,   // 30 segundos
            key_prefix: "lum:offers:".to_string(),
        }
    }
}

/// Servicio de cache para ofertas de redención
pub struct OffersCacheService {
    pool: RedisPool,
    config: OffersCacheConfig,
}

impl OffersCacheService {
    pub fn new(pool: RedisPool, config: OffersCacheConfig) -> Self {
        Self { pool, config }
    }
    
    /// Genera la key para la lista de ofertas
    fn list_key(&self, user_id: i32, filters_hash: &str) -> String {
        format!("{}list:{}:{}", self.config.key_prefix, user_id, filters_hash)
    }
    
    /// Genera la key para el detalle de una oferta
    fn detail_key(&self, offer_id: &Uuid) -> String {
        format!("{}detail:{}", self.config.key_prefix, offer_id)
    }
    
    /// Genera la key para el balance de un usuario
    fn balance_key(&self, user_id: i32) -> String {
        format!("{}balance:{}", self.config.key_prefix, user_id)
    }
    
    /// Hash de filtros para la key
    pub fn hash_filters(&self, category: Option<&str>, min_cost: Option<i32>, max_cost: Option<i32>) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}:{:?}:{:?}", category, min_cost, max_cost).as_bytes());
        format!("{:x}", hasher.finalize())[..8].to_string()
    }
    
    /// Obtener lista de ofertas del cache
    pub async fn get_offers_list<T: for<'de> Deserialize<'de>>(
        &self,
        user_id: i32,
        filters_hash: &str,
    ) -> Option<T> {
        let key = self.list_key(user_id, filters_hash);
        
        match self.pool.get().await {
            Ok(mut conn) => {
                match conn.get::<_, Option<String>>(&key).await {
                    Ok(Some(data)) => {
                        match serde_json::from_str(&data) {
                            Ok(offers) => {
                                debug!("Cache HIT for offers list: {}", key);
                                Some(offers)
                            }
                            Err(e) => {
                                warn!("Failed to deserialize cached offers: {}", e);
                                None
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("Cache MISS for offers list: {}", key);
                        None
                    }
                    Err(e) => {
                        error!("Redis error getting offers list: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                None
            }
        }
    }
    
    /// Guardar lista de ofertas en cache
    pub async fn set_offers_list<T: Serialize>(
        &self,
        user_id: i32,
        filters_hash: &str,
        offers: &T,
    ) -> bool {
        let key = self.list_key(user_id, filters_hash);
        
        match serde_json::to_string(offers) {
            Ok(data) => {
                match self.pool.get().await {
                    Ok(mut conn) => {
                        match conn.set_ex::<_, _, ()>(
                            &key,
                            &data,
                            self.config.list_ttl_seconds
                        ).await {
                            Ok(_) => {
                                debug!("Cached offers list: {}", key);
                                true
                            }
                            Err(e) => {
                                error!("Failed to cache offers list: {}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get Redis connection: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Failed to serialize offers: {}", e);
                false
            }
        }
    }
    
    /// Obtener detalle de oferta del cache
    pub async fn get_offer_detail<T: for<'de> Deserialize<'de>>(
        &self,
        offer_id: &Uuid,
    ) -> Option<T> {
        let key = self.detail_key(offer_id);
        
        match self.pool.get().await {
            Ok(mut conn) => {
                match conn.get::<_, Option<String>>(&key).await {
                    Ok(Some(data)) => {
                        match serde_json::from_str(&data) {
                            Ok(offer) => {
                                debug!("Cache HIT for offer detail: {}", key);
                                Some(offer)
                            }
                            Err(e) => {
                                warn!("Failed to deserialize cached offer: {}", e);
                                None
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("Cache MISS for offer detail: {}", key);
                        None
                    }
                    Err(e) => {
                        error!("Redis error getting offer detail: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                None
            }
        }
    }
    
    /// Guardar detalle de oferta en cache
    pub async fn set_offer_detail<T: Serialize>(
        &self,
        offer_id: &Uuid,
        offer: &T,
    ) -> bool {
        let key = self.detail_key(offer_id);
        
        match serde_json::to_string(offer) {
            Ok(data) => {
                match self.pool.get().await {
                    Ok(mut conn) => {
                        match conn.set_ex::<_, _, ()>(
                            &key,
                            &data,
                            self.config.detail_ttl_seconds
                        ).await {
                            Ok(_) => {
                                debug!("Cached offer detail: {}", key);
                                true
                            }
                            Err(e) => {
                                error!("Failed to cache offer detail: {}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get Redis connection: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Failed to serialize offer: {}", e);
                false
            }
        }
    }
    
    /// Obtener balance de usuario del cache
    pub async fn get_user_balance(&self, user_id: i32) -> Option<i64> {
        let key = self.balance_key(user_id);
        
        match self.pool.get().await {
            Ok(mut conn) => {
                match conn.get::<_, Option<i64>>(&key).await {
                    Ok(balance) => {
                        if balance.is_some() {
                            debug!("Cache HIT for user balance: {}", key);
                        }
                        balance
                    }
                    Err(e) => {
                        error!("Redis error getting user balance: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                None
            }
        }
    }
    
    /// Guardar balance de usuario en cache
    pub async fn set_user_balance(&self, user_id: i32, balance: i64) -> bool {
        let key = self.balance_key(user_id);
        
        match self.pool.get().await {
            Ok(mut conn) => {
                match conn.set_ex::<_, _, ()>(
                    &key,
                    balance,
                    self.config.balance_ttl_seconds
                ).await {
                    Ok(_) => {
                        debug!("Cached user balance: {} = {}", key, balance);
                        true
                    }
                    Err(e) => {
                        error!("Failed to cache user balance: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                false
            }
        }
    }
    
    /// Invalidar balance de usuario (al crear redención)
    pub async fn invalidate_user_balance(&self, user_id: i32) -> bool {
        let key = self.balance_key(user_id);
        
        match self.pool.get().await {
            Ok(mut conn) => {
                match conn.del::<_, ()>(&key).await {
                    Ok(_) => {
                        debug!("Invalidated user balance cache: {}", key);
                        true
                    }
                    Err(e) => {
                        error!("Failed to invalidate user balance: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                false
            }
        }
    }
    
    /// Invalidar cache de oferta (al modificarla)
    pub async fn invalidate_offer(&self, offer_id: &Uuid) -> bool {
        let key = self.detail_key(offer_id);
        
        match self.pool.get().await {
            Ok(mut conn) => {
                match conn.del::<_, ()>(&key).await {
                    Ok(_) => {
                        debug!("Invalidated offer cache: {}", key);
                        true
                    }
                    Err(e) => {
                        error!("Failed to invalidate offer: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                false
            }
        }
    }
    
    /// Invalidar todas las listas de ofertas (al crear/modificar ofertas)
    pub async fn invalidate_all_lists(&self) -> bool {
        let pattern = format!("{}list:*", self.config.key_prefix);
        
        match self.pool.get().await {
            Ok(mut conn) => {
                // Scan y delete keys matching pattern
                let keys: Vec<String> = match redis::cmd("KEYS")
                    .arg(&pattern)
                    .query_async(&mut conn)
                    .await
                {
                    Ok(keys) => keys,
                    Err(e) => {
                        error!("Failed to scan keys: {}", e);
                        return false;
                    }
                };
                
                if keys.is_empty() {
                    return true;
                }
                
                for key in keys {
                    let _ = conn.del::<_, ()>(&key).await;
                }
                
                debug!("Invalidated all offers lists");
                true
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                false
            }
        }
    }
}

/// Wrapper para usar el servicio de cache globalmente
pub struct OffersCacheWrapper(pub Arc<OffersCacheService>);

impl Clone for OffersCacheWrapper {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl OffersCacheWrapper {
    pub fn new(pool: RedisPool) -> Self {
        Self(Arc::new(OffersCacheService::new(pool, OffersCacheConfig::default())))
    }
    
    pub fn with_config(pool: RedisPool, config: OffersCacheConfig) -> Self {
        Self(Arc::new(OffersCacheService::new(pool, config)))
    }
}
