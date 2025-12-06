-- Migration: Add streak accumulations to rewards.dim_accumulations
-- Adds two separate accumulations for week_perfect (daily-login 7 days) and consistent_month (4 weeks)

BEGIN;

INSERT INTO rewards.dim_accumulations (name, valid_from, points, name_friendly, description_friendly, update_date)
VALUES
('gamification_week_perfect', NOW(), 1, 'Semana Perfecta', 'Recompensa por 7 días de login consecutivos', NOW())
ON CONFLICT (name) DO NOTHING;

INSERT INTO rewards.dim_accumulations (name, valid_from, points, name_friendly, description_friendly, update_date)
VALUES
('gamification_consistent_month', NOW(), 1, 'Perfect Month', 'Recompensa por 4 semanas consecutivas de facturas', NOW())
ON CONFLICT (name) DO NOTHING;

COMMIT;
