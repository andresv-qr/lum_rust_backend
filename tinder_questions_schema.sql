-- 1. Las preguntas (El "Qué")
CREATE TABLE IF NOT EXISTS tinder_questions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    image_url TEXT, -- Imagen principal de la pregunta
    
    -- Configuración de Vigencia
    valid_from TIMESTAMPTZ,
    valid_to TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    priority INTEGER DEFAULT 0, -- Para ordenar qué sale primero
    
    -- Segmentación Flexible (El "A Quién")
    -- Ej: {"min_age": 18, "tags": ["coffee_lover"], "merchant_id": 123}
    targeting_rules JSONB DEFAULT '{}'::jsonb, 
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2. Las opciones (El "Cómo responder")
CREATE TABLE IF NOT EXISTS tinder_options (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    question_id UUID REFERENCES tinder_questions(id) ON DELETE CASCADE,
    label TEXT,       -- Texto de la opción
    image_url TEXT,   -- Imagen opcional (ej: foto de producto)
    icon_url TEXT,    -- Icono opcional
    display_order INTEGER DEFAULT 0
);

-- 3. Las respuestas (El "Qué pasó")
CREATE TABLE IF NOT EXISTS tinder_user_answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INTEGER NOT NULL, -- Asumiendo que tu user_id es INT
    question_id UUID REFERENCES tinder_questions(id),
    option_id UUID REFERENCES tinder_options(id),
    answered_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Restricción: Un usuario solo responde una vez a cada pregunta
    UNIQUE(user_id, question_id)
);

-- Índices para Performance
CREATE INDEX IF NOT EXISTS idx_tinder_questions_validity ON tinder_questions(is_active, valid_from, valid_to);
CREATE INDEX IF NOT EXISTS idx_tinder_answers_user ON tinder_user_answers(user_id);
