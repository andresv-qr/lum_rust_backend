-- Migration to add EmailVerification purpose to password_verification_codes table
-- Date: 2025-09-26
-- Purpose: Allow EmailVerification as a valid purpose value

-- Drop the existing constraint
ALTER TABLE password_verification_codes 
DROP CONSTRAINT password_verification_codes_purpose_check;

-- Add the new constraint with EmailVerification included
ALTER TABLE password_verification_codes 
ADD CONSTRAINT password_verification_codes_purpose_check 
CHECK (purpose::text = ANY (ARRAY[
    'reset_password'::character varying::text, 
    'first_time_setup'::character varying::text, 
    'change_password'::character varying::text,
    'EmailVerification'::character varying::text
]));

-- Verify the constraint was added correctly
SELECT 
    conname AS constraint_name,
    consrc AS constraint_definition
FROM pg_constraint 
WHERE conname = 'password_verification_codes_purpose_check';

COMMENT ON TABLE password_verification_codes IS 'Updated to support EmailVerification purpose for unified verification system';