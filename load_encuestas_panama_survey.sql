-- ============================================
-- CARGA DE ENCUESTAS - ESTUDIO DE MERCADO PANAMÁ
-- ============================================
-- Fecha: 2025-08-25
-- Descripción: Carga inicial de 4 encuestas sobre hábitos de consumo en Panamá
-- Esquema: survey (dimensional con auto-asignación)
-- ACTUALIZADO: Compatible con nueva estructura y targeting automático

-- ============================================
-- INSERCIÓN DE CAMPAÑA
-- ============================================

-- Insertar campaña principal
INSERT INTO survey.dim_campaigns (name, description, category, is_active) 
VALUES (
    'Estudio de Mercado Panamá 2025',
    'Investigación de hábitos de consumo, preferencias alimenticias y comportamiento del consumidor panameño',
    'Market Research',
    TRUE
) ON CONFLICT DO NOTHING;

-- Obtener campaign_id para las encuestas
-- Nota: Asumimos que será el campaign_id = 1, ajustar según sea necesario

-- ============================================
-- ENCUESTA 1: HÁBITOS ALIMENTICIOS Y DE CONSUMO
-- ============================================

INSERT INTO survey.dim_surveys (
    campaign_id,
    title,
    survey_description,
    instructions,
    questions,
    total_questions,
    max_attempts,
    time_limit_minutes,
    points_per_question,
    difficulty,
    target_audience,
    auto_assign,
    is_active
) VALUES (
    1, -- campaign_id
    'Hábitos Alimenticios y de Consumo',
    'Comprender los hábitos diarios de alimentación, frecuencia de consumo fuera de casa, preferencias de cocina y presupuesto destinado a comidas.',
    'Por favor responde todas las preguntas de manera honesta. No hay respuestas correctas o incorrectas, solo queremos conocer tus hábitos y preferencias.',
    '{
        "questions": [
            {
                "question_id": 1,
                "question_text": "¿Con qué frecuencia comes fuera de casa durante la semana?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Nunca", "is_correct": null},
                    {"value": "B", "text": "1-2 veces", "is_correct": null},
                    {"value": "C", "text": "3-5 veces", "is_correct": null},
                    {"value": "D", "text": "Más de 5 veces", "is_correct": null}
                ],
                "explanation": "Pregunta sobre frecuencia de consumo fuera del hogar."
            },
            {
                "question_id": 2,
                "question_text": "¿Qué tipo de comida consumes con más frecuencia? (selecciona hasta 3)",
                "question_type": "multiple_choice",
                "max_selections": 3,
                "options": [
                    {"value": "A", "text": "Comida panameña", "is_correct": null},
                    {"value": "B", "text": "Comida rápida", "is_correct": null},
                    {"value": "C", "text": "Comida saludable / fit", "is_correct": null},
                    {"value": "D", "text": "Comida asiática", "is_correct": null},
                    {"value": "E", "text": "Comida italiana", "is_correct": null},
                    {"value": "F", "text": "Comida vegetariana/vegana", "is_correct": null},
                    {"value": "G", "text": "Otra", "is_correct": null}
                ],
                "explanation": "Identificar preferencias gastronómicas principales."
            },
            {
                "question_id": 3,
                "question_text": "¿Prefieres salir a cenar o cocinar en casa?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Salir a cenar", "is_correct": null},
                    {"value": "B", "text": "Cocinar en casa", "is_correct": null},
                    {"value": "C", "text": "Depende del día", "is_correct": null}
                ],
                "explanation": "Preferencia entre comer fuera vs cocinar en casa."
            },
            {
                "question_id": 4,
                "question_text": "¿Cuál es tu presupuesto mensual aproximado para alimentos (supermercado y restaurantes)?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Menos de $150", "is_correct": null},
                    {"value": "B", "text": "$150 - $300", "is_correct": null},
                    {"value": "C", "text": "$301 - $500", "is_correct": null},
                    {"value": "D", "text": "Más de $500", "is_correct": null}
                ],
                "explanation": "Rango de presupuesto destinado a alimentación."
            },
            {
                "question_id": 5,
                "question_text": "¿Qué factores influyen más en tu elección de comida? (elige 2)",
                "question_type": "multiple_choice",
                "max_selections": 2,
                "options": [
                    {"value": "A", "text": "Precio", "is_correct": null},
                    {"value": "B", "text": "Sabor", "is_correct": null},
                    {"value": "C", "text": "Tiempo de preparación", "is_correct": null},
                    {"value": "D", "text": "Salud/nutrición", "is_correct": null},
                    {"value": "E", "text": "Variedad", "is_correct": null},
                    {"value": "F", "text": "Influencias de redes sociales", "is_correct": null}
                ],
                "explanation": "Factores de decisión en elección alimentaria."
            },
            {
                "question_id": 6,
                "question_text": "¿Tienes alguna restricción alimenticia?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Sí", "is_correct": null},
                    {"value": "B", "text": "No", "is_correct": null}
                ],
                "explanation": "Identificar restricciones dietéticas."
            },
            {
                "question_id": 7,
                "question_text": "¿Compras tus alimentos principalmente en...?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Supermercados grandes", "is_correct": null},
                    {"value": "B", "text": "Tiendas de conveniencia", "is_correct": null},
                    {"value": "C", "text": "Aplicaciones de delivery", "is_correct": null},
                    {"value": "D", "text": "Mercados locales / abastos", "is_correct": null},
                    {"value": "E", "text": "Otros", "is_correct": null}
                ],
                "explanation": "Canal principal de compra de alimentos."
            },
            {
                "question_id": 8,
                "question_text": "¿Qué tan importante es para ti que los alimentos sean locales?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Muy importante", "is_correct": null},
                    {"value": "B", "text": "Poco importante", "is_correct": null},
                    {"value": "C", "text": "Me es indiferente", "is_correct": null}
                ],
                "explanation": "Importancia de productos locales."
            },
            {
                "question_id": 9,
                "question_text": "¿Qué aplicación usas con mayor frecuencia para pedir comida a domicilio?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Appetito24", "is_correct": null},
                    {"value": "B", "text": "Uber Eats", "is_correct": null},
                    {"value": "C", "text": "PedidosYa", "is_correct": null},
                    {"value": "D", "text": "No uso apps", "is_correct": null},
                    {"value": "E", "text": "Otra", "is_correct": null}
                ],
                "explanation": "App preferida para delivery de comida."
            },
            {
                "question_id": 10,
                "question_text": "¿Hay algún platillo o tipo de comida que te gustaría ver más en los restaurantes locales?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Pregunta abierta sobre preferencias gastronómicas no satisfechas."
            }
        ]
    }',
    10, -- total_questions
    1,  -- max_attempts
    15, -- time_limit_minutes
    10, -- points_per_question
    'easy',
    'todos', -- target_audience: aplica para todos los usuarios
    TRUE,    -- auto_assign: se asigna automáticamente
    TRUE     -- is_active
);

-- ============================================
-- ENCUESTA 2: PREFERENCIAS Y EXPERIENCIAS EN CITAS
-- ============================================

INSERT INTO survey.dim_surveys (
    campaign_id,
    title,
    survey_description,
    instructions,
    questions,
    total_questions,
    max_attempts,
    time_limit_minutes,
    points_per_question,
    difficulty,
    target_audience,
    auto_assign,
    is_active
) VALUES (
    1, -- campaign_id
    'Preferencias y Experiencias en Citas',
    'Comprender hábitos y preferencias en salidas sociales o románticas, ya sea en primeras citas o con una pareja estable.',
    'Responde con sinceridad sobre tus experiencias y preferencias en citas. Toda la información es confidencial.',
    '{
        "questions": [
            {
                "question_id": 1,
                "question_text": "¿Actualmente estás?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Soltero/a", "is_correct": null},
                    {"value": "B", "text": "En una relación", "is_correct": null},
                    {"value": "C", "text": "Casado/a", "is_correct": null},
                    {"value": "D", "text": "Es complicado", "is_correct": null}
                ],
                "explanation": "Estado civil actual."
            },
            {
                "question_id": 2,
                "question_text": "¿Con qué frecuencia sales a citas (ya sea con alguien nuevo o tu pareja)?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Una vez a la semana o más", "is_correct": null},
                    {"value": "B", "text": "1-2 veces al mes", "is_correct": null},
                    {"value": "C", "text": "Raramente", "is_correct": null},
                    {"value": "D", "text": "Nunca", "is_correct": null}
                ],
                "explanation": "Frecuencia de citas o salidas románticas."
            },
            {
                "question_id": 3,
                "question_text": "¿Qué tipo de lugar prefieres para una cita?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Restaurante", "is_correct": null},
                    {"value": "B", "text": "Café", "is_correct": null},
                    {"value": "C", "text": "Parque o paseo al aire libre", "is_correct": null},
                    {"value": "D", "text": "Bar o lugar con música", "is_correct": null},
                    {"value": "E", "text": "Cine", "is_correct": null},
                    {"value": "F", "text": "Otro", "is_correct": null}
                ],
                "explanation": "Tipo de lugar preferido para citas."
            },
            {
                "question_id": 4,
                "question_text": "¿Qué tipo de comida prefieres al salir con tu pareja o en una cita?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Casual y cómoda (hamburguesas, pizza)", "is_correct": null},
                    {"value": "B", "text": "Comida gourmet", "is_correct": null},
                    {"value": "C", "text": "Comida típica panameña", "is_correct": null},
                    {"value": "D", "text": "Vegana/vegetariana", "is_correct": null},
                    {"value": "E", "text": "No importa / Soy flexible", "is_correct": null}
                ],
                "explanation": "Preferencia gastronómica en citas."
            },
            {
                "question_id": 5,
                "question_text": "¿Prefieres ir a un lugar nuevo o al mismo de siempre cuando sales con alguien?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Me encanta conocer lugares nuevos", "is_correct": null},
                    {"value": "B", "text": "Prefiero ir a lugares que ya conozco", "is_correct": null},
                    {"value": "C", "text": "Depende del momento o la ocasión", "is_correct": null}
                ],
                "explanation": "Preferencia por lugares nuevos vs conocidos."
            },
            {
                "question_id": 6,
                "question_text": "¿Quién suele pagar en tus citas?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Yo", "is_correct": null},
                    {"value": "B", "text": "La otra persona", "is_correct": null},
                    {"value": "C", "text": "Se divide la cuenta", "is_correct": null},
                    {"value": "D", "text": "Depende de la ocasión", "is_correct": null}
                ],
                "explanation": "Dinámica de pago en citas."
            },
            {
                "question_id": 7,
                "question_text": "¿Qué tan importante es para ti la experiencia gastronómica en una cita?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Muy importante", "is_correct": null},
                    {"value": "B", "text": "Moderadamente importante", "is_correct": null},
                    {"value": "C", "text": "Poco importante", "is_correct": null},
                    {"value": "D", "text": "No le doy importancia", "is_correct": null}
                ],
                "explanation": "Importancia de la experiencia gastronómica."
            },
            {
                "question_id": 8,
                "question_text": "¿Usas aplicaciones para conocer personas o planear citas?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Sí, para conocer personas", "is_correct": null},
                    {"value": "B", "text": "Sí, para planificar actividades o lugares", "is_correct": null},
                    {"value": "C", "text": "No", "is_correct": null},
                    {"value": "D", "text": "No, pero lo he considerado", "is_correct": null}
                ],
                "explanation": "Uso de aplicaciones para citas."
            },
            {
                "question_id": 9,
                "question_text": "¿Qué tan relevante es el presupuesto a la hora de planear una salida o cita?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Muy relevante", "is_correct": null},
                    {"value": "B", "text": "Algo relevante", "is_correct": null},
                    {"value": "C", "text": "Poco relevante", "is_correct": null},
                    {"value": "D", "text": "No pienso en eso", "is_correct": null}
                ],
                "explanation": "Relevancia del presupuesto en citas."
            },
            {
                "question_id": 10,
                "question_text": "¿Cuál ha sido tu mejor o peor experiencia en una cita, relacionada con el lugar o la comida?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Pregunta abierta sobre experiencias memorables en citas."
            }
        ]
    }',
    10, -- total_questions
    1,  -- max_attempts
    12, -- time_limit_minutes
    10, -- points_per_question
    'easy',
    'todos', -- target_audience: aplica para todos los usuarios
    TRUE,    -- auto_assign: se asigna automáticamente
    TRUE     -- is_active
);

-- ============================================
-- ENCUESTA 3: ESTILO DE VIDA Y PERFIL SOCIOECONÓMICO
-- ============================================

INSERT INTO survey.dim_surveys (
    campaign_id,
    title,
    survey_description,
    instructions,
    questions,
    total_questions,
    max_attempts,
    time_limit_minutes,
    points_per_question,
    difficulty,
    target_audience,
    auto_assign,
    is_active
) VALUES (
    1, -- campaign_id
    'Estilo de Vida y Perfil Socioeconómico',
    'Conocer aspectos demográficos, ingresos, prioridades de gasto y su relación con el estilo de vida y consumo gastronómico.',
    'Esta información nos ayuda a entender mejor el perfil de nuestros usuarios. Todos los datos son confidenciales y utilizados únicamente para fines estadísticos.',
    '{
        "questions": [
            {
                "question_id": 1,
                "question_text": "¿Cuál es tu rango de edad?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Menos de 18", "is_correct": null},
                    {"value": "B", "text": "18-24", "is_correct": null},
                    {"value": "C", "text": "25-34", "is_correct": null},
                    {"value": "D", "text": "35-44", "is_correct": null},
                    {"value": "E", "text": "45-54", "is_correct": null},
                    {"value": "F", "text": "55 o más", "is_correct": null}
                ],
                "explanation": "Rango de edad demográfico."
            },
            {
                "question_id": 2,
                "question_text": "¿En qué provincia de Panamá resides?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Panamá", "is_correct": null},
                    {"value": "B", "text": "Panamá Oeste", "is_correct": null},
                    {"value": "C", "text": "Colón", "is_correct": null},
                    {"value": "D", "text": "Chiriquí", "is_correct": null},
                    {"value": "E", "text": "Veraguas", "is_correct": null},
                    {"value": "F", "text": "Otra", "is_correct": null}
                ],
                "explanation": "Ubicación geográfica del usuario."
            },
            {
                "question_id": 3,
                "question_text": "¿Cuál es tu nivel educativo más alto alcanzado?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Secundaria", "is_correct": null},
                    {"value": "B", "text": "Técnico", "is_correct": null},
                    {"value": "C", "text": "Universitario (pregrado)", "is_correct": null},
                    {"value": "D", "text": "Postgrado", "is_correct": null},
                    {"value": "E", "text": "Otro", "is_correct": null}
                ],
                "explanation": "Nivel educativo alcanzado."
            },
            {
                "question_id": 4,
                "question_text": "¿Cuál es tu rango de ingreso mensual personal (en balboas)?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Menos de B/. 500", "is_correct": null},
                    {"value": "B", "text": "B/. 500 - 1,000", "is_correct": null},
                    {"value": "C", "text": "B/. 1,001 - 2,000", "is_correct": null},
                    {"value": "D", "text": "Más de B/. 2,000", "is_correct": null},
                    {"value": "E", "text": "Prefiero no decir", "is_correct": null}
                ],
                "explanation": "Rango de ingresos mensuales."
            },
            {
                "question_id": 5,
                "question_text": "¿En qué gastas la mayor parte de tu ingreso mensual? (elige hasta 2)",
                "question_type": "multiple_choice",
                "max_selections": 2,
                "options": [
                    {"value": "A", "text": "Alimentación", "is_correct": null},
                    {"value": "B", "text": "Transporte", "is_correct": null},
                    {"value": "C", "text": "Entretenimiento", "is_correct": null},
                    {"value": "D", "text": "Alquiler o hipoteca", "is_correct": null},
                    {"value": "E", "text": "Educación", "is_correct": null},
                    {"value": "F", "text": "Ahorros/inversión", "is_correct": null}
                ],
                "explanation": "Principales categorías de gasto mensual."
            },
            {
                "question_id": 6,
                "question_text": "¿Qué tan importante es para ti cuidar tu alimentación?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Muy importante", "is_correct": null},
                    {"value": "B", "text": "Algo importante", "is_correct": null},
                    {"value": "C", "text": "Poco importante", "is_correct": null},
                    {"value": "D", "text": "No me interesa", "is_correct": null}
                ],
                "explanation": "Importancia del cuidado alimentario."
            },
            {
                "question_id": 7,
                "question_text": "¿Con qué frecuencia haces compras en línea?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Varias veces por semana", "is_correct": null},
                    {"value": "B", "text": "1-2 veces al mes", "is_correct": null},
                    {"value": "C", "text": "Raramente", "is_correct": null},
                    {"value": "D", "text": "Nunca", "is_correct": null}
                ],
                "explanation": "Frecuencia de compras en línea."
            },
            {
                "question_id": 8,
                "question_text": "¿Tienes hijos o dependientes?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Sí", "is_correct": null},
                    {"value": "B", "text": "No", "is_correct": null}
                ],
                "explanation": "Información sobre dependientes."
            },
            {
                "question_id": 9,
                "question_text": "¿Cuál es tu estado laboral actual?",
                "question_type": "single_choice",
                "options": [
                    {"value": "A", "text": "Empleado/a", "is_correct": null},
                    {"value": "B", "text": "Independiente", "is_correct": null},
                    {"value": "C", "text": "Estudiante", "is_correct": null},
                    {"value": "D", "text": "Desempleado/a", "is_correct": null},
                    {"value": "E", "text": "Jubilado/a", "is_correct": null}
                ],
                "explanation": "Estado laboral actual."
            },
            {
                "question_id": 10,
                "question_text": "¿Cuál consideras que es el mayor reto económico que enfrentas actualmente?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Pregunta abierta sobre desafíos económicos personales."
            }
        ]
    }',
    10, -- total_questions
    1,  -- max_attempts
    10, -- time_limit_minutes
    10, -- points_per_question
    'easy',
    'todos', -- target_audience: aplica para todos los usuarios
    TRUE,    -- auto_assign: se asigna automáticamente
    TRUE     -- is_active
);

-- ============================================
-- ENCUESTA 4: TOP OF MIND - MARCAS Y PREFERENCIAS EN PANAMÁ
-- ============================================

INSERT INTO survey.dim_surveys (
    campaign_id,
    title,
    survey_description,
    instructions,
    questions,
    total_questions,
    max_attempts,
    time_limit_minutes,
    points_per_question,
    difficulty,
    target_audience,
    auto_assign,
    is_active
) VALUES (
    1, -- campaign_id
    'Top of Mind: Marcas y Preferencias en Panamá',
    'Identificar qué marcas, lugares o servicios vienen primero a la mente del consumidor en diferentes categorías clave del mercado panameño.',
    'Responde con la primera marca o nombre que se te venga a la mente. No pienses mucho, queremos tu respuesta más espontánea.',
    '{
        "questions": [
            {
                "question_id": 1,
                "question_text": "¿Cuál es el primer supermercado que se te viene a la mente?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en supermercados."
            },
            {
                "question_id": 2,
                "question_text": "¿Cuál es el primer restaurante que piensas cuando quieres salir a comer?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en restaurantes."
            },
            {
                "question_id": 3,
                "question_text": "¿Qué aplicación de delivery de comida te viene primero a la mente?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en apps de delivery."
            },
            {
                "question_id": 4,
                "question_text": "¿Qué marca de agua embotellada recuerdas primero?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en agua embotellada."
            },
            {
                "question_id": 5,
                "question_text": "¿Cuál es la primera marca de café que piensas?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en marcas de café."
            },
            {
                "question_id": 6,
                "question_text": "¿Qué marca de productos de limpieza del hogar piensas primero?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en productos de limpieza."
            },
            {
                "question_id": 7,
                "question_text": "¿Qué lugar piensas primero cuando quieres planear una cita o salida en pareja?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en lugares para citas."
            },
            {
                "question_id": 8,
                "question_text": "¿Qué banco se te viene a la mente primero cuando piensas en abrir una cuenta o sacar un préstamo?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en servicios bancarios."
            },
            {
                "question_id": 9,
                "question_text": "¿Qué marca de celular piensas primero cuando piensas en comprar uno nuevo?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en marcas de celulares."
            },
            {
                "question_id": 10,
                "question_text": "¿Qué canal de televisión, programa o medio digital se te viene a la mente cuando piensas en noticias?",
                "question_type": "open_text",
                "options": [],
                "explanation": "Top of mind en medios de noticias."
            }
        ]
    }',
    10, -- total_questions
    1,  -- max_attempts
    8,  -- time_limit_minutes
    10, -- points_per_question
    'easy',
    'todos', -- target_audience: aplica para todos los usuarios
    TRUE,    -- auto_assign: se asigna automáticamente
    TRUE     -- is_active
);

-- ============================================
-- VERIFICACIÓN DE CARGA
-- ============================================

-- Consulta para verificar que las encuestas se cargaron correctamente
SELECT 
    c.name AS campaign_name,
    s.survey_id,
    s.title,
    s.total_questions,
    s.difficulty,
    s.created_at
FROM survey.dim_surveys s
INNER JOIN survey.dim_campaigns c ON s.campaign_id = c.campaign_id
WHERE c.name = 'Estudio de Mercado Panamá 2025'
ORDER BY s.survey_id;

-- ============================================
-- COMENTARIOS DE USO
-- ============================================

/*
INSTRUCCIONES PARA USO:

1. Ejecutar este archivo SQL en la base de datos donde ya existe el esquema survey
2. Verificar que se crearon las 4 encuestas correctamente
3. ¡IMPORTANTE! Las encuestas se asignan AUTOMÁTICAMENTE a todos los usuarios
   - target_audience = 'todos'
   - auto_assign = TRUE
   - Los triggers se encargan de la asignación automática

FUNCIONES DISPONIBLES:

-- Auto-asignación para todos los usuarios existentes (se ejecuta automáticamente)
SELECT survey.api_auto_assign_surveys();

-- Auto-asignación rápida para nuevos usuarios 
SELECT survey.api_auto_assign_surveys_async(user_id, user_profile_jsonb);

-- Asignación manual específica (opcional)
SELECT survey.api_assign_survey(user_id, survey_id, due_date, is_mandatory);

CONSULTAS ÚTILES:

-- Ver todas las encuestas de un usuario
SELECT * FROM survey.v_user_surveys WHERE user_id = 1;

-- Ver solo encuestas pendientes
SELECT * FROM survey.api_get_user_surveys(1, 'pending'::text);

-- Obtener encuesta específica con preguntas
SELECT survey.api_get_survey_details(1, 1);

-- Ver estadísticas de asignación
SELECT 
    s.title,
    s.target_audience,
    s.auto_assign,
    COUNT(fuss.user_id) as usuarios_asignados,
    COUNT(CASE WHEN fuss.status = 'completed' THEN 1 END) as completadas
FROM survey.dim_surveys s
LEFT JOIN survey.fact_user_survey_status fuss ON s.survey_id = fuss.survey_id
WHERE s.campaign_id = 1
GROUP BY s.survey_id, s.title, s.target_audience, s.auto_assign
ORDER BY s.survey_id;
*/
