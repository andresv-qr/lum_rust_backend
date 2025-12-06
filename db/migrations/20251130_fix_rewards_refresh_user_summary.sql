CREATE OR REPLACE FUNCTION rewards.refresh_user_summary(p_user_id integer)
 RETURNS void
 LANGUAGE plpgsql
AS $function$
BEGIN
  DELETE FROM rewards.user_invoice_summary WHERE user_id = p_user_id;

  INSERT INTO rewards.user_invoice_summary (
    user_id, total_facturas, total_monto, total_items, n_descuentos, total_descuento,
    top_emisores, top_categorias, serie_mensual, comparativo_categoria, updated_at
  )
  SELECT 
    p_user_id,
    COUNT(DISTINCT b.cufe),
    SUM(amount::float),
    COUNT(*),
    SUM(CASE WHEN unit_discount ~ '^[0-9]+(\.[0-9]+)?$' AND unit_discount::float > 0 THEN 1 ELSE 0 END),
    SUM(CASE WHEN unit_discount ~ '^[0-9]+(\.[0-9]+)?$' THEN unit_discount::float ELSE 0 END),
    
    -- Top emisores
    (
      SELECT jsonb_agg(row_to_json(t)) FROM (
        SELECT 
          upper(COALESCE(d.issuer_best_name, b.issuer_name)) AS issuer,
          SUM(b.tot_amount) AS monto
        FROM invoice_header b
        LEFT JOIN dim_issuer d ON b.issuer_name = d.issuer_name
        WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months'
        GROUP BY 1 ORDER BY monto DESC LIMIT 5
      ) t
    ),
    
    -- Top categorías
    (
      SELECT jsonb_agg(row_to_json(t)) FROM (
        SELECT 
          upper(COALESCE(p.l1, 'OTRO')) AS categoria,
          SUM(d.amount::float) AS monto
        FROM invoice_detail d
        JOIN invoice_header b ON b.cufe = d.cufe
        LEFT JOIN dim_product p ON p.code = d.code AND p.issuer_name = b.issuer_name AND p.description = d.description
        WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months'
        GROUP BY 1 ORDER BY monto DESC LIMIT 10
      ) t
    ),

    -- Serie mensual
    (
      jsonb_build_object(
        'issuer', (
          SELECT jsonb_agg(row_to_json(t)) FROM (
            SELECT 
              DATE_TRUNC('month', b.date) AS mes,
              upper(COALESCE(d.issuer_best_name, b.issuer_name)) AS issuer,
              SUM(b.tot_amount) AS monto
            FROM invoice_header b
            LEFT JOIN dim_issuer d ON b.issuer_name = d.issuer_name
            WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months'
            GROUP BY 1, 2 ORDER BY 1, 3 DESC
          ) t
        ),
        'issuer_category', (
          SELECT jsonb_agg(row_to_json(t)) FROM (
            SELECT 
              DATE_TRUNC('month', b.date) AS mes,
              upper(COALESCE(d.issuer_l1, 'OTRO')) AS issuer_l1,
              SUM(b.tot_amount) AS monto
            FROM invoice_header b
            LEFT JOIN dim_issuer d ON b.issuer_name = d.issuer_name
            WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months'
            GROUP BY 1, 2 ORDER BY 1, 3 DESC
          ) t
        ),
        'category', (
          SELECT jsonb_agg(row_to_json(t)) FROM (
            SELECT 
              DATE_TRUNC('month', b.date) AS mes,
              upper(COALESCE(p.l1, 'OTRO')) AS categoria,
              SUM(d.amount::float) AS monto
            FROM invoice_detail d
            JOIN invoice_header b ON b.cufe = d.cufe
            LEFT JOIN dim_product p ON p.code = d.code AND p.issuer_name = b.issuer_name AND p.description = d.description
            WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months'
            GROUP BY 1, 2 ORDER BY 1, 3 DESC
          ) t
        ),
        'summary', (
          SELECT jsonb_agg(row_to_json(t)) FROM (
            SELECT 
              DATE_TRUNC('month', b.date) AS mes,
              SUM(d.amount::float) AS monto,
              SUM(CASE WHEN d.unit_discount ~ '^[0-9]+(\\.[0-9]+)?$' THEN d.unit_discount::float ELSE 0 END) AS descuento,
              SUM(CASE WHEN d.unit_discount ~ '^[0-9]+(\\.[0-9]+)?$' AND d.unit_discount::float > 0 THEN 1 ELSE 0 END) AS n_descuentos,
              COUNT(*) AS tot_items
            FROM invoice_detail d
            JOIN invoice_header b ON b.cufe = d.cufe
            WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months'
            GROUP BY 1 ORDER BY 1
          ) t
        )
      )
    ),

    -- Comparativo por categoría
    (
      SELECT jsonb_agg(row_to_json(t)) FROM (
        WITH cliente_categoria AS (
          SELECT 
            upper(COALESCE(p.l1, 'OTRO')) AS categoria,
            SUM(d.amount::float) AS monto_cliente
          FROM invoice_detail d
          JOIN invoice_header b ON b.cufe = d.cufe
          LEFT JOIN dim_product p ON p.code = d.code AND p.issuer_name = b.issuer_name AND p.description = d.description
          WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months'
          GROUP BY 1
        ),
        total_cliente AS (
          SELECT SUM(monto_cliente) AS total_cliente FROM cliente_categoria
        ),
        promedio_categoria AS (
          SELECT 
            upper(COALESCE(p.l1, 'OTRO')) AS categoria,
            SUM(d.amount::float) AS monto_promedio_general
          FROM invoice_detail d
          JOIN invoice_header b ON b.cufe = d.cufe
          LEFT JOIN dim_product p ON p.code = d.code AND p.issuer_name = b.issuer_name AND p.description = d.description
          WHERE b.date >= CURRENT_DATE - INTERVAL '6 months'
          GROUP BY 1
        ),
        total_general AS (
          SELECT SUM(monto_promedio_general) AS total_general FROM promedio_categoria
        )
        SELECT 
          c.categoria,
          c.monto_cliente,
          p.monto_promedio_general,
          ROUND((c.monto_cliente / NULLIF(t1.total_cliente, 0) * 100)::numeric, 2) AS pct_cliente,
          ROUND((p.monto_promedio_general / NULLIF(t2.total_general, 0) * 100)::numeric, 2) AS pct_general,
          ROUND((c.monto_cliente / NULLIF(t1.total_cliente, 0) * 100)::numeric, 2) - ROUND((p.monto_promedio_general / NULLIF(t2.total_general, 0) * 100)::numeric, 2) AS var_relativa
        FROM cliente_categoria c
        LEFT JOIN promedio_categoria p USING (categoria)
        CROSS JOIN total_cliente t1
        CROSS JOIN total_general t2
        ORDER BY var_relativa DESC NULLS LAST
      ) t
    ),

    now()
  FROM invoice_detail a
  JOIN invoice_header b ON a.cufe = b.cufe
  LEFT JOIN dim_product p ON p.code = a.code AND p.issuer_name = b.issuer_name AND p.description = a.description
  WHERE b.user_id = p_user_id AND b.date >= CURRENT_DATE - INTERVAL '6 months';
END;
$function$;
