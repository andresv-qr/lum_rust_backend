-- Migration: Update User Levels to "Lüm Universe" Theme
-- Description: Replaces generic levels with a 17-tier cosmic progression system.
-- Strategy: Use a temporary placeholder to preserve user_status integrity during the swap.

BEGIN;

-- 1. Create a temporary placeholder level to hold users during migration
INSERT INTO gamification.dim_user_levels (level_id, level_number, level_name, min_xp, max_xp, benefits_json)
VALUES (9999, 0, 'MIGRATION_PLACEHOLDER', 0, 0, '{}')
ON CONFLICT (level_id) DO NOTHING;

-- 1.1 Drop FK constraints on backup tables that might block deletion
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fact_user_progression_current_level_fkey' AND table_name = '_backup_fact_user_progression') THEN
        ALTER TABLE gamification._backup_fact_user_progression DROP CONSTRAINT fact_user_progression_current_level_fkey;
    END IF;
END$$;

-- 2. Move all users to the placeholder level
UPDATE gamification.user_status SET current_level_id = 9999;

-- 3. Delete all old levels (excluding the placeholder)
DELETE FROM gamification.dim_user_levels WHERE level_id != 9999;

-- 4. Reset the sequence for level_id (assuming standard serial naming convention)
ALTER SEQUENCE gamification.dim_user_levels_level_id_seq RESTART WITH 1;

-- 5. Insert the new "Lüm Universe" levels
INSERT INTO gamification.dim_user_levels (level_number, level_name, min_xp, max_xp, level_color, benefits_json) VALUES
-- TIER 1: ORIGEN (0 - 49)
(1, 'Chispa', 0, 9, '#FFCCBC', '{"lumi_multiplier": 1.0, "description": "El inicio de la luz."}'),
(2, 'Llama', 10, 24, '#FF7043', '{"lumi_multiplier": 1.05, "description": "Tu luz comienza a crecer."}'),
(3, 'Antorcha', 25, 49, '#D84315', '{"lumi_multiplier": 1.10, "description": "Iluminas el camino."}'),

-- TIER 2: ATMÓSFERA (50 - 149)
(4, 'Viento', 50, 74, '#B2DFDB', '{"lumi_multiplier": 1.15, "description": "Te elevas sobre el suelo."}'),
(5, 'Nube', 75, 99, '#4DB6AC', '{"lumi_multiplier": 1.20, "description": "Alcanzas nuevas alturas."}'),
(6, 'Tormenta', 100, 149, '#00695C', '{"lumi_multiplier": 1.25, "description": "Tu poder se siente."}'),

-- TIER 3: ESPACIO (150 - 299)
(7, 'Luna', 150, 199, '#E1BEE7', '{"lumi_multiplier": 1.30, "description": "Brillas en la oscuridad."}'),
(8, 'Planeta', 200, 249, '#BA68C8', '{"lumi_multiplier": 1.35, "description": "Un mundo propio."}'),
(9, 'Sol', 250, 299, '#FFD54F', '{"lumi_multiplier": 1.40, "description": "Centro de tu sistema."}'),

-- TIER 4: COSMOS (300 - 499)
(10, 'Cometa', 300, 349, '#4FC3F7', '{"lumi_multiplier": 1.45, "description": "Velocidad y estela."}'),
(11, 'Estrella', 350, 399, '#0288D1', '{"lumi_multiplier": 1.50, "description": "Luz distante y poderosa."}'),
(12, 'Nebulosa', 400, 449, '#303F9F', '{"lumi_multiplier": 1.55, "description": "Cuna de estrellas."}'),
(13, 'Galaxia', 450, 499, '#311B92', '{"lumi_multiplier": 1.60, "description": "Un universo en sí mismo."}'),

-- TIER 5: LÜM UNIVERSE (500+)
(14, 'Magio', 500, 599, '#F48FB1', '{"lumi_multiplier": 1.70, "description": "Dominio de la magia Lüm.", "vip_access": true}'),
(15, 'Magio Supremo', 600, 749, '#C2185B', '{"lumi_multiplier": 1.80, "description": "Poder sin límites.", "vip_access": true}'),
(16, 'Arquitecto', 750, 999, '#212121', '{"lumi_multiplier": 1.90, "description": "Constructor de realidades.", "vip_access": true, "personal_manager": true}'),
(17, 'Omnisciente', 1000, 999999, '#FFD700', '{"lumi_multiplier": 2.00, "description": "El todo y la nada.", "vip_access": true, "personal_manager": true, "exclusive_events": true}');

-- 6. Recalculate levels for all users based on their invoice count
-- We iterate over all users in user_status and call the update function
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT user_id FROM gamification.user_status LOOP
        PERFORM gamification.update_user_level(r.user_id);
    END LOOP;
END$$;

-- 7. Delete the placeholder level
-- Note: If any user didn't match a new level (e.g. negative invoices?), they might still be here.
-- But our levels cover 0 to 999999, so it should be safe.
DELETE FROM gamification.dim_user_levels WHERE level_id = 9999;

COMMIT;
