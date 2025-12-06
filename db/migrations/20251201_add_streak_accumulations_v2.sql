-- Migration: Add streak accumulations to rewards.dim_accumulations (safe insert if not exists)
-- Adds two separate accumulations for week_perfect (daily-login 7 days) and consistent_month (4 weeks)

BEGIN;

-- Fix sequence if it is out of sync (set to max id so nextval is max+1)
SELECT setval('rewards.dim_accumulations_id_seq', (SELECT MAX(id) FROM rewards.dim_accumulations));

INSERT INTO rewards.dim_accumulations (name, valid_from, points, name_friendly, description_friendly, update_date)
SELECT 'gamification_week_perfect', NOW(), 1, 'Semana Perfecta', 'Recompensa por 7 días de login consecutivos', NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM rewards.dim_accumulations WHERE name = 'gamification_week_perfect'
);

INSERT INTO rewards.dim_accumulations (name, valid_from, points, name_friendly, description_friendly, update_date)
SELECT 'gamification_consistent_month', NOW(), 1, 'Perfect Month', 'Recompensa por 4 semanas consecutivas de facturas', NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM rewards.dim_accumulations WHERE name = 'gamification_consistent_month'
);

COMMIT;
