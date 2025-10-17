-- Verificación de datos del Daily Game
-- Ejecutar con: \i verify_daily_game_data.sql

\echo '===================================================='
\echo '📊 VERIFICACIÓN DE DAILY GAME'
\echo '===================================================='
\echo ''

\echo '1️⃣ Jugadas registradas (fact_daily_game_plays):'
\echo '----------------------------------------------------'
SELECT 
    id,
    user_id,
    play_date,
    play_time,
    star_id,
    lumis_won,
    created_at
FROM rewards.fact_daily_game_plays
ORDER BY id DESC
LIMIT 5;

\echo ''
\echo '2️⃣ Acumulaciones generadas (fact_accumulations):'
\echo '----------------------------------------------------'
SELECT 
    user_id,
    accum_id,
    accum_type,
    quantity,
    date
FROM rewards.fact_accumulations
WHERE accum_type = 'daily_game'
ORDER BY date DESC
LIMIT 5;

\echo ''
\echo '3️⃣ Balance actualizado del usuario 1:'
\echo '----------------------------------------------------'
SELECT 
    user_id,
    total_points,
    updated_at
FROM rewards.fact_balance_points
WHERE user_id = 1;

\echo ''
\echo '4️⃣ Estadísticas globales:'
\echo '----------------------------------------------------'
SELECT 
    COUNT(*) as total_plays,
    COUNT(DISTINCT user_id) as unique_players,
    SUM(lumis_won) as total_lumis_distributed,
    SUM(CASE WHEN lumis_won = 5 THEN 1 ELSE 0 END) as golden_stars,
    SUM(CASE WHEN lumis_won = 1 THEN 1 ELSE 0 END) as normal_stars,
    SUM(CASE WHEN lumis_won = 0 THEN 1 ELSE 0 END) as empty_stars
FROM rewards.fact_daily_game_plays;

\echo ''
\echo '✅ Verificación completada'
\echo '===================================================='
