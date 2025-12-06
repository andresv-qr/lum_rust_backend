-- ============================================================================
-- SCRIPT DE PREGUNTAS GENERALES PANAMÁ - LUMIMATCH
-- Ejecutar: psql -h localhost -d tfactu -U avalencia -f lumimatch_seed_panama_questions.sql
-- ============================================================================

BEGIN;

-- Función auxiliar para insertar preguntas y opciones más fácilmente
-- (Esto es solo para el script, no se guarda en la BD)
CREATE OR REPLACE FUNCTION temp_insert_question(
    _title TEXT, 
    _opt1 TEXT, _icon1 TEXT, 
    _opt2 TEXT, _icon2 TEXT
) RETURNS VOID AS $$
DECLARE
    _q_id UUID;
BEGIN
    INSERT INTO lumimatch.questions (title, priority, targeting_rules)
    VALUES (_title, 50, '{}'::jsonb)
    RETURNING id INTO _q_id;

    INSERT INTO lumimatch.options (question_id, label, icon_url, display_order) VALUES
    (_q_id, _opt1, _icon1, 1),
    (_q_id, _opt2, _icon2, 2);
END;
$$ LANGUAGE plpgsql;

-- 1. Preferencias de Turismo Interno
SELECT temp_insert_question(
    '¿Qué prefieres para un fin de semana largo?',
    'Playa', 'icon_beach',
    'Montaña', 'icon_mountain'
);

-- 2. Gastronomía Panameña - Plato Fuerte
SELECT temp_insert_question(
    '¿Cuál es el rey de la comida panameña?',
    'Sancocho', 'icon_soup',
    'Arroz con Pollo', 'icon_rice_chicken'
);

-- 3. Transporte
SELECT temp_insert_question(
    '¿Cómo prefieres moverte por la ciudad?',
    'Metro', 'icon_subway',
    'Metrobús / Auto', 'icon_bus_car'
);

-- 4. Hábitos de Compra
SELECT temp_insert_question(
    '¿Cómo prefieres hacer tus compras?',
    'En tienda física', 'icon_store',
    'Online / Delivery', 'icon_delivery'
);

-- 5. Métodos de Pago
SELECT temp_insert_question(
    '¿Tu método de pago favorito?',
    'Efectivo', 'icon_cash',
    'Yappy / Tarjeta', 'icon_credit_card'
);

-- 6. Entretenimiento
SELECT temp_insert_question(
    '¿Plan ideal para el viernes?',
    'Cine', 'icon_cinema',
    'Netflix / Streaming', 'icon_tv'
);

-- 7. Bebidas Calientes
SELECT temp_insert_question(
    '¿Para empezar el día?',
    'Café', 'icon_coffee',
    'Té', 'icon_tea'
);

-- 8. Mascotas
SELECT temp_insert_question(
    '¿Eres team perros o gatos?',
    'Perros', 'icon_dog',
    'Gatos', 'icon_cat'
);

-- 9. Horarios
SELECT temp_insert_question(
    '¿Cómo funcionas mejor?',
    'Madrugar', 'icon_sunrise',
    'Trasnochar', 'icon_moon'
);

-- 10. Clima
SELECT temp_insert_question(
    '¿Qué prefieres?',
    'Verano (Sol)', 'icon_sun',
    'Invierno (Lluvia)', 'icon_rain'
);

-- 11. Carnavales
SELECT temp_insert_question(
    '¿Dónde pasas los carnavales?',
    'Las Tablas / Interior', 'icon_party',
    'La City / Relax', 'icon_city'
);

-- 12. Desayuno Típico
SELECT temp_insert_question(
    '¿El acompañamiento perfecto?',
    'Hojaldra', 'icon_fried_dough',
    'Tortilla', 'icon_corn_tortilla'
);

-- 13. Acompañamiento
SELECT temp_insert_question(
    '¿Para acompañar el almuerzo?',
    'Patacones', 'icon_plantain',
    'Yuca Frita', 'icon_yuca'
);

-- 14. Frecuencia de Supermercado
SELECT temp_insert_question(
    '¿Cómo haces el súper?',
    'Frecuente (poquito)', 'icon_basket',
    'Quincenal (grande)', 'icon_cart_full'
);

-- 15. Comida en Casa
SELECT temp_insert_question(
    '¿Hoy qué toca?',
    'Cocinar', 'icon_cooking',
    'Pedir Delivery', 'icon_motorcycle'
);

-- 16. Raspao
SELECT temp_insert_question(
    '¿Sabor de raspao favorito?',
    'Rojo (Fresa/Colita)', 'icon_shaved_ice_red',
    'Maracuyá / Limón', 'icon_shaved_ice_yellow'
);

-- 17. Cerveza
SELECT temp_insert_question(
    '¿Qué prefieres tomar?',
    'Cerveza Nacional', 'icon_beer',
    'Cerveza Artesanal', 'icon_craft_beer'
);

-- 18. Mood Fin de Semana
SELECT temp_insert_question(
    '¿Tu mood actual?',
    'Relax en casa', 'icon_sofa',
    'Fiesta / Salir', 'icon_disco'
);

-- 19. Ejercicio
SELECT temp_insert_question(
    '¿Dónde prefieres ejercitarte?',
    'Gimnasio', 'icon_dumbbell',
    'Aire Libre', 'icon_park'
);

-- 20. Finanzas
SELECT temp_insert_question(
    '¿Filosofía financiera?',
    'Ahorrar para el futuro', 'icon_piggy_bank',
    'Disfrutar el momento', 'icon_money_wings'
);

-- 21. Redes Sociales
SELECT temp_insert_question(
    '¿Dónde pasas más tiempo?',
    'Instagram', 'icon_instagram',
    'TikTok', 'icon_tiktok'
);

-- 22. Tecnología Móvil
SELECT temp_insert_question(
    '¿Tu sistema operativo?',
    'Android', 'icon_android',
    'iPhone (iOS)', 'icon_apple'
);

-- 23. Proteína
SELECT temp_insert_question(
    '¿Preferencia principal?',
    'Carne / Pollo', 'icon_meat',
    'Mariscos / Pescado', 'icon_fish'
);

-- 24. Antojos
SELECT temp_insert_question(
    '¿Qué se te antoja más?',
    'Algo Dulce', 'icon_candy',
    'Algo Salado', 'icon_pretzel'
);

-- 25. Cultura
SELECT temp_insert_question(
    '¿Para desconectar?',
    'Leer un libro', 'icon_book',
    'Ver una película', 'icon_movie'
);

-- 26. Viajes
SELECT temp_insert_question(
    '¿Próximas vacaciones?',
    'Turismo Interno', 'icon_map_panama',
    'Viaje al Exterior', 'icon_airplane'
);

-- 27. En el Tranque
SELECT temp_insert_question(
    '¿Qué escuchas en el tranque?',
    'Podcast / Noticias', 'icon_podcast',
    'Música a todo volumen', 'icon_music'
);

-- 28. Regalos
SELECT temp_insert_question(
    '¿Qué disfrutas más?',
    'Dar regalos', 'icon_gift_give',
    'Recibir regalos', 'icon_gift_receive'
);

-- 29. Modalidad de Trabajo
SELECT temp_insert_question(
    '¿Tu preferencia laboral?',
    'Trabajo Remoto', 'icon_laptop_home',
    'Oficina Presencial', 'icon_office_building'
);

-- 30. Polémica Pizza
SELECT temp_insert_question(
    'La pregunta definitiva:',
    'Pizza CON piña', 'icon_pizza_pineapple',
    'Pizza SIN piña', 'icon_pizza'
);

-- Limpiar función temporal
DROP FUNCTION temp_insert_question;

COMMIT;

SELECT 'Se han insertado 30 preguntas generales sobre Panamá y hábitos.' as status;
