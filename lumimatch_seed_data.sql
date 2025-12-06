-- ============================================================================
-- SCRIPT DE DATOS DE PRUEBA (SEED) - LUMIMATCH
-- Ejecutar: psql -h localhost -d tfactu -U avalencia -f lumimatch_seed_data.sql
-- ============================================================================

BEGIN;

-- 1. Limpiar datos previos de prueba (Opcional, comentar si no se desea)
-- TRUNCATE TABLE lumimatch.user_answers CASCADE;
-- DELETE FROM lumimatch.questions WHERE title LIKE '[TEST]%';
-- DELETE FROM lumimatch.user_tags WHERE user_id = 1;

-- 2. Crear Preguntas de Prueba

-- A. Pregunta General (Para todos)
INSERT INTO lumimatch.questions (id, title, image_url, priority, targeting_rules)
VALUES (
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11',
    '[TEST] ¿Qué prefieres para desayunar?',
    'https://images.unsplash.com/photo-1533089862017-5614ec95e941',
    100,
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO lumimatch.options (question_id, label, icon_url, display_order) VALUES
('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'Café', 'coffee_icon', 1),
('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'Té', 'tea_icon', 2)
ON CONFLICT DO NOTHING;


-- B. Pregunta Segmentada por Producto (Solo para compradores de Coca-Cola)
INSERT INTO lumimatch.questions (id, title, image_url, priority, targeting_rules)
VALUES (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b22',
    '[TEST] Vimos que compraste Coca-Cola, ¿te gustaría probar la versión Zero?',
    'https://images.unsplash.com/photo-1622483767028-3f66f32aef97',
    90,
    '{"product_brands": ["cocacola"]}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO lumimatch.options (question_id, label, display_order) VALUES
('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b22', 'Sí, claro', 1),
('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b22', 'No, prefiero la normal', 2)
ON CONFLICT DO NOTHING;


-- C. Pregunta Segmentada por Comercio (Solo clientes de McDonald's)
INSERT INTO lumimatch.questions (id, title, priority, targeting_rules)
VALUES (
    'c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c33',
    '[TEST] ¿Cómo calificarías tu última visita a McDonald''s?',
    80,
    '{"issuer_brand_names": ["mcdonalds"]}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO lumimatch.options (question_id, label, display_order) VALUES
('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c33', 'Excelente', 1),
('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c33', 'Regular', 2),
('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c33', 'Mala', 3)
ON CONFLICT DO NOTHING;


-- D. Pregunta de Fecha Específica (Solo hoy)
INSERT INTO lumimatch.questions (id, title, priority, specific_date, targeting_rules)
VALUES (
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d44',
    '[TEST] Pregunta exclusiva de HOY',
    200,
    CURRENT_DATE, -- Se establece para el día de hoy dinámicamente
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO lumimatch.options (question_id, label, display_order) VALUES
('d0eebc99-9c0b-4ef8-bb6d-6bb9bd380d44', 'Entendido', 1)
ON CONFLICT DO NOTHING;


-- 3. Asignar Tags al Usuario de Prueba (ID 1)
-- Asumimos que el usuario ID 1 existe. Si usas otro ID, cámbialo aquí.

-- Tag de marca (habilita la pregunta B)
INSERT INTO lumimatch.user_tags (user_id, tag) VALUES (1, 'product_brand:cocacola') ON CONFLICT DO NOTHING;

-- Tag de comercio (habilita la pregunta C)
INSERT INTO lumimatch.user_tags (user_id, tag) VALUES (1, 'issuer_brand_name:mcdonalds') ON CONFLICT DO NOTHING;

-- Tag de perfil (ejemplo extra)
INSERT INTO lumimatch.user_tags (user_id, tag) VALUES (1, 'vip') ON CONFLICT DO NOTHING;


COMMIT;

SELECT 'Datos de prueba insertados correctamente. Usuario 1 debería ver 4 preguntas nuevas.' as status;
