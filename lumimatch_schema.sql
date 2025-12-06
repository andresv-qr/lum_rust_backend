-- Crear el esquema para el módulo
CREATE SCHEMA IF NOT EXISTS lumimatch;

-- 1. Las preguntas (El "Qué")
CREATE TABLE IF NOT EXISTS lumimatch.questions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    title TEXT NOT NULL,
    image_url TEXT, -- Imagen principal de la pregunta
    
    -- Configuración de Vigencia
    valid_from TIMESTAMPTZ,
    valid_to TIMESTAMPTZ,
    specific_date DATE, -- Para preguntas que solo aplican un día específico (ej: Navidad)
    is_active BOOLEAN DEFAULT true,
    priority INTEGER DEFAULT 0, -- Para ordenar qué sale primero
    
    -- Segmentación Flexible (El "A Quién")
    -- Ver documentación completa de targeting_rules al final del archivo
    targeting_rules JSONB DEFAULT '{}'::jsonb, 
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2. Las opciones (El "Cómo responder")
CREATE TABLE IF NOT EXISTS lumimatch.options (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    question_id UUID REFERENCES lumimatch.questions(id) ON DELETE CASCADE,
    label TEXT,       -- Texto de la opción
    image_url TEXT,   -- Imagen opcional (ej: foto de producto)
    icon_url TEXT,    -- Icono opcional
    display_order INTEGER DEFAULT 0
);

-- 3. Las respuestas (El "Qué pasó")
CREATE TABLE IF NOT EXISTS lumimatch.user_answers (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id INTEGER NOT NULL, -- Asumiendo que tu user_id es INT
    question_id UUID REFERENCES lumimatch.questions(id),
    option_id UUID REFERENCES lumimatch.options(id),
    answered_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Restricción: Un usuario solo responde una vez a cada pregunta
    UNIQUE(user_id, question_id)
);

-- Índices para Performance
CREATE INDEX IF NOT EXISTS idx_lumimatch_questions_validity ON lumimatch.questions(is_active, valid_from, valid_to);
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_user ON lumimatch.user_answers(user_id);

-- 4. Etiquetas de Usuario (Para segmentación performante)
-- Esta tabla se llena asincrónicamente cuando el usuario realiza acciones (subir facturas, etc.)
-- Los tags se generan automáticamente basados en el comportamiento del usuario.
-- Ejemplos de tags:
--   - product_code:ABC123 (compró producto con código ABC123)
--   - product_l1:alimentos (compró en categoría nivel 1)
--   - product_l2:bebidas (compró en categoría nivel 2)
--   - product_brand:cocacola (compró marca específica)
--   - issuer_ruc:12345678 (compró en comercio con RUC)
--   - issuer_brand_name:mcdonalds (compró en marca de comercio)
--   - issuer_store_name:mcdonalds_via_espana (compró en tienda específica)
--   - issuer_l1:restaurantes (compró en tipo de comercio nivel 1)
--   - vip, early_adopter, churned (tags manuales/calculados)
CREATE TABLE IF NOT EXISTS lumimatch.user_tags (
    user_id INTEGER NOT NULL,
    tag TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, tag)
);

-- Índices para Performance de Questions
CREATE INDEX IF NOT EXISTS idx_lumimatch_questions_validity ON lumimatch.questions(is_active, valid_from, valid_to);
CREATE INDEX IF NOT EXISTS idx_lumimatch_questions_specific_date ON lumimatch.questions(specific_date) WHERE specific_date IS NOT NULL;

-- Índices para Performance de Answers
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_user ON lumimatch.user_answers(user_id);

-- Índices para Analítica
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_question ON lumimatch.user_answers(question_id);
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_option ON lumimatch.user_answers(option_id);
CREATE INDEX IF NOT EXISTS idx_lumimatch_answers_date ON lumimatch.user_answers(answered_at);

-- Índice para búsqueda de tags por usuario
CREATE INDEX IF NOT EXISTS idx_lumimatch_user_tags_user ON lumimatch.user_tags(user_id);

-- ============================================================================
-- DOCUMENTACIÓN: targeting_rules (JSONB)
-- ============================================================================
-- 
-- El campo targeting_rules permite segmentar preguntas de forma flexible.
-- Todas las condiciones son opcionales y se evalúan con lógica AND entre grupos.
--
-- ESTRUCTURA COMPLETA:
-- {
--   // --- Demografía ---
--   "min_age": 18,                    // Edad mínima del usuario
--   "max_age": 65,                    // Edad máxima del usuario
--   "countries": ["PA", "CO"],        // Países permitidos (ISO 3166-1 alpha-2)
--
--   // --- Usuarios Específicos ---
--   "user_ids": [1, 2, 3],            // Solo estos user_ids verán la pregunta
--
--   // --- Tags (Sistema Flexible) ---
--   "required_tags": ["vip"],         // Usuario DEBE tener TODOS estos tags (AND)
--   "any_tags": ["coffee", "tea"],    // Usuario DEBE tener AL MENOS UNO (OR)
--   "excluded_tags": ["churned"],     // Usuario NO DEBE tener NINGUNO (NOT)
--
--   // --- Productos (via tags automáticos) ---
--   // Estos se mapean a tags: "product_code:ABC123", "product_l1:alimentos", etc.
--   "product_codes": ["ABC123", "XYZ789"],
--   "product_l1": ["alimentos", "bebidas"],
--   "product_l2": ["lacteos", "gaseosas"],
--   "product_l3": ["leche_entera"],
--   "product_l4": ["leche_entera_1L"],
--   "product_brands": ["cocacola", "pepsi"],
--
--   // --- Comercios/Emisores (via tags automáticos) ---
--   // Estos se mapean a tags: "issuer_ruc:12345", "issuer_l1:restaurantes", etc.
--   "issuer_rucs": ["12345678-1-2024"],
--   "issuer_brand_names": ["mcdonalds", "wendys"],
--   "issuer_store_names": ["mcdonalds_via_espana"],
--   "issuer_l1": ["restaurantes", "supermercados"],
--   "issuer_l2": ["comida_rapida"],
--   "issuer_l3": ["hamburguesas"],
--   "issuer_l4": ["hamburguesas_premium"]
-- }
--
-- NOTA: Los campos de producto/comercio funcionan con el sistema de tags.
-- Cuando un usuario sube una factura, el sistema debe generar tags como:
--   - product_code:ABC123
--   - product_l1:alimentos
--   - issuer_ruc:12345678
--   - issuer_brand_name:mcdonalds
-- El motor de preguntas luego verifica si el usuario tiene estos tags.
-- ============================================================================
