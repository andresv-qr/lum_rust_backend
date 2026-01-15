-- Migration: Invoice notifications action_url -> /mi_historial
-- Date: 2025-12-18
-- Purpose: Frontend wants invoices notifications to open /mi_historial instead of invoice detail.

BEGIN;

CREATE OR REPLACE FUNCTION public.notify_invoice_processed()
RETURNS TRIGGER AS $$
DECLARE
    v_merchant TEXT;
    v_amount_text TEXT;
BEGIN
    IF TG_OP = 'INSERT' AND NEW.user_id IS NOT NULL THEN
        v_merchant := COALESCE(NEW.issuer_name, 'comercio');
        v_amount_text := to_char(COALESCE(NEW.tot_amount, 0)::numeric, 'FM999999990.00');

        PERFORM public.create_notification(
            p_user_id := NEW.user_id,
            p_title := '¡Factura procesada!',
            p_body := FORMAT('Tu factura de %s por valor de $%s fue procesada exitosamente.', v_merchant, v_amount_text),
            p_type := 'invoice',
            p_priority := 'normal',
            p_action_url := '/mi_historial',
            p_payload := jsonb_build_object(
                'cufe', NEW.cufe,
                'merchant_name', NEW.issuer_name,
                'amount', NEW.tot_amount,
                'date', NEW.date
            ),
            p_idempotency_key := FORMAT('invoice_%s', NEW.cufe),
            p_send_push := TRUE
        );
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Recreate trigger idempotently
DROP TRIGGER IF EXISTS trg_notify_invoice_processed ON public.invoice_header;

CREATE TRIGGER trg_notify_invoice_processed
    AFTER INSERT ON public.invoice_header
    FOR EACH ROW
    EXECUTE FUNCTION public.notify_invoice_processed();

COMMIT;
