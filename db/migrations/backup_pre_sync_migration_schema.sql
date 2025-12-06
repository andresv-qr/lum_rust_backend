--
-- PostgreSQL database dump
--

\restrict gbAnqr2p9QabFcVWhxMBrgpdmP7mIBwLKojyNNuyYe92WT07f2KxoVwho0SXHJa

-- Dumped from database version 18.0 (Ubuntu 18.0-1.pgdg24.04+3)
-- Dumped by pg_dump version 18.0 (Ubuntu 18.0-1.pgdg24.04+3)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: invoice_header; Type: TABLE; Schema: public; Owner: avalencia
--

CREATE TABLE public.invoice_header (
    cufe text,
    issuer_name text,
    no text,
    user_phone_number text,
    user_telegram_id text,
    "time" text,
    receptor_address text,
    tot_itbms double precision,
    issuer_dv text,
    receptor_phone text,
    auth_date text,
    date timestamp without time zone,
    receptor_id text,
    issuer_address text,
    issuer_ruc text,
    tot_amount double precision,
    receptor_name text,
    receptor_dv text,
    issuer_phone text,
    user_email text,
    origin character varying,
    user_ws character varying,
    type character varying,
    user_id bigint,
    url character varying,
    process_date timestamp with time zone,
    reception_date timestamp with time zone,
    sucursal text GENERATED ALWAYS AS (SUBSTRING(cufe FROM 29 FOR 4)) STORED,
    is_deleted boolean DEFAULT false,
    deleted_at timestamp without time zone,
    update_date timestamp without time zone DEFAULT now()
);


ALTER TABLE public.invoice_header OWNER TO avalencia;

--
-- Name: COLUMN invoice_header.is_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.invoice_header.is_deleted IS 'Soft delete flag - TRUE si la factura fue eliminada';


--
-- Name: COLUMN invoice_header.deleted_at; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.invoice_header.deleted_at IS 'Timestamp cuando la factura fue marcada como eliminada';


--
-- Name: COLUMN invoice_header.update_date; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.invoice_header.update_date IS 'Timestamp de última actualización del registro (para sync incremental)';


--
-- Name: dataset_versions; Type: TABLE; Schema: public; Owner: avalencia
--

CREATE TABLE public.dataset_versions (
    table_name character varying(100) NOT NULL,
    version bigint DEFAULT 0 NOT NULL,
    last_modified timestamp without time zone DEFAULT now() NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.dataset_versions OWNER TO avalencia;

--
-- Name: TABLE dataset_versions; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON TABLE public.dataset_versions IS 'Tracking de versiones para sincronización incremental';


--
-- Name: COLUMN dataset_versions.table_name; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dataset_versions.table_name IS 'Nombre de la tabla versionada';


--
-- Name: COLUMN dataset_versions.version; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dataset_versions.version IS 'Número de versión incremental (se incrementa en cada INSERT/UPDATE/DELETE)';


--
-- Name: COLUMN dataset_versions.last_modified; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dataset_versions.last_modified IS 'Timestamp de la última modificación al dataset';


--
-- Name: COLUMN dataset_versions.created_at; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dataset_versions.created_at IS 'Timestamp de creación del registro de versión';


--
-- Name: dim_issuer; Type: TABLE; Schema: public; Owner: avalencia
--

CREATE TABLE public.dim_issuer (
    issuer_ruc text NOT NULL,
    issuer_name text NOT NULL,
    issuer_best_name character varying,
    issuer_l1 character varying,
    issuer_l2 character varying,
    issuer_l3 character varying,
    issuer_l4 character varying,
    id integer NOT NULL,
    update_date timestamp without time zone,
    human_checked boolean,
    issuer_l1_model1 jsonb,
    issuer_l2_model1 jsonb,
    issuer_l3_model1 jsonb,
    issuer_l1_model2 jsonb,
    issuer_l2_model2 jsonb,
    issuer_l3_model2 jsonb,
    issuer_l1_model3 jsonb,
    issuer_l2_model3 jsonb,
    issuer_l3_model3 jsonb,
    lmatches integer,
    is_deleted boolean DEFAULT false,
    deleted_at timestamp without time zone
);


ALTER TABLE public.dim_issuer OWNER TO avalencia;

--
-- Name: COLUMN dim_issuer.is_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dim_issuer.is_deleted IS 'Soft delete flag - TRUE si el emisor fue eliminado';


--
-- Name: COLUMN dim_issuer.deleted_at; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dim_issuer.deleted_at IS 'Timestamp cuando el emisor fue marcado como eliminado';


--
-- Name: dim_issuer_id_seq; Type: SEQUENCE; Schema: public; Owner: avalencia
--

ALTER TABLE public.dim_issuer ALTER COLUMN id ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.dim_issuer_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: dim_product; Type: TABLE; Schema: public; Owner: avalencia
--

CREATE TABLE public.dim_product (
    code text,
    issuer_ruc text,
    code_cleaned text,
    process_date text,
    description text,
    l1_gemini jsonb,
    l2_gemini jsonb,
    l3_gemini jsonb,
    l4_gemini jsonb,
    l1_model2 jsonb,
    l2_model2 jsonb,
    l3_model2 jsonb,
    l4_model2 jsonb,
    l1_model3 jsonb,
    l2_model3 character varying,
    l3_model3 jsonb,
    l4_model3 jsonb,
    ws_cat1 character varying,
    ws_cat2 character varying,
    ws_cat3 character varying,
    brand character varying,
    name character varying,
    gtin character varying,
    seller character varying,
    product_type character varying,
    prop0_key character varying,
    prop0_value character varying,
    prop1_key character varying,
    prop1_value character varying,
    l1 character varying,
    l2 character varying,
    l3 character varying,
    l4 character varying,
    update_date timestamp without time zone,
    lmatches integer,
    issuer_name character varying,
    human_checked boolean,
    is_deleted boolean DEFAULT false,
    deleted_at timestamp without time zone
);


ALTER TABLE public.dim_product OWNER TO avalencia;

--
-- Name: COLUMN dim_product.is_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dim_product.is_deleted IS 'Soft delete flag - TRUE si el producto fue eliminado';


--
-- Name: COLUMN dim_product.deleted_at; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.dim_product.deleted_at IS 'Timestamp cuando el producto fue marcado como eliminado';


--
-- Name: invoice_detail; Type: TABLE; Schema: public; Owner: avalencia
--

CREATE TABLE public.invoice_detail (
    cufe text,
    quantity text,
    code text,
    date text,
    partkey text,
    total text,
    unit_price text,
    amount text,
    unit_discount text,
    description text,
    information_of_interest text,
    itbms text,
    is_deleted boolean DEFAULT false,
    deleted_at timestamp without time zone,
    update_date timestamp without time zone DEFAULT now()
);


ALTER TABLE public.invoice_detail OWNER TO avalencia;

--
-- Name: COLUMN invoice_detail.is_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.invoice_detail.is_deleted IS 'Soft delete flag - TRUE si el detalle fue eliminado';


--
-- Name: COLUMN invoice_detail.deleted_at; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.invoice_detail.deleted_at IS 'Timestamp cuando el detalle fue marcado como eliminado';


--
-- Name: COLUMN invoice_detail.update_date; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON COLUMN public.invoice_detail.update_date IS 'Timestamp de última actualización del registro (para sync incremental)';


--
-- Name: dataset_versions dataset_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: avalencia
--

ALTER TABLE ONLY public.dataset_versions
    ADD CONSTRAINT dataset_versions_pkey PRIMARY KEY (table_name);


--
-- Name: dim_issuer pk_ruc_name; Type: CONSTRAINT; Schema: public; Owner: avalencia
--

ALTER TABLE ONLY public.dim_issuer
    ADD CONSTRAINT pk_ruc_name PRIMARY KEY (issuer_ruc, issuer_name);


--
-- Name: idx_dim_issuer_deleted; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_issuer_deleted ON public.dim_issuer USING btree (deleted_at DESC) WHERE (is_deleted = true);


--
-- Name: INDEX idx_dim_issuer_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_dim_issuer_deleted IS 'Performance para queries de deleted items';


--
-- Name: idx_dim_issuer_ruc_name; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_issuer_ruc_name ON public.dim_issuer USING btree (issuer_ruc, issuer_name);


--
-- Name: INDEX idx_dim_issuer_ruc_name; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_dim_issuer_ruc_name IS 'Performance para JOINs con invoice_header';


--
-- Name: idx_dim_issuer_update_date_active; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_issuer_update_date_active ON public.dim_issuer USING btree (update_date DESC) WHERE (is_deleted = false);


--
-- Name: INDEX idx_dim_issuer_update_date_active; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_dim_issuer_update_date_active IS 'Performance para queries de sync incremental (emisores activos)';


--
-- Name: idx_dim_product_code_ruc; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_product_code_ruc ON public.dim_product USING btree (code, issuer_ruc) WHERE (is_deleted = false);


--
-- Name: INDEX idx_dim_product_code_ruc; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_dim_product_code_ruc IS 'Performance para JOINs con invoice_detail';


--
-- Name: idx_dim_product_composite; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_product_composite ON public.dim_product USING btree (code, description, issuer_ruc);


--
-- Name: idx_dim_product_deleted; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_product_deleted ON public.dim_product USING btree (deleted_at DESC) WHERE (is_deleted = true);


--
-- Name: INDEX idx_dim_product_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_dim_product_deleted IS 'Performance para queries de deleted items';


--
-- Name: idx_dim_product_description_trgm; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_product_description_trgm ON public.dim_product USING gin (description public.gin_trgm_ops);


--
-- Name: idx_dim_product_update_date_active; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_dim_product_update_date_active ON public.dim_product USING btree (update_date DESC) WHERE (is_deleted = false);


--
-- Name: INDEX idx_dim_product_update_date_active; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_dim_product_update_date_active IS 'Performance para queries de sync incremental (productos activos)';


--
-- Name: idx_invoice_detail_cufe; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_detail_cufe ON public.invoice_detail USING btree (cufe, code, description);


--
-- Name: INDEX idx_invoice_detail_cufe; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_invoice_detail_cufe IS 'Performance para queries por factura';


--
-- Name: idx_invoice_detail_deleted; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_detail_deleted ON public.invoice_detail USING btree (deleted_at DESC) WHERE (is_deleted = true);


--
-- Name: INDEX idx_invoice_detail_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_invoice_detail_deleted IS 'Performance para queries de deleted items';


--
-- Name: idx_invoice_detail_description; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_detail_description ON public.invoice_detail USING btree (lower(description));


--
-- Name: idx_invoice_detail_update_date_active; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_detail_update_date_active ON public.invoice_detail USING btree (update_date DESC) WHERE (is_deleted = false);


--
-- Name: idx_invoice_header_cufe; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE UNIQUE INDEX idx_invoice_header_cufe ON public.invoice_header USING btree (cufe);


--
-- Name: idx_invoice_header_deleted; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_header_deleted ON public.invoice_header USING btree (deleted_at DESC) WHERE (is_deleted = true);


--
-- Name: INDEX idx_invoice_header_deleted; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_invoice_header_deleted IS 'Performance para queries de deleted items';


--
-- Name: idx_invoice_header_update_date_active; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_header_update_date_active ON public.invoice_header USING btree (update_date DESC) WHERE (is_deleted = false);


--
-- Name: idx_invoice_header_user_cufe; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_header_user_cufe ON public.invoice_header USING btree (user_id, cufe, issuer_name);


--
-- Name: idx_invoice_header_user_id_cufe; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_header_user_id_cufe ON public.invoice_header USING btree (user_id, cufe) WHERE (is_deleted = false);


--
-- Name: INDEX idx_invoice_header_user_id_cufe; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON INDEX public.idx_invoice_header_user_id_cufe IS 'Performance para queries por usuario';


--
-- Name: idx_invoice_header_user_issuer; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX idx_invoice_header_user_issuer ON public.invoice_header USING btree (user_id, issuer_ruc, issuer_name);


--
-- Name: invoice_header_user_email_idx; Type: INDEX; Schema: public; Owner: avalencia
--

CREATE INDEX invoice_header_user_email_idx ON public.invoice_header USING btree (user_email);


--
-- Name: invoice_detail increment_detail_version; Type: TRIGGER; Schema: public; Owner: avalencia
--

CREATE TRIGGER increment_detail_version AFTER INSERT OR DELETE OR UPDATE ON public.invoice_detail FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();


--
-- Name: TRIGGER increment_detail_version ON invoice_detail; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON TRIGGER increment_detail_version ON public.invoice_detail IS 'Auto-incrementa version del dataset en cada cambio';


--
-- Name: invoice_header increment_header_version; Type: TRIGGER; Schema: public; Owner: avalencia
--

CREATE TRIGGER increment_header_version AFTER INSERT OR DELETE OR UPDATE ON public.invoice_header FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();


--
-- Name: TRIGGER increment_header_version ON invoice_header; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON TRIGGER increment_header_version ON public.invoice_header IS 'Auto-incrementa version del dataset en cada cambio';


--
-- Name: dim_issuer increment_issuer_version; Type: TRIGGER; Schema: public; Owner: avalencia
--

CREATE TRIGGER increment_issuer_version AFTER INSERT OR DELETE OR UPDATE ON public.dim_issuer FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();


--
-- Name: TRIGGER increment_issuer_version ON dim_issuer; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON TRIGGER increment_issuer_version ON public.dim_issuer IS 'Auto-incrementa version del dataset en cada cambio';


--
-- Name: dim_product increment_product_version; Type: TRIGGER; Schema: public; Owner: avalencia
--

CREATE TRIGGER increment_product_version AFTER INSERT OR DELETE OR UPDATE ON public.dim_product FOR EACH STATEMENT EXECUTE FUNCTION public.increment_dataset_version();


--
-- Name: TRIGGER increment_product_version ON dim_product; Type: COMMENT; Schema: public; Owner: avalencia
--

COMMENT ON TRIGGER increment_product_version ON public.dim_product IS 'Auto-incrementa version del dataset en cada cambio';


--
-- Name: invoice_header trg_refresh_lum_levels; Type: TRIGGER; Schema: public; Owner: avalencia
--

CREATE TRIGGER trg_refresh_lum_levels AFTER INSERT ON public.invoice_header FOR EACH ROW EXECUTE FUNCTION gamification.trigger_refresh_lum_levels();


--
-- Name: invoice_header trg_refresh_user_summary; Type: TRIGGER; Schema: public; Owner: avalencia
--

CREATE TRIGGER trg_refresh_user_summary AFTER INSERT OR DELETE OR UPDATE ON public.invoice_header FOR EACH ROW EXECUTE FUNCTION rewards.trigger_refresh_user_summary();


--
-- Name: TABLE invoice_header; Type: ACL; Schema: public; Owner: avalencia
--

GRANT SELECT ON TABLE public.invoice_header TO aval_fastapi;


--
-- Name: TABLE dataset_versions; Type: ACL; Schema: public; Owner: avalencia
--

GRANT SELECT,INSERT ON TABLE public.dataset_versions TO flutteruser;


--
-- Name: TABLE dim_issuer; Type: ACL; Schema: public; Owner: avalencia
--

GRANT SELECT,INSERT ON TABLE public.dim_issuer TO flutteruser;


--
-- Name: TABLE dim_product; Type: ACL; Schema: public; Owner: avalencia
--

GRANT SELECT,INSERT ON TABLE public.dim_product TO flutteruser;


--
-- Name: TABLE invoice_detail; Type: ACL; Schema: public; Owner: avalencia
--

GRANT SELECT ON TABLE public.invoice_detail TO aval_fastapi;


--
-- PostgreSQL database dump complete
--

\unrestrict gbAnqr2p9QabFcVWhxMBrgpdmP7mIBwLKojyNNuyYe92WT07f2KxoVwho0SXHJa

