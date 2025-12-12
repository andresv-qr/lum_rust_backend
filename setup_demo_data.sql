-- ============================================================================
-- SETUP DEMO DATA FOR MERCHANT PORTAL
-- ============================================================================

-- 1. Create Test Merchant
-- Name: Demo Store
-- API Key: lumis123
-- Hash: $2b$12$tgtP1GQOqhmwh9FCAAxrNe4C8.yPbzayfnZ9hfneZ0lakPVxsnJUm
INSERT INTO rewards.merchants (
    merchant_id, 
    merchant_name, 
    api_key_hash, 
    is_active, 
    contact_email, 
    created_at
)
VALUES (
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11',
    'Demo Store',
    '$2b$12$tgtP1GQOqhmwh9FCAAxrNe4C8.yPbzayfnZ9hfneZ0lakPVxsnJUm',
    true,
    'demo@lumis.com',
    NOW()
)
ON CONFLICT (merchant_name) DO NOTHING;

-- 2. Create Test Offer
-- Name: Café Gratis
-- Cost: 10 Lumis
INSERT INTO rewards.redemption_offers (
    offer_id,
    merchant_id,
    name,
    name_friendly,
    description_friendly,
    lumis_cost,
    offer_category,
    valid_from,
    valid_to,
    is_active,
    stock_quantity,
    max_redemptions_per_user,
    img,
    created_at
)
VALUES (
    'b1ffcd00-0d1c-5ff9-cc7e-7cc0ce491b22',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11',
    'Café Gratis',
    'Café Americano Gratis',
    'Disfruta de un delicioso café americano de 12oz.',
    10,
    'Alimentos',
    NOW(),
    NOW() + INTERVAL '1 year',
    true,
    1000,
    5,
    'https://placehold.co/400x400/6B46C1/white?text=Cafe',
    NOW()
)
ON CONFLICT DO NOTHING;

-- 3. Create Test User Balance (if needed for testing redemption)
-- Assuming user_id 1 exists, give them Lumis
INSERT INTO rewards.fact_balance_points (user_id, balance)
VALUES (1, 5000)
ON CONFLICT (user_id) 
DO UPDATE SET balance = rewards.fact_balance_points.balance + 5000;
